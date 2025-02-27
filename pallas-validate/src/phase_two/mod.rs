use pallas_traverse::MultiEraTx;

use crate::{
    uplc::{error::Error, script_context::SlotConfig, tx::TxEvalResult},
    utils::{MultiEraProtocolParameters, UtxoMap},
};

pub fn evaluate_tx(
    tx: &MultiEraTx,
    pparams: &MultiEraProtocolParameters,
    utxos: &UtxoMap,
    slot_config: &SlotConfig,
) -> Result<Vec<TxEvalResult>, Error> {
    crate::uplc::tx::eval_tx(tx, pparams, utxos, slot_config)
}
