//! Logic for validating and applying new blocks and txs to the chain state

pub mod alonzo;
pub mod babbage;
pub mod byron;
pub mod conway;
pub mod shelley_ma;

use alonzo::validate_alonzo_tx;
use babbage::validate_babbage_tx;
use byron::validate_byron_tx;
use conway::validate_conway_tx;
use pallas_primitives::alonzo::TransactionIndex;
use pallas_traverse::{Era, MultiEraTx};
use shelley_ma::validate_shelley_ma_tx;

use crate::utils::{
    CertState, Environment, MultiEraProtocolParameters, UTxOs,
    ValidationError::{
        EnvMissingAccountState, PParamsByronDoesntNeedAccountState, TxAndProtParamsDiffer,
    },
    ValidationResult,
};

/// Ledger sequence rule: LEDGERS
pub fn validate_txs(
    metxs: &[MultiEraTx],
    env: &Environment,
    utxos: &UTxOs,
    cert_state: &mut CertState,
) -> ValidationResult {
    let mut delta_state: CertState = cert_state.clone();
    for (txix, metx) in metxs.iter().enumerate() {
        validate_tx(metx, txix.try_into().unwrap(), env, utxos, &mut delta_state)?;
    }
    *cert_state = delta_state;
    Ok(())
}

/// Ledger inference rule: LEDGER
pub fn validate_tx(
    metx: &MultiEraTx,
    txix: TransactionIndex,
    env: &Environment,
    utxos: &UTxOs,
    cert_state: &mut CertState,
) -> ValidationResult {
    let pp_acnt = (env.prot_params(), env.acnt());
    match pp_acnt {
        (MultiEraProtocolParameters::Byron(bpp), None) => match metx {
            MultiEraTx::Byron(mtxp) => validate_byron_tx(mtxp, utxos, bpp, env.prot_magic()),
            _ => Err(TxAndProtParamsDiffer),
        },
        (MultiEraProtocolParameters::Byron(_), Some(_)) => Err(PParamsByronDoesntNeedAccountState),
        (MultiEraProtocolParameters::Shelley(spp), Some(acnt)) => match metx {
            MultiEraTx::AlonzoCompatible(mtx, Era::Shelley)
            | MultiEraTx::AlonzoCompatible(mtx, Era::Allegra)
            | MultiEraTx::AlonzoCompatible(mtx, Era::Mary) => validate_shelley_ma_tx(
                mtx,
                txix,
                utxos,
                cert_state,
                spp,
                acnt,
                env.block_slot(),
                env.network_id(),
                &metx.era(),
            ),
            _ => Err(TxAndProtParamsDiffer),
        },
        (MultiEraProtocolParameters::Alonzo(app), _) => match metx {
            MultiEraTx::AlonzoCompatible(mtx, Era::Alonzo) => {
                validate_alonzo_tx(mtx, utxos, app, env.block_slot(), env.network_id())
            }
            _ => Err(TxAndProtParamsDiffer),
        },
        (MultiEraProtocolParameters::Babbage(bpp), _) => match metx {
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
        (MultiEraProtocolParameters::Conway(cpp), _) => match metx {
            MultiEraTx::Conway(mtx) => {
                validate_conway_tx(mtx, utxos, cpp, env.block_slot(), env.network_id())
            }
            _ => Err(TxAndProtParamsDiffer),
        },
        (_, None) => Err(EnvMissingAccountState),
    }
}
