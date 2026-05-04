use crate::utils::{MultiEraProtocolParameters, UtxoMap};

use super::{
    error::Error,
    evaluator,
    script_context::{
        find_script, DataLookupTable, ResolvedInput, ScriptVersion, SlotConfig, TxInfo, TxInfoV1,
        TxInfoV2, TxInfoV3,
    },
    to_plutus_data::ToPlutusData,
};
use pallas_primitives::{
    conway::{Language, Redeemer, RedeemerTag},
    ExUnits, PlutusData,
};
use pallas_traverse::{MultiEraRedeemer, MultiEraTx};

use tracing::{debug, instrument};

#[derive(Debug)]
pub struct TxEvalResult {
    pub tag: RedeemerTag,
    pub index: u32,
    pub units: ExUnits,
    pub success: bool,
    pub logs: Vec<String>,
    pub failure_message: Option<String>,
}

pub fn eval_tx(
    tx: &MultiEraTx,
    pparams: &MultiEraProtocolParameters,
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
    language: Language,
    tx_info: TxInfo,
    script_bytes: &[u8],
    datum: Option<PlutusData>,
    redeemer: &Redeemer,
    protocol_version_major: u32,
) -> Result<TxEvalResult, Error> {
    let script_context = tx_info
        .into_script_context(redeemer, datum.as_ref())
        .ok_or_else(|| Error::ScriptContextBuildError)?;

    let script_context_data = script_context.to_plutus_data();
    let redeemer_data = redeemer.to_plutus_data();

    let result = evaluator::eval_script(
        language,
        script_bytes,
        datum.as_ref(),
        &redeemer_data,
        &script_context_data,
        protocol_version_major,
    )?;

    let failure_message = result.failure.map(|f| {
        debug!(
            message = %f.message,
            logs = ?f.logs,
            "phase-two script execution failed"
        );
        f.message
    });

    Ok(TxEvalResult {
        tag: redeemer.tag,
        index: redeemer.index,
        success: result.success,
        units: result.units,
        logs: result.logs,
        failure_message,
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
            Language::PlutusV1,
            TxInfoV1::from_transaction(tx, utxos, slot_config)?,
            script.as_ref(),
            datum,
            &redeemer,
            protocol_version_major,
        )?),

        (ScriptVersion::V2(script), datum) => Ok(execute_script(
            Language::PlutusV2,
            TxInfoV2::from_transaction(tx, utxos, slot_config)?,
            script.as_ref(),
            datum,
            &redeemer,
            protocol_version_major,
        )?),

        (ScriptVersion::V3(script), datum) => Ok(execute_script(
            Language::PlutusV3,
            TxInfoV3::from_transaction(tx, utxos, slot_config)?,
            script.as_ref(),
            datum,
            &redeemer,
            protocol_version_major,
        )?),
    }
}
