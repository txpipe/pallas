//! Logic for validating and applying new blocks and txs to the chain state

pub mod alonzo;
pub mod babbage;
pub mod byron;
pub mod shelley_ma;
pub mod utils;

use alonzo::validate_alonzo_tx;
use babbage::validate_babbage_tx;
use byron::validate_byron_tx;
use pallas_traverse::{Era, MultiEraTx};
use shelley_ma::validate_shelley_ma_tx;

pub use utils::{
    Environment, MultiEraProtocolParameters, UTxOs,
    ValidationError::{TxAndProtParamsDiffer, UnknownProtParams},
    ValidationResult,
};

pub fn validate(metx: &MultiEraTx, utxos: &UTxOs, env: &Environment) -> ValidationResult {
    match env.prot_params() {
        MultiEraProtocolParameters::Byron(bpp) => match metx {
            MultiEraTx::Byron(mtxp) => validate_byron_tx(mtxp, utxos, bpp, env.prot_magic()),
            _ => Err(TxAndProtParamsDiffer),
        },
        MultiEraProtocolParameters::Shelley(spp) => match metx {
            MultiEraTx::AlonzoCompatible(mtx, Era::Shelley)
            | MultiEraTx::AlonzoCompatible(mtx, Era::Allegra)
            | MultiEraTx::AlonzoCompatible(mtx, Era::Mary) => validate_shelley_ma_tx(
                mtx,
                utxos,
                spp,
                env.block_slot(),
                env.network_id(),
                &metx.era(),
            ),
            _ => Err(TxAndProtParamsDiffer),
        },
        MultiEraProtocolParameters::Alonzo(app) => match metx {
            MultiEraTx::AlonzoCompatible(mtx, Era::Alonzo) => {
                validate_alonzo_tx(mtx, utxos, app, env.block_slot(), env.network_id())
            }
            _ => Err(TxAndProtParamsDiffer),
        },
        MultiEraProtocolParameters::Babbage(bpp) => match metx {
            MultiEraTx::Babbage(mtx) => validate_babbage_tx(
                mtx,
                utxos,
                bpp,
                env.block_slot(),
                env.prot_magic(),
                env.network_id(),
            ),
            _ => Err(TxAndProtParamsDiffer),
        },
    }
}
