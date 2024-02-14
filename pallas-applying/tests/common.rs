use pallas_applying::UTxOs;
use pallas_codec::{minicbor::bytes::ByteVec, utils::TagWrap};
use pallas_primitives::{
    alonzo::{MintedTx, TransactionBody, TransactionOutput, Value},
    babbage::{
        MintedDatumOption, MintedPostAlonzoTransactionOutput, MintedScriptRef,
        MintedTransactionBody, MintedTransactionOutput, MintedTx as BabbageMintedTx,
        PseudoTransactionOutput,
    },
    byron::{Address, MintedTxPayload, Tx, TxOut},
};
use pallas_traverse::{MultiEraInput, MultiEraOutput};
use std::{borrow::Cow, iter::zip, vec::Vec};

use pallas_codec::utils::{Bytes, CborWrap};
use pallas_crypto::hash::Hash;

pub fn cbor_to_bytes(input: &str) -> Vec<u8> {
    hex::decode(input).unwrap()
}

pub fn minted_tx_from_cbor(tx_cbor: &[u8]) -> MintedTx<'_> {
    pallas_codec::minicbor::decode::<MintedTx>(tx_cbor).unwrap()
}

pub fn babbage_minted_tx_from_cbor(tx_cbor: &[u8]) -> BabbageMintedTx<'_> {
    pallas_codec::minicbor::decode::<BabbageMintedTx>(&tx_cbor[..]).unwrap()
}

pub fn minted_tx_payload_from_cbor<'a>(tx_cbor: &'a Vec<u8>) -> MintedTxPayload<'a> {
    pallas_codec::minicbor::decode::<MintedTxPayload>(&tx_cbor[..]).unwrap()
}

pub fn mk_utxo_for_byron_tx<'a>(tx: &Tx, tx_outs_info: &[(String, u64)]) -> UTxOs<'a> {
    let mut utxos: UTxOs = UTxOs::new();
    for (tx_in, (address_payload, amount)) in zip(tx.inputs.clone().to_vec(), tx_outs_info) {
        let input_tx_out_addr: Address = match hex::decode(address_payload) {
            Ok(addr_bytes) => Address {
                payload: TagWrap(ByteVec::from(addr_bytes)),
                crc: 3430631884,
            },
            _ => panic!("Unable to decode input address"),
        };
        let tx_out: TxOut = TxOut {
            address: input_tx_out_addr,
            amount: *amount,
        };
        let multi_era_in: MultiEraInput = MultiEraInput::Byron(Box::new(Cow::Owned(tx_in)));
        let multi_era_out: MultiEraOutput = MultiEraOutput::Byron(Box::new(Cow::Owned(tx_out)));
        utxos.insert(multi_era_in, multi_era_out);
    }
    utxos
}

pub fn mk_utxo_for_alonzo_compatible_tx<'a>(
    tx_body: &TransactionBody,
    tx_outs_info: &[(
        String, // address in string format
        Value,
        Option<Hash<32>>,
    )],
) -> UTxOs<'a> {
    let mut utxos: UTxOs = UTxOs::new();
    for (tx_in, (address, amount, datum_hash)) in zip(tx_body.inputs.clone(), tx_outs_info) {
        let multi_era_in: MultiEraInput =
            MultiEraInput::AlonzoCompatible(Box::new(Cow::Owned(tx_in)));
        let address_bytes: Bytes = match hex::decode(address) {
            Ok(bytes_vec) => Bytes::from(bytes_vec),
            _ => panic!("Unable to decode input address"),
        };
        let tx_out: TransactionOutput = TransactionOutput {
            address: address_bytes,
            amount: amount.clone(),
            datum_hash: *datum_hash,
        };
        let multi_era_out: MultiEraOutput =
            MultiEraOutput::AlonzoCompatible(Box::new(Cow::Owned(tx_out)));
        utxos.insert(multi_era_in, multi_era_out);
    }
    utxos
}

pub fn mk_utxo_for_babbage_tx<'a>(
    tx_body: &MintedTransactionBody,
    tx_outs_info: &'a [(
        String, // address in string format
        Value,
        Option<MintedDatumOption>,
        Option<CborWrap<MintedScriptRef>>,
    )],
) -> UTxOs<'a> {
    let mut utxos: UTxOs = UTxOs::new();
    for (tx_in, (addr, val, datum_opt, script_ref)) in zip(tx_body.inputs.clone(), tx_outs_info) {
        let multi_era_in: MultiEraInput =
            MultiEraInput::AlonzoCompatible(Box::new(Cow::Owned(tx_in)));
        let address_bytes: Bytes = match hex::decode(addr) {
            Ok(bytes_vec) => Bytes::from(bytes_vec),
            _ => panic!("Unable to decode input address"),
        };
        let tx_out: MintedTransactionOutput =
            PseudoTransactionOutput::PostAlonzo(MintedPostAlonzoTransactionOutput {
                address: address_bytes,
                value: val.clone(),
                datum_option: datum_opt.clone(),
                script_ref: script_ref.clone(),
            });
        let multi_era_out: MultiEraOutput = MultiEraOutput::Babbage(Box::new(Cow::Owned(tx_out)));
        utxos.insert(multi_era_in, multi_era_out);
    }
    utxos
}

pub fn add_collateral_alonzo<'a>(
    tx_body: &TransactionBody,
    utxos: &mut UTxOs<'_>,
    collateral_info: &[(
        String, // address in string format
        Value,
        Option<Hash<32>>,
    )],
) {
    match &tx_body.collateral {
        Some(collaterals) => {
            for (tx_in, (address, amount, datum_hash)) in zip(collaterals, collateral_info) {
                let address_bytes: Bytes = match hex::decode(address) {
                    Ok(bytes_vec) => Bytes::from(bytes_vec),
                    _ => panic!("Unable to decode input address"),
                };
                let tx_out: TransactionOutput = TransactionOutput {
                    address: address_bytes,
                    amount: amount.clone(),
                    datum_hash: *datum_hash,
                };
                let multi_era_in: MultiEraInput =
                    MultiEraInput::AlonzoCompatible(Box::new(Cow::Owned(tx_in.clone())));
                let multi_era_out: MultiEraOutput =
                    MultiEraOutput::AlonzoCompatible(Box::new(Cow::Owned(tx_out)));
                utxos.insert(multi_era_in, multi_era_out);
            }
        }
        None => panic!("Adding collateral to UTxO failed due to an empty list of collaterals"),
    }
}

pub fn add_collateral_babbage<'a>(
    tx_body: &MintedTransactionBody,
    utxos: &mut UTxOs<'a>,
    collateral_info: &'a [(
        String, // address in string format
        Value,
        Option<MintedDatumOption>,
        Option<CborWrap<MintedScriptRef>>,
    )],
) {
    match &tx_body.collateral {
        Some(collaterals) => {
            if collaterals.is_empty() {
                panic!("UTxO addition error - collateral input missing")
            } else {
                for (tx_in, (addr, val, datum_opt, script_ref)) in
                    zip(collaterals.clone(), collateral_info)
                {
                    let multi_era_in: MultiEraInput =
                        MultiEraInput::AlonzoCompatible(Box::new(Cow::Owned(tx_in)));
                    let address_bytes: Bytes = match hex::decode(addr) {
                        Ok(bytes_vec) => Bytes::from(bytes_vec),
                        _ => panic!("Unable to decode input address"),
                    };
                    let tx_out: MintedTransactionOutput =
                        PseudoTransactionOutput::PostAlonzo(MintedPostAlonzoTransactionOutput {
                            address: address_bytes,
                            value: val.clone(),
                            datum_option: datum_opt.clone(),
                            script_ref: script_ref.clone(),
                        });
                    let multi_era_out: MultiEraOutput =
                        MultiEraOutput::Babbage(Box::new(Cow::Owned(tx_out)));
                    utxos.insert(multi_era_in, multi_era_out);
                }
            }
        }
        None => panic!("UTxO addition error - collateral input missing"),
    }
}

pub fn add_ref_input_babbage<'a>(
    tx_body: &MintedTransactionBody,
    utxos: &mut UTxOs<'a>,
    ref_input_info: &'a [(
        String, // address in string format
        Value,
        Option<MintedDatumOption>,
        Option<CborWrap<MintedScriptRef>>,
    )],
) {
    match &tx_body.reference_inputs {
        Some(ref_inputs) => {
            if ref_inputs.is_empty() {
                panic!("UTxO addition error - reference input missing")
            } else {
                for (tx_in, (addr, val, datum_opt, script_ref)) in
                    zip(ref_inputs.clone(), ref_input_info)
                {
                    let multi_era_in: MultiEraInput =
                        MultiEraInput::AlonzoCompatible(Box::new(Cow::Owned(tx_in)));
                    let address_bytes: Bytes = match hex::decode(addr) {
                        Ok(bytes_vec) => Bytes::from(bytes_vec),
                        _ => panic!("Unable to decode input address"),
                    };
                    let tx_out: MintedTransactionOutput =
                        PseudoTransactionOutput::PostAlonzo(MintedPostAlonzoTransactionOutput {
                            address: address_bytes,
                            value: val.clone(),
                            datum_option: datum_opt.clone(),
                            script_ref: script_ref.clone(),
                        });
                    let multi_era_out: MultiEraOutput =
                        MultiEraOutput::Babbage(Box::new(Cow::Owned(tx_out)));
                    utxos.insert(multi_era_in, multi_era_out);
                }
            }
        }
        None => panic!("UTxO addition error - reference input missing"),
    }
}
