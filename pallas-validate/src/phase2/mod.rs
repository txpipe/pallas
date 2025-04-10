use pallas_codec::minicbor;
use pallas_primitives::{
    conway::{Redeemer, Tx},
    Hash, TransactionInput,
};
use pallas_traverse::MultiEraTx;
use uplc::{
    machine::{cost_model::ExBudget, eval_result::EvalResult},
    tx::{error::Error, eval_phase_two, ResolvedInput, SlotConfig},
};

use crate::utils::{MultiEraProtocolParameters, UtxoMap};

pub mod uplc;

pub type EvalReport = Vec<(Redeemer, EvalResult)>;

pub fn evaluate_tx(
    tx: &MultiEraTx,
    pparams: &MultiEraProtocolParameters,
    utxos: &UtxoMap,
    slot_config: &SlotConfig,
) -> Result<EvalReport, Error> {
    let cbor = tx.encode();
    let tx: Tx = minicbor::decode(&cbor)?;

    let utxos = utxos
        .iter()
        .map(|(txoref, eracbor)| {
            let txhash = Hash::<32>::from(txoref.0.as_ref());
            Ok(ResolvedInput {
                input: TransactionInput {
                    transaction_id: txhash,
                    index: txoref.1.into(),
                },
                output: minicbor::decode(&eracbor.1)?,
            })
        })
        .collect::<Result<Vec<_>, pallas_codec::minicbor::decode::Error>>()?;

    let cost_model = match pparams {
        MultiEraProtocolParameters::Conway(params) => {
            Some(&params.cost_models_for_script_languages)
        }
        _ => None,
    };

    eval_phase_two(
        &tx,
        &utxos,
        cost_model,
        Some(&ExBudget::default()),
        slot_config,
        false,
        |_| (),
    )
}
