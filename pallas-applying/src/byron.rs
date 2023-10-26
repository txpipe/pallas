//! Utilities required for Byron-era transaction validation.

use crate::types::{ByronProtParams, UTxOs, ValidationResult};

use pallas_primitives::byron::MintedTxPayload;

// TODO: implement each of the validation rules.
pub fn validate_byron_tx(
    _mtxp: &MintedTxPayload,
    _utxos: &UTxOs,
    _prot_pps: &ByronProtParams,
) -> ValidationResult {
    Ok(())
}
