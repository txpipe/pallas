use pallas_codec::{minicbor::{self, bytes::ByteVec}, utils::TagWrap};
use pallas_primitives::{
    alonzo::{Tx as AlonzoTx, TransactionBody, TransactionOutput, Value},
    babbage::Tx as BabbageTx,
    byron::{Address, TxPayload, Tx, TxOut},
    conway::Tx as ConwayTx,
};
use pallas_traverse::{Era, MultiEraInput, MultiEraOutput};
use pallas_validate::utils::UTxOs;
use pallas_validate::utils::{EraCbor, TxoRef, UtxoMap};
use std::{borrow::Cow, iter::zip, vec::Vec};

use pallas_codec::utils::{Bytes, CborWrap};
use pallas_crypto::hash::Hash;

pub fn cbor_to_bytes(input: &str) -> Vec<u8> {
    hex::decode(input).unwrap()
}

pub fn minted_tx_from_cbor(tx_cbor: &[u8]) -> AlonzoTx<'_> {
    pallas_codec::minicbor::decode::<AlonzoTx>(tx_cbor).unwrap()
}

pub fn babbage_minted_tx_from_cbor(tx_cbor: &[u8]) -> BabbageTx<'_> {
    pallas_codec::minicbor::decode::<BabbageTx>(tx_cbor).unwrap()
}

pub fn conway_minted_tx_from_cbor(tx_cbor: &[u8]) -> ConwayTx<'_> {
    pallas_codec::minicbor::decode::<ConwayTx>(tx_cbor).unwrap()
}

pub fn minted_tx_payload_from_cbor(tx_cbor: &[u8]) -> TxPayload<'_> {
    pallas_codec::minicbor::decode::<TxPayload>(tx_cbor).unwrap()
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
            MultiEraOutput::AlonzoCompatible(Box::new(Cow::Owned(tx_out)), Era::Alonzo);
        utxos.insert(multi_era_in, multi_era_out);
    }
    utxos
}

pub fn mk_utxo_for_babbage_tx<'a>(
    tx_body: &pallas_primitives::babbage::TransactionBody,
    tx_outs_info: &'a [(
        String, // address in string format
        Value,
        Option<pallas_primitives::babbage::DatumOption>,
        Option<CborWrap<pallas_primitives::babbage::ScriptRef>>,
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
        let tx_out: pallas_primitives::babbage::TransactionOutput =
            pallas_primitives::babbage::TransactionOutput::PostAlonzo(
                pallas_primitives::babbage::PostAlonzoTransactionOutput {
                    address: address_bytes,
                    value: val.clone(),
                    datum_option: datum_opt.clone().map(|x| x.into()),
                    script_ref: script_ref.clone(),
                }.into(),
            );
        let multi_era_out: MultiEraOutput = MultiEraOutput::Babbage(Box::new(Cow::Owned(tx_out)));
        utxos.insert(multi_era_in, multi_era_out);
    }
    utxos
}

pub fn mk_utxo_for_conway_tx<'a>(
    tx_body: &pallas_primitives::conway::TransactionBody,
    tx_outs_info: &'a [(
        String, // address in string format
        pallas_primitives::conway::Value,
        Option<pallas_primitives::conway::DatumOption>,
        Option<CborWrap<pallas_primitives::conway::ScriptRef>>,
    )],
) -> UTxOs<'a> {
    let mut utxos: UTxOs = UTxOs::new();

    for (tx_in, (addr, val, datum_opt, script_ref)) in
        zip(tx_body.inputs.clone().to_vec(), tx_outs_info)
    {
        let multi_era_in: MultiEraInput =
            MultiEraInput::AlonzoCompatible(Box::new(Cow::Owned(tx_in)));
        let address_bytes: Bytes = match hex::decode(addr) {
            Ok(bytes_vec) => Bytes::from(bytes_vec),
            _ => panic!("Unable to decode input address"),
        };
        let tx_out: pallas_primitives::conway::TransactionOutput =
            pallas_primitives::conway::TransactionOutput::PostAlonzo(
                pallas_primitives::conway::PostAlonzoTransactionOutput {
                    address: address_bytes,
                    value: val.clone(),
                    datum_option: datum_opt.clone().map(|x| x.into()),
                    script_ref: script_ref.clone(),
                }.into(),
            );
        let multi_era_out: MultiEraOutput = MultiEraOutput::Conway(Box::new(Cow::Owned(tx_out)));
        utxos.insert(multi_era_in, multi_era_out);
    }
    utxos
}

pub fn mk_codec_safe_utxo_for_conway_tx<'a>(
    tx_body: &pallas_primitives::conway::TransactionBody,
    tx_outs_info: &'a mut Vec<(
        String, // address in string format
        pallas_primitives::conway::Value,
        Option<pallas_codec::utils::KeepRaw<'a, pallas_primitives::conway::DatumOption>>,
        Option<CborWrap<pallas_primitives::conway::ScriptRef>>,
        Vec<u8>, // Placeholder for CBOR data.
    )>,
) -> UTxOs<'a> {
    let mut utxos: UTxOs = UTxOs::new();

    for (tx_in, (addr, val, datum_opt, script_ref, cbor)) in
        zip(tx_body.inputs.clone().to_vec(), tx_outs_info)
    {
        let multi_era_in: MultiEraInput =
            MultiEraInput::AlonzoCompatible(Box::new(Cow::Owned(tx_in)));
        let address_bytes: Bytes = match hex::decode(addr) {
            Ok(bytes_vec) => Bytes::from(bytes_vec),
            _ => panic!("Unable to decode input address"),
        };
        let post_alonzo =
            pallas_primitives::conway::PostAlonzoTransactionOutput {
                address: address_bytes,
                value: val.clone(),
                datum_option: datum_opt.clone(),
                script_ref: script_ref.clone(),
            };
        *cbor = minicbor::to_vec(post_alonzo).unwrap();
        let post_alonzo = minicbor::decode::<
                pallas_codec::utils::KeepRaw<'a, pallas_primitives::conway::PostAlonzoTransactionOutput
                                             >>(
            cbor
        ).unwrap();
        let tx_out = pallas_primitives::conway::TransactionOutput::PostAlonzo(post_alonzo);
        let multi_era_out: MultiEraOutput = MultiEraOutput::Conway(Box::new(Cow::Owned(tx_out)));
        utxos.insert(multi_era_in, multi_era_out);
    }
    utxos
}

pub fn mk_utxo_for_eval<'a>(utxos: UTxOs) -> UtxoMap {
    let mut eval_utxos: UtxoMap = UtxoMap::new();

    for (tx_in, tx_out) in utxos {
        eval_utxos.insert(TxoRef::from(&tx_in), EraCbor::from(tx_out));
    }
    eval_utxos
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
                    MultiEraOutput::AlonzoCompatible(Box::new(Cow::Owned(tx_out)), Era::Alonzo);
                utxos.insert(multi_era_in, multi_era_out);
            }
        }
        None => panic!("Adding collateral to UTxO failed due to an empty list of collaterals"),
    }
}

pub fn add_collateral_babbage<'a>(
    tx_body: &pallas_primitives::babbage::TransactionBody,
    utxos: &mut UTxOs<'a>,
    collateral_info: &'a [(
        String, // address in string format
        Value,
        Option<pallas_primitives::babbage::DatumOption>,
        Option<CborWrap<pallas_primitives::babbage::ScriptRef>>,
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
                    let tx_out: pallas_primitives::babbage::TransactionOutput =
                        pallas_primitives::babbage::TransactionOutput::PostAlonzo(
                            pallas_primitives::babbage::PostAlonzoTransactionOutput {
                                address: address_bytes,
                                value: val.clone(),
                                datum_option: datum_opt.clone().map(|x| x.into()),
                                script_ref: script_ref.clone(),
                            }.into(),
                        );
                    let multi_era_out: MultiEraOutput =
                        MultiEraOutput::Babbage(Box::new(Cow::Owned(tx_out)));
                    utxos.insert(multi_era_in, multi_era_out);
                }
            }
        }
        None => panic!("UTxO addition error - collateral input missing"),
    }
}

pub fn add_collateral_conway<'a>(
    tx_body: &pallas_primitives::conway::TransactionBody,
    utxos: &mut UTxOs<'a>,
    collateral_info: &'a [(
        String, // address in string format
        pallas_primitives::conway::Value,
        Option<pallas_primitives::conway::DatumOption>,
        Option<CborWrap<pallas_primitives::conway::ScriptRef>>,
    )],
) {
    match &tx_body.collateral {
        Some(collaterals) => {
            if collaterals.is_empty() {
                panic!("UTxO addition error - collateral input missing")
            } else {
                for (tx_in, (addr, val, datum_opt, script_ref)) in
                    zip(collaterals.clone().to_vec(), collateral_info)
                {
                    let multi_era_in: MultiEraInput =
                        MultiEraInput::AlonzoCompatible(Box::new(Cow::Owned(tx_in)));
                    let address_bytes: Bytes = match hex::decode(addr) {
                        Ok(bytes_vec) => Bytes::from(bytes_vec),
                        _ => panic!("Unable to decode input address"),
                    };
                    let tx_out: pallas_primitives::conway::TransactionOutput =
                        pallas_primitives::conway::TransactionOutput::PostAlonzo(
                            pallas_primitives::conway::PostAlonzoTransactionOutput {
                                address: address_bytes,
                                value: val.clone(),
                                datum_option: datum_opt.clone().map(|x| x.into()),
                                script_ref: script_ref.clone(),
                            }.into(),
                        );
                    let multi_era_out: MultiEraOutput =
                        MultiEraOutput::Conway(Box::new(Cow::Owned(tx_out)));
                    utxos.insert(multi_era_in, multi_era_out);
                }
            }
        }
        None => panic!("UTxO addition error - collateral input missing"),
    }
}

pub fn add_codec_safe_collateral_conway<'a>(
    tx_body: &pallas_primitives::conway::TransactionBody,
    utxos: &mut UTxOs<'a>,
    collateral_info: &'a mut Vec<(
        String, // address in string format
        pallas_primitives::conway::Value,
        Option<pallas_codec::utils::KeepRaw<'a, pallas_primitives::conway::DatumOption>>,
        Option<CborWrap<pallas_primitives::conway::ScriptRef>>,
        Vec<u8>, // Placeholder for CBOR data.
    )>,
) {
    match &tx_body.collateral {
        Some(collaterals) => {
            if collaterals.is_empty() {
                panic!("UTxO addition error - collateral input missing")
            } else {
                for (tx_in, (addr, val, datum_opt, script_ref, cbor)) in
                    zip(collaterals.clone().to_vec(), collateral_info)
                {
                    let multi_era_in: MultiEraInput =
                        MultiEraInput::AlonzoCompatible(Box::new(Cow::Owned(tx_in)));
                    let address_bytes: Bytes = match hex::decode(addr) {
                        Ok(bytes_vec) => Bytes::from(bytes_vec),
                        _ => panic!("Unable to decode input address"),
                    };
                    let post_alonzo =
                        pallas_primitives::conway::PostAlonzoTransactionOutput {
                            address: address_bytes,
                            value: val.clone(),
                            datum_option: datum_opt.clone(),
                            script_ref: script_ref.clone(),
                        };
                    *cbor = minicbor::to_vec(post_alonzo).unwrap();
                    let post_alonzo = minicbor::decode::<
                            pallas_codec::utils::KeepRaw<'a, pallas_primitives::conway::PostAlonzoTransactionOutput
                                                         >>(
                        cbor
                    ).unwrap();
                    let tx_out = pallas_primitives::conway::TransactionOutput::PostAlonzo(post_alonzo);
                    let multi_era_out: MultiEraOutput =
                        MultiEraOutput::Conway(Box::new(Cow::Owned(tx_out)));
                    utxos.insert(multi_era_in, multi_era_out);
                }
            }
        }
        None => panic!("UTxO addition error - collateral input missing"),
    }
}

pub fn add_ref_input_babbage<'a>(
    tx_body: &pallas_primitives::babbage::TransactionBody,
    utxos: &mut UTxOs<'a>,
    ref_input_info: &'a [(
        String, // address in string format
        Value,
        Option<pallas_primitives::babbage::DatumOption>,
        Option<CborWrap<pallas_primitives::babbage::ScriptRef>>,
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
                    let tx_out: pallas_primitives::babbage::TransactionOutput =
                        pallas_primitives::babbage::TransactionOutput::PostAlonzo(
                            pallas_primitives::babbage::PostAlonzoTransactionOutput {
                                address: address_bytes,
                                value: val.clone(),
                                datum_option: datum_opt.clone().map(|x| x.into()),
                                script_ref: script_ref.clone(),
                            }.into(),
                        );
                    let multi_era_out: MultiEraOutput =
                        MultiEraOutput::Babbage(Box::new(Cow::Owned(tx_out)));
                    utxos.insert(multi_era_in, multi_era_out);
                }
            }
        }
        None => panic!("UTxO addition error - reference input missing"),
    }
}

pub fn add_ref_input_conway<'a>(
    tx_body: &pallas_primitives::conway::TransactionBody,
    utxos: &mut UTxOs<'a>,
    ref_input_info: &'a [(
        String, // address in string format
        pallas_primitives::conway::Value,
        Option<pallas_primitives::conway::DatumOption>,
        Option<CborWrap<pallas_primitives::conway::ScriptRef>>,
    )],
) {
    match &tx_body.reference_inputs {
        Some(ref_inputs) => {
            if ref_inputs.is_empty() {
                panic!("UTxO addition error - reference input missing")
            } else {
                for (tx_in, (addr, val, datum_opt, script_ref)) in
                    zip(ref_inputs.clone().to_vec(), ref_input_info)
                {
                    let multi_era_in: MultiEraInput =
                        MultiEraInput::AlonzoCompatible(Box::new(Cow::Owned(tx_in)));
                    let address_bytes: Bytes = match hex::decode(addr) {
                        Ok(bytes_vec) => Bytes::from(bytes_vec),
                        _ => panic!("Unable to decode input address"),
                    };
                    let tx_out: pallas_primitives::conway::TransactionOutput =
                        pallas_primitives::conway::TransactionOutput::PostAlonzo(
                            pallas_primitives::conway::PostAlonzoTransactionOutput {
                                address: address_bytes,
                                value: val.clone(),
                                datum_option: datum_opt.clone().map(|x| x.into()),
                                script_ref: script_ref.clone(),
                            }.into(),
                        );
                    let multi_era_out: MultiEraOutput =
                        MultiEraOutput::Conway(Box::new(Cow::Owned(tx_out)));
                    utxos.insert(multi_era_in, multi_era_out);
                }
            }
        }
        None => panic!("UTxO addition error - reference input missing"),
    }
}

pub fn add_codec_safe_ref_input_conway<'a>(
    tx_body: &pallas_primitives::conway::TransactionBody,
    utxos: &mut UTxOs<'a>,
    ref_input_info: &'a mut Vec<(
        String, // address in string format
        pallas_primitives::conway::Value,
        Option<pallas_codec::utils::KeepRaw<'a, pallas_primitives::conway::DatumOption>>,
        Option<CborWrap<pallas_primitives::conway::ScriptRef>>,
        Vec<u8>, // Placeholder for CBOR data.
    )>,
) {
    match &tx_body.reference_inputs {
        Some(ref_inputs) => {
            if ref_inputs.is_empty() {
                panic!("UTxO addition error - reference input missing")
            } else {
                for (tx_in, (addr, val, datum_opt, script_ref, cbor)) in
                    zip(ref_inputs.clone().to_vec(), ref_input_info)
                {
                    let multi_era_in: MultiEraInput =
                        MultiEraInput::AlonzoCompatible(Box::new(Cow::Owned(tx_in)));
                    let address_bytes: Bytes = match hex::decode(addr) {
                        Ok(bytes_vec) => Bytes::from(bytes_vec),
                        _ => panic!("Unable to decode input address"),
                    };
                    let post_alonzo =
                        pallas_primitives::conway::PostAlonzoTransactionOutput {
                            address: address_bytes,
                            value: val.clone(),
                            datum_option: datum_opt.clone(),
                            script_ref: script_ref.clone(),
                        };
                    *cbor = minicbor::to_vec(post_alonzo).unwrap();
                    let post_alonzo = minicbor::decode::<
                            pallas_codec::utils::KeepRaw<'a, pallas_primitives::conway::PostAlonzoTransactionOutput
                                                         >>(
                        cbor
                    ).unwrap();
                    let tx_out = pallas_primitives::conway::TransactionOutput::PostAlonzo(post_alonzo);
                    let multi_era_out: MultiEraOutput =
                        MultiEraOutput::Conway(Box::new(Cow::Owned(tx_out)));
                    utxos.insert(multi_era_in, multi_era_out);
                }
            }
        }
        None => panic!("UTxO addition error - reference input missing"),
    }
}
