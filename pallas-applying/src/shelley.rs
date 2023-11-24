//! Utilities required for Shelley-era transaction validation.

use crate::types::{
    ShelleyMAError::*,
    ShelleyProtParams, UTxOs,
    ValidationError::{self, *},
    ValidationResult,
};
use pallas_addresses::{Address, ShelleyAddress};
use pallas_codec::minicbor::encode;
use pallas_primitives::{
    alonzo::{MintedTx, MintedWitnessSet, TransactionBody, TransactionOutput, Value},
    byron::TxOut,
};
use pallas_traverse::{MultiEraInput, MultiEraOutput};

// TODO: implement each of the validation rules.
pub fn validate_shelley_tx(
    mtx: &MintedTx,
    utxos: &UTxOs,
    prot_pps: &ShelleyProtParams,
    _prot_magic: &u32,
    block_slot: &u64,
    network_id: &u8,
) -> ValidationResult {
    let tx_body: &TransactionBody = &mtx.transaction_body;
    let tx_wits: &MintedWitnessSet = &mtx.transaction_witness_set;
    let size: &u64 = &get_tx_size(tx_body)?;
    check_ins_not_empty(tx_body)?;
    check_ins_in_utxos(tx_body, utxos)?;
    check_ttl(tx_body, block_slot)?;
    check_size(size, prot_pps)?;
    check_min_lovelace(tx_body, prot_pps)?;
    check_preservation_of_value(tx_body, utxos)?;
    check_fees(tx_body, prot_pps)?;
    check_network_id(tx_body, network_id)?;
    check_witnesses(tx_body, tx_wits)
}

fn get_tx_size(tx_body: &TransactionBody) -> Result<u64, ValidationError> {
    let mut buff: Vec<u8> = Vec::new();
    match encode(tx_body, &mut buff) {
        Ok(()) => Ok(buff.len() as u64),
        Err(_) => Err(Shelley(UnknownTxSize)),
    }
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
            return Err(Shelley(InputNotInUTxO));
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

fn check_size(size: &u64, prot_pps: &ShelleyProtParams) -> ValidationResult {
    if *size > prot_pps.max_tx_size {
        return Err(Shelley(MaxTxSizeExceeded));
    }
    Ok(())
}

fn check_min_lovelace(tx_body: &TransactionBody, prot_pps: &ShelleyProtParams) -> ValidationResult {
    for TransactionOutput { amount, .. } in &tx_body.outputs {
        match amount {
            Value::Coin(lovelace) => {
                if *lovelace < prot_pps.min_lovelace {
                    return Err(Shelley(MinLovelaceUnreached));
                }
            }
            _ => return Err(Shelley(ValueNotShelley)),
        }
    }
    Ok(())
}

fn check_preservation_of_value(tx_body: &TransactionBody, utxos: &UTxOs) -> ValidationResult {
    if get_consumed(tx_body, utxos)? != get_produced(tx_body)? + tx_body.fee {
        return Err(Shelley(PreservationOfValue));
    }
    Ok(())
}

fn get_consumed(tx_body: &TransactionBody, utxos: &UTxOs) -> Result<u64, ValidationError> {
    let mut res: u64 = 0;
    for input in tx_body.inputs.iter() {
        let utxo_value: &MultiEraOutput = utxos
            .get(&MultiEraInput::from_alonzo_compatible(input))
            .ok_or(Shelley(InputNotInUTxO))?;
        match MultiEraOutput::as_alonzo(utxo_value) {
            Some(TransactionOutput { amount, .. }) => match amount {
                Value::Coin(n) => res += n,
                _ => return Err(Shelley(WrongEraOutput)),
            },
            None => match MultiEraOutput::as_byron(utxo_value) {
                Some(TxOut { amount, .. }) => res += amount,
                _ => return Err(Shelley(InputNotInUTxO)),
            },
        }
    }
    Ok(res)
}

fn get_produced(tx_body: &TransactionBody) -> Result<u64, ValidationError> {
    let mut res: u64 = 0;
    for TransactionOutput { amount, .. } in tx_body.outputs.iter() {
        match amount {
            Value::Coin(n) => res += n,
            _ => return Err(Shelley(WrongEraOutput)),
        }
    }
    Ok(res)
}

fn check_fees(_tx_body: &TransactionBody, _prot_pps: &ShelleyProtParams) -> ValidationResult {
    Ok(())
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

fn check_witnesses(_tx_body: &TransactionBody, _tx_wits: &MintedWitnessSet) -> ValidationResult {
    Ok(())
}
