//! Logic for validating and applying new blocks and txs to the chain state

pub mod byron;
pub mod shelley_ma;
pub mod types;

use byron::validate_byron_tx;
use pallas_traverse::{Era, MultiEraTx};
use shelley_ma::validate_shelley_ma_tx;

pub use types::{
    Environment, MultiEraProtParams, UTxOs, ValidationError::TxAndProtParamsDiffer,
    ValidationResult,
};

pub fn validate(metx: &MultiEraTx, utxos: &UTxOs, env: &Environment) -> ValidationResult {
    match env {
        Environment {
            prot_params: MultiEraProtParams::Byron(bpp),
            prot_magic,
            ..
        } => match metx {
            MultiEraTx::Byron(mtxp) => validate_byron_tx(mtxp, utxos, bpp, prot_magic),
            _ => Err(TxAndProtParamsDiffer),
        },
        Environment {
            prot_params: MultiEraProtParams::Shelley(spp),
            block_slot,
            network_id,
            ..
        } => match metx.era() {
            Era::Shelley | Era::Allegra | Era::Mary => match metx.as_alonzo() {
                Some(mtx) => {
                    validate_shelley_ma_tx(mtx, utxos, spp, block_slot, network_id, &metx.era())
                }
                None => Err(TxAndProtParamsDiffer),
            },
            _ => Err(TxAndProtParamsDiffer),
        },
    }
}
