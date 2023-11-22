//! Logic for validating and applying new blocks and txs to the chain state

pub mod byron;
pub mod shelley;
pub mod types;

use byron::validate_byron_tx;
use pallas_traverse::{
    Era, MultiEraTx, MultiEraTx::AlonzoCompatible, MultiEraTx::Byron as ByronTxPayload,
};
use shelley::validate_shelley_tx;

pub use types::{Environment, MultiEraProtParams, UTxOs, ValidationResult};

pub fn validate(metx: &MultiEraTx, utxos: &UTxOs, env: &Environment) -> ValidationResult {
    match (metx, env) {
        (
            ByronTxPayload(mtxp),
            Environment {
                prot_params: MultiEraProtParams::Byron(bpp),
                prot_magic,
                ..
            },
        ) => validate_byron_tx(mtxp, utxos, bpp, prot_magic),
        (
            AlonzoCompatible(shelley_minted_tx, Era::Shelley),
            Environment {
                prot_params: MultiEraProtParams::Shelley(spp),
                prot_magic,
                block_slot,
                network_id,
            },
        ) => validate_shelley_tx(
            shelley_minted_tx,
            utxos,
            spp,
            prot_magic,
            block_slot,
            network_id,
        ),
        // TODO: implement the rest of the eras.
        _ => Ok(()),
    }
}
