use crate::utils::{MultiEraProtocolParameters, UtxoMap};

use super::{
    script_context::{ScriptContext, TxInfo, TxInfoV1},
    to_plutus_data::ToPlutusData,
};

use super::{
    error::Error,
    script_context::{
        find_script, DataLookupTable, ResolvedInput, ScriptVersion, SlotConfig, TxInfoV2, TxInfoV3,
    },
    to_plutus_data::convert_tag_to_constr,
};
use pallas_primitives::{
    conway::{Redeemer, RedeemerTag},
    ExUnits, PlutusData,
};
use pallas_traverse::{MultiEraRedeemer, MultiEraTx};

use rug::{ops::NegAssign, Complete, Integer};
use tracing::{debug, instrument};
use uplc_turbo::{
    binder::DeBruijn, bumpalo::Bump, constant::Constant, data::PlutusData as PragmaPlutusData,
    term::Term,
};

#[derive(Debug)]
pub struct TxEvalResult {
    pub tag: RedeemerTag,
    pub index: u32,
    pub units: ExUnits,
    pub success: bool,
    pub logs: Vec<String>,
}

pub fn map_pallas_data_to_pragma_data<'a>(
    arena: &'a Bump,
    data: &'a PlutusData,
) -> &'a PragmaPlutusData<'a> {
    match data {
        PlutusData::Constr(constr) => {
            let fields = constr
                .fields
                .iter()
                .map(|f| map_pallas_data_to_pragma_data(arena, f));

            let fields = arena.alloc_slice_fill_iter(fields);

            PragmaPlutusData::constr(arena, convert_tag_to_constr(constr.tag).unwrap(), fields)
        }
        PlutusData::Map(key_value_pairs) => {
            let key_value_pairs = key_value_pairs.iter().map(|(k, v)| {
                (
                    map_pallas_data_to_pragma_data(arena, k),
                    map_pallas_data_to_pragma_data(arena, v),
                )
            });

            let key_value_pairs = arena.alloc_slice_fill_iter(key_value_pairs);

            PragmaPlutusData::map(arena, key_value_pairs)
        }
        PlutusData::BigInt(big_int) => match big_int {
            pallas_primitives::BigInt::Int(int) => {
                let val = i128::from(*int);
                PragmaPlutusData::integer_from(arena, val)
            }
            pallas_primitives::BigInt::BigNInt(big_num_bytes) => {
                let mut val = Integer::parse(big_num_bytes.as_slice()).unwrap().complete();
                val.neg_assign();

                let val = arena.alloc(val);
                PragmaPlutusData::integer(arena, val)
            }
            // @TODO: recheck this implementations correctness
            pallas_primitives::BigInt::BigUInt(big_num_bytes) => {
                let val = Integer::parse(big_num_bytes.as_slice()).unwrap().complete();
                let val = arena.alloc(val);
                PragmaPlutusData::integer(arena, val)
            }
        },
        PlutusData::BoundedBytes(bounded_bytes) => {
            let bounded_bytes = arena.alloc(bounded_bytes.as_slice());
            PragmaPlutusData::byte_string(arena, bounded_bytes)
        }
        PlutusData::Array(maybe_indef_array) => {
            let items = maybe_indef_array
                .iter()
                .map(|x| map_pallas_data_to_pragma_data(arena, x));

            let items = arena.alloc_slice_fill_iter(items);

            PragmaPlutusData::list(arena, items)
        }
    }
}

pub fn plutus_data_to_pragma_term<'a>(
    arena: &'a Bump,
    data: &'a PlutusData,
) -> &'a Term<'a, DeBruijn> {
    Term::data(arena, map_pallas_data_to_pragma_data(arena, data))
}

pub fn eval_tx(
    tx: &MultiEraTx,
    _pparams: &MultiEraProtocolParameters, // For Cost Models
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

    let redeemers = tx.redeemers();

    let redeemers = redeemers
        .iter()
        .map(|r| eval_redeemer(r, tx, &utxos, &lookup_table, slot_config))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(redeemers)
}

fn execute_script(
    tx_info: TxInfo,
    script_bytes: &[u8],
    datum: Option<PlutusData>,
    redeemer: &Redeemer,
) -> Result<TxEvalResult, Error> {
    let script_context = tx_info
        .into_script_context(redeemer, datum.as_ref())
        .ok_or_else(|| Error::ScriptContextBuildError)?;

    let arena = Bump::with_capacity(1_024_000);

    let script_context_data = script_context.to_plutus_data();
    let script_context_term = plutus_data_to_pragma_term(&arena, &script_context_data);

    let redeemer_data = redeemer.to_plutus_data();
    let redeemer_term = plutus_data_to_pragma_term(&arena, &redeemer_data);

    let datum_term = datum
        .as_ref()
        .map(|d| plutus_data_to_pragma_term(&arena, d));

    let flat: pallas_codec::minicbor::bytes::ByteVec =
        pallas_codec::minicbor::decode(&script_bytes)
            .map_err(pallas_codec::minicbor::decode::Error::from)?;

    let program = uplc_turbo::flat::decode(&arena, &flat)?;

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

    let result = program.eval(&arena);

    let success = match script_context {
        // a non-error result is enough success criteria for v1v2
        ScriptContext::V1V2 { .. } => result.term.is_ok(),
        // v3 requires the result to be ok and the term to be a unit
        ScriptContext::V3 { .. } => match result.term {
            Ok(term) => match term {
                Term::Constant(constant) => match constant {
                    Constant::Unit => true,
                    _ => false,
                },
                _ => false,
            },
            Err(_) => false,
        },
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
        )?),

        (ScriptVersion::V2(script), datum) => Ok(execute_script(
            TxInfoV2::from_transaction(tx, utxos, slot_config)?,
            script.as_ref(),
            datum,
            &redeemer,
        )?),

        (ScriptVersion::V3(script), datum) => Ok(execute_script(
            TxInfoV3::from_transaction(tx, utxos, slot_config)?,
            script.as_ref(),
            datum,
            &redeemer,
        )?),
    }
}
