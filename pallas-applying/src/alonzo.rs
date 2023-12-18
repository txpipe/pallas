//! Utilities required for Shelley-era transaction validation.

use crate::types::{AlonzoProtParams, UTxOs, ValidationResult};
use pallas_primitives::alonzo::MintedTx;

pub fn validate_alonzo_tx(
    _mtxp: &MintedTx,
    _utxos: &UTxOs,
    _prot_pps: &AlonzoProtParams,
) -> ValidationResult {
    Ok(())
}
