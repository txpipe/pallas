use crate::utils::{MultiEraProtocolParameters, UtxoMap};

use super::{
    error::Error,
    script_context::{
        find_script, DataLookupTable, ResolvedInput, ScriptContext, ScriptVersion, SlotConfig,
        TxInfo, TxInfoV1, TxInfoV2, TxInfoV3,
    },
    to_plutus_data::ToPlutusData,
};
use pallas_primitives::{
    conway::{Redeemer, RedeemerTag},
    ExUnits, PlutusData,
};
use pallas_traverse::{MultiEraRedeemer, MultiEraTx};

use amaru_uplc::{
    arena::Arena, binder::DeBruijn, bumpalo::Bump, constant::Constant,
    data::PlutusData as PragmaPlutusData, machine::PlutusVersion, term::Term,
};
use tracing::{debug, instrument};

#[derive(Debug)]
pub struct TxEvalResult {
    pub tag: RedeemerTag,
    pub index: u32,
    pub units: ExUnits,
    pub success: bool,
    pub logs: Vec<String>,
}

pub fn plutus_data_to_pragma_term<'a>(
    arena: &'a Arena,
    data: &PlutusData,
) -> &'a Term<'a, DeBruijn> {
    // Bridge pallas PlutusData -> amaru-uplc PlutusData through CBOR. Both
    // sides implement the same Plutus data encoding, and the upstream amaru
    // node uses this exact pattern. Vec writer is Infallible and the bytes
    // are fresh, so neither side can fail in practice.
    let bytes = pallas_codec::minicbor::to_vec(data).expect("PlutusData encode");
    let pragma_data = PragmaPlutusData::from_cbor(arena, &bytes).expect("PlutusData decode");
    Term::data(arena, pragma_data)
}

pub fn eval_tx(
    tx: &MultiEraTx,
    pparams: &MultiEraProtocolParameters, // For Cost Models
    utxos: &UtxoMap,
    slot_config: &SlotConfig,
) -> Result<Vec<TxEvalResult>, Error> {
    let utxos = utxos
        .iter()
        .map(|(txoref, eracbor)| {
            Ok(ResolvedInput {
                input: pallas_primitives::TransactionInput {
                    transaction_id: txoref.0,
                    index: txoref.1.into(),
                },
                output: pallas_codec::minicbor::decode(&eracbor.1)?,
            })
        })
        .collect::<Result<Vec<_>, pallas_codec::minicbor::decode::Error>>()?;

    let lookup_table = DataLookupTable::from_transaction(tx, &utxos);

    let protocol_version_major = pparams.protocol_version() as u32;

    let redeemers = tx.redeemers();

    let redeemers = redeemers
        .iter()
        .map(|r| {
            eval_redeemer(
                r,
                tx,
                &utxos,
                &lookup_table,
                slot_config,
                protocol_version_major,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(redeemers)
}

fn execute_script(
    tx_info: TxInfo,
    script_bytes: &[u8],
    datum: Option<PlutusData>,
    redeemer: &Redeemer,
    plutus_version: PlutusVersion,
    protocol_version_major: u32,
) -> Result<TxEvalResult, Error> {
    let script_context = tx_info
        .into_script_context(redeemer, datum.as_ref())
        .ok_or_else(|| Error::ScriptContextBuildError)?;

    let arena = Arena::from_bump(Bump::with_capacity(1_024_000));

    let script_context_data = script_context.to_plutus_data();
    let script_context_term = plutus_data_to_pragma_term(&arena, &script_context_data);

    let redeemer_data = redeemer.to_plutus_data();
    let redeemer_term = plutus_data_to_pragma_term(&arena, &redeemer_data);

    let datum_term = datum
        .as_ref()
        .map(|d| plutus_data_to_pragma_term(&arena, d));

    let flat: pallas_codec::minicbor::bytes::ByteVec =
        pallas_codec::minicbor::decode(script_bytes)?;

    let program = amaru_uplc::flat::decode(&arena, &flat, plutus_version, protocol_version_major)?;

    let program = match script_context {
        ScriptContext::V1V2 { .. } => if let Some(datum_term) = datum_term {
            program.apply(&arena, datum_term)
        } else {
            program
        }
        .apply(&arena, redeemer_term)
        .apply(&arena, script_context_term),

        ScriptContext::V3 { .. } => program.apply(&arena, script_context_term),
    };

    let result = program.eval_version(&arena, plutus_version);

    let success = match script_context {
        // a non-error result is enough success criteria for v1v2
        ScriptContext::V1V2 { .. } => result.term.is_ok(),
        // v3 requires the result to be ok and the term to be a unit
        ScriptContext::V3 { .. } => matches!(
            result.term,
            Ok(Term::Constant(constant)) if matches!(*constant, Constant::Unit)
        ),
    };

    Ok(TxEvalResult {
        tag: redeemer.tag,
        index: redeemer.index,
        success,
        units: ExUnits {
            // @TODO hack until we have cost models
            steps: (result.info.consumed_budget.cpu * 11 / 10) as u64,
            mem: (result.info.consumed_budget.mem * 11 / 10) as u64,
        },
        logs: result.info.logs,
    })
}

#[instrument(skip_all, fields(tag = ?redeemer.tag(), index = redeemer.index()))]
pub fn eval_redeemer(
    redeemer: &MultiEraRedeemer,
    tx: &MultiEraTx,
    utxos: &[ResolvedInput],
    lookup_table: &DataLookupTable,
    slot_config: &SlotConfig,
    protocol_version_major: u32,
) -> Result<TxEvalResult, Error> {
    // TODO: trickle down the use of MultiEraX structs instead of dealing with
    // primitives directly. For now, we'll just downcast to Conway era.

    let tx = tx.as_conway().ok_or(Error::WrongEra())?;
    let redeemer = redeemer.into_conway_deprecated().ok_or(Error::WrongEra())?;

    debug!("evaluating redeemer");

    match find_script(&redeemer, tx, utxos, lookup_table)? {
        (ScriptVersion::Native(_), _) => Err(Error::NativeScriptPhaseTwo),

        (ScriptVersion::V1(script), datum) => Ok(execute_script(
            TxInfoV1::from_transaction(tx, utxos, slot_config)?,
            script.as_ref(),
            datum,
            &redeemer,
            PlutusVersion::V1,
            protocol_version_major,
        )?),

        (ScriptVersion::V2(script), datum) => Ok(execute_script(
            TxInfoV2::from_transaction(tx, utxos, slot_config)?,
            script.as_ref(),
            datum,
            &redeemer,
            PlutusVersion::V2,
            protocol_version_major,
        )?),

        (ScriptVersion::V3(script), datum) => Ok(execute_script(
            TxInfoV3::from_transaction(tx, utxos, slot_config)?,
            script.as_ref(),
            datum,
            &redeemer,
            PlutusVersion::V3,
            protocol_version_major,
        )?),
    }
}
