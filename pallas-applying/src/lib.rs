//! Logic for validating and applying new blocks and txs to the chain state

pub mod byron;
pub mod types;

use byron::validate_byron_tx;

use pallas_traverse::{MultiEraTx, MultiEraTx::Byron as ByronTxPayload};

pub use types::{Environment, MultiEraProtParams, UTxOs, ValidationResult};

pub fn validate(metx: &MultiEraTx, utxos: &UTxOs, env: &Environment) -> ValidationResult {
    match (metx, env) {
        (
            ByronTxPayload(mtxp),
            Environment {
                prot_params: MultiEraProtParams::Byron(bpp),
                prot_magic,
            },
        ) => validate_byron_tx(mtxp, utxos, bpp, prot_magic),
        // TODO: implement the rest of the eras.
        _ => Ok(()),
    }
}
