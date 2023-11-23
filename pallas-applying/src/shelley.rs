//! Utilities required for Shelley-era transaction validation.

use crate::types::{
    ShelleyMAError::*, ShelleyProtParams, UTxOs, ValidationError::*, ValidationResult,
};
use pallas_addresses::{Address, ShelleyAddress};
use pallas_primitives::alonzo::{MintedTx, TransactionBody};
use pallas_traverse::MultiEraInput;

// TODO: implement each of the validation rules.
pub fn validate_shelley_tx(
    mtx: &MintedTx,
    utxos: &UTxOs,
    _prot_pps: &ShelleyProtParams,
    _prot_magic: &u32,
    block_slot: &u64,
    network_id: &u8,
) -> ValidationResult {
    let tx_body: &TransactionBody = &mtx.transaction_body;
    check_ins_not_empty(tx_body)?;
    check_ins_in_utxos(tx_body, utxos)?;
    check_ttl(tx_body, block_slot)?;
    check_network_id(tx_body, network_id)
}

fn check_ins_not_empty(tx_body: &TransactionBody) -> ValidationResult {
    if tx_body.inputs.is_empty() {
        return Err(Shelley(TxInsEmpty));
    }
    Ok(())
}

fn check_ins_in_utxos(tx_body: &TransactionBody, utxos: &UTxOs) -> ValidationResult {
    for input in tx_body.inputs.iter() {
        if !(utxos.contains_key(&MultiEraInput::from_alonzo_compatible(input))) {
            return Err(Shelley(InputMissingInUTxO));
        }
    }
    Ok(())
}

fn check_ttl(tx_body: &TransactionBody, block_slot: &u64) -> ValidationResult {
    match tx_body.ttl {
        Some(ttl) => {
            if ttl < *block_slot {
                Err(Shelley(TTLExceeded))
            } else {
                Ok(())
            }
        }
        None => Err(Shelley(AlonzoCompNotShelley)),
    }
}

fn check_network_id(tx_body: &TransactionBody, network_id: &u8) -> ValidationResult {
    for output in tx_body.outputs.iter() {
        let addr: ShelleyAddress =
            match Address::from_bytes(&Vec::<u8>::from(output.address.clone())) {
                Ok(Address::Shelley(sa)) => sa,
                Ok(_) => return Err(Shelley(WrongEraOutput)),
                Err(_) => return Err(Shelley(AddressDecoding)),
            };
        if addr.network().value() != *network_id {
            return Err(Shelley(WrongNetworkID));
        }
    }
    Ok(())
}
