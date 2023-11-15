//! Utilities required for Shelley-era transaction validation.

use crate::types::{ShelleyProtParams, UTxOs, ValidationResult};

use pallas_primitives::alonzo::MintedTx;

// TODO: implement each of the validation rules.
pub fn validate_shelley_tx(
    _mtxp: &MintedTx,
    _utxos: &UTxOs,
    _prot_pps: &ShelleyProtParams,
    _prot_magic: &u32,
) -> ValidationResult {
    Ok(())
}
