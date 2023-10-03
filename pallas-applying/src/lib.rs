//! Logic for validating and applying new blocks and txs to the chain state

pub mod byron;
pub mod types;

use byron::validate_byron_tx;

use pallas_traverse::{MultiEraTx, MultiEraTx::Byron as ByronTxPayload};

pub use types::{
    MultiEraProtParams, MultiEraProtParams::Byron as ByronProtParams, UTxOs, ValidationResult,
};

pub fn validate(
    metx: &MultiEraTx,
    utxos: &UTxOs,
    prot_pps: &MultiEraProtParams,
) -> ValidationResult {
    match (metx, prot_pps) {
        (ByronTxPayload(mtxp), ByronProtParams(bpp)) => validate_byron_tx(mtxp, utxos, bpp),
        // TODO: implement the rest of the eras.
        _ => Ok(()),
    }
}
