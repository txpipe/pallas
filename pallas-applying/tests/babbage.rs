pub mod common;

use common::*;
use hex;
use pallas_addresses::{Address, ShelleyAddress, ShelleyPaymentPart};
use pallas_applying::{
    utils::{
        BabbageError, BabbageProtParams, Environment, FeePolicy, MultiEraProtParams,
        ValidationError::*,
    },
    validate, UTxOs,
};
use pallas_codec::minicbor::{
    decode::{Decode, Decoder},
    encode,
};
use pallas_codec::utils::{Bytes, CborWrap, KeyValuePairs};
use pallas_primitives::babbage::{
    MintedDatumOption, MintedPostAlonzoTransactionOutput, MintedScriptRef, MintedTransactionBody,
    MintedTx, PseudoDatumOption, PseudoTransactionOutput, Value,
};
use pallas_traverse::{MultiEraInput, MultiEraOutput, MultiEraTx};
use std::borrow::Cow;

#[cfg(test)]
mod babbage_tests {
    use super::*;

    #[test]
    // Transaction hash:
    // b17d685c42e714238c1fb3abcd40e5c6291ebbb420c9c69b641209607bd00c7d
    fn successful_mainnet_tx() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/babbage3.tx"));
        let mtx: MintedTx = babbage_minted_tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_babbage(&mtx);
        let tx_outs_info: &[(
            String,
            Value,
            Option<MintedDatumOption>,
            Option<CborWrap<MintedScriptRef>>,
        )] = &[(
            String::from(include_str!("../../test_data/babbage3.address")),
            Value::Coin(103324335),
            None,
            None,
        )];
        let utxos: UTxOs = mk_utxo_for_babbage_tx(&mtx.transaction_body, tx_outs_info);
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Babbage(BabbageProtParams {
                fee_policy: FeePolicy {
                    summand: 155381,
                    multiplier: 44,
                },
                max_tx_size: 16384,
                max_block_ex_mem: 62000000,
                max_block_ex_steps: 40000000000,
                max_tx_ex_mem: 14000000,
                max_tx_ex_steps: 10000000000,
                max_val_size: 5000,
                collateral_percent: 150,
                max_collateral_inputs: 3,
                coins_per_utxo_word: 4310,
            }),
            prot_magic: 764824073,
            block_slot: 72316896,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => (),
            Err(err) => assert!(false, "Unexpected error ({:?})", err),
        }
    }

    #[test]
    // Transaction hash:
    // f33d6f7eb877132af7307e385bb24a7d2c12298c8ac0b1460296748810925ccc
    fn successful_mainnet_tx_with_plutus_script() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/babbage4.tx"));
        let mtx: MintedTx = babbage_minted_tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_babbage(&mtx);
        let tx_outs_info: &[(
            String,
            Value,
            Option<MintedDatumOption>,
            Option<CborWrap<MintedScriptRef>>,
        )] = &[
            (
                String::from(include_str!("../../test_data/babbage4.0.address")),
                Value::Coin(25000000),
                Some(PseudoDatumOption::Hash(
                    hex::decode("3E8C4B1D396BB8132E5097F5A2F012D97900CBC496A3745DB4226CEA4CB66465")
                        .unwrap()
                        .as_slice()
                        .into(),
                )),
                None,
            ),
            (
                String::from(include_str!("../../test_data/babbage4.1.address")),
                Value::Multiasset(
                    1795660,
                    KeyValuePairs::from(Vec::from([(
                        "787f0c946b98153500edc0a753e65457250544da8486b17c85708135"
                            .parse()
                            .unwrap(),
                        KeyValuePairs::from(Vec::from([(
                            Bytes::from(
                                hex::decode("506572666563744c6567656e64617279446572705365616c")
                                    .unwrap(),
                            ),
                            1,
                        )])),
                    )])),
                ),
                None,
                None,
            ),
        ];
        let mut utxos: UTxOs = mk_utxo_for_babbage_tx(&mtx.transaction_body, tx_outs_info);
        let collateral_info: &[(
            String,
            Value,
            Option<MintedDatumOption>,
            Option<CborWrap<MintedScriptRef>>,
        )] = &[(
            String::from(include_str!("../../test_data/babbage4.collateral.address")),
            Value::Coin(5000000),
            None,
            None,
        )];
        add_collateral_babbage(&mtx.transaction_body, &mut utxos, collateral_info);
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Babbage(BabbageProtParams {
                fee_policy: FeePolicy {
                    summand: 155381,
                    multiplier: 44,
                },
                max_tx_size: 16384,
                max_block_ex_mem: 62000000,
                max_block_ex_steps: 40000000000,
                max_tx_ex_mem: 14000000,
                max_tx_ex_steps: 10000000000,
                max_val_size: 5000,
                collateral_percent: 150,
                max_collateral_inputs: 3,
                coins_per_utxo_word: 4310,
            }),
            prot_magic: 764824073,
            block_slot: 72317003,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => (),
            Err(err) => assert!(false, "Unexpected error ({:?})", err),
        }
    }

    #[test]
    // Same as succesful_mainnet_tx, except that all inputs are removed.
    fn empty_ins() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/babbage3.tx"));
        let mut mtx: MintedTx = babbage_minted_tx_from_cbor(&cbor_bytes);
        let tx_outs_info: &[(
            String,
            Value,
            Option<MintedDatumOption>,
            Option<CborWrap<MintedScriptRef>>,
        )] = &[(
            String::from(include_str!("../../test_data/babbage3.address")),
            Value::Coin(103324335),
            None,
            None,
        )];
        let utxos: UTxOs = mk_utxo_for_babbage_tx(&mtx.transaction_body, tx_outs_info);
        let mut tx_body: MintedTransactionBody = (*mtx.transaction_body).clone();
        tx_body.inputs = Vec::new();
        let mut tx_buf: Vec<u8> = Vec::new();
        let _ = encode(tx_body, &mut tx_buf);
        mtx.transaction_body =
            Decode::decode(&mut Decoder::new(&tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_babbage(&mtx);
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Babbage(BabbageProtParams {
                fee_policy: FeePolicy {
                    summand: 155381,
                    multiplier: 44,
                },
                max_tx_size: 16384,
                max_block_ex_mem: 62000000,
                max_block_ex_steps: 40000000000,
                max_tx_ex_mem: 14000000,
                max_tx_ex_steps: 10000000000,
                max_val_size: 5000,
                collateral_percent: 150,
                max_collateral_inputs: 3,
                coins_per_utxo_word: 4310,
            }),
            prot_magic: 764824073,
            block_slot: 72316896,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => assert!(false, "Inputs set should not be empty"),
            Err(err) => match err {
                Babbage(BabbageError::TxInsEmpty) => (),
                _ => assert!(false, "Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Same as succesful_mainnet_tx, but validation is called with an empty UTxO
    // set.
    fn unfound_utxo_input() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/babbage3.tx"));
        let mtx: MintedTx = babbage_minted_tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_babbage(&mtx);
        let utxos: UTxOs = UTxOs::new();
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Babbage(BabbageProtParams {
                fee_policy: FeePolicy {
                    summand: 155381,
                    multiplier: 44,
                },
                max_tx_size: 16384,
                max_block_ex_mem: 62000000,
                max_block_ex_steps: 40000000000,
                max_tx_ex_mem: 14000000,
                max_tx_ex_steps: 10000000000,
                max_val_size: 5000,
                collateral_percent: 150,
                max_collateral_inputs: 3,
                coins_per_utxo_word: 4310,
            }),
            prot_magic: 764824073,
            block_slot: 72316896,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => assert!(false, "All inputs should be within the UTxO set"),
            Err(err) => match err {
                Babbage(BabbageError::InputNotInUTxO) => (),
                _ => assert!(false, "Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Same as succesful_mainnet_tx, except that the lower bound of the validity
    // interval is greater than the block slot.
    fn validity_interval_lower_bound_unreached() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/babbage3.tx"));
        let mut mtx: MintedTx = babbage_minted_tx_from_cbor(&cbor_bytes);
        let tx_outs_info: &[(
            String,
            Value,
            Option<MintedDatumOption>,
            Option<CborWrap<MintedScriptRef>>,
        )] = &[(
            String::from(include_str!("../../test_data/babbage3.address")),
            Value::Coin(103324335),
            None,
            None,
        )];
        let utxos: UTxOs = mk_utxo_for_babbage_tx(&mtx.transaction_body, tx_outs_info);
        let mut tx_body: MintedTransactionBody = (*mtx.transaction_body).clone();
        tx_body.validity_interval_start = Some(72316897); // One slot after the block.
        let mut tx_buf: Vec<u8> = Vec::new();
        let _ = encode(tx_body, &mut tx_buf);
        mtx.transaction_body =
            Decode::decode(&mut Decoder::new(&tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_babbage(&mtx);
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Babbage(BabbageProtParams {
                fee_policy: FeePolicy {
                    summand: 155381,
                    multiplier: 44,
                },
                max_tx_size: 16384,
                max_block_ex_mem: 62000000,
                max_block_ex_steps: 40000000000,
                max_tx_ex_mem: 14000000,
                max_tx_ex_steps: 10000000000,
                max_val_size: 5000,
                collateral_percent: 150,
                max_collateral_inputs: 3,
                coins_per_utxo_word: 4310,
            }),
            prot_magic: 764824073,
            block_slot: 72316896,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => assert!(
                false,
                "Validity interval lower bound should have been reached"
            ),
            Err(err) => match err {
                Babbage(BabbageError::BlockPrecedesValInt) => (),
                _ => assert!(false, "Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Same as succesful_mainnet_tx, except that the upper bound of the validity
    // interval is lower than the block slot.
    fn validity_interval_upper_bound_surpassed() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/babbage3.tx"));
        let mut mtx: MintedTx = babbage_minted_tx_from_cbor(&cbor_bytes);
        let tx_outs_info: &[(
            String,
            Value,
            Option<MintedDatumOption>,
            Option<CborWrap<MintedScriptRef>>,
        )] = &[(
            String::from(include_str!("../../test_data/babbage3.address")),
            Value::Coin(103324335),
            None,
            None,
        )];
        let utxos: UTxOs = mk_utxo_for_babbage_tx(&mtx.transaction_body, tx_outs_info);
        let mut tx_body: MintedTransactionBody = (*mtx.transaction_body).clone();
        tx_body.ttl = Some(72316895); // One slot before the block.
        let mut tx_buf: Vec<u8> = Vec::new();
        let _ = encode(tx_body, &mut tx_buf);
        mtx.transaction_body =
            Decode::decode(&mut Decoder::new(&tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_babbage(&mtx);
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Babbage(BabbageProtParams {
                fee_policy: FeePolicy {
                    summand: 155381,
                    multiplier: 44,
                },
                max_tx_size: 16384,
                max_block_ex_mem: 62000000,
                max_block_ex_steps: 40000000000,
                max_tx_ex_mem: 14000000,
                max_tx_ex_steps: 10000000000,
                max_val_size: 5000,
                collateral_percent: 150,
                max_collateral_inputs: 3,
                coins_per_utxo_word: 4310,
            }),
            prot_magic: 764824073,
            block_slot: 72316896,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => assert!(
                false,
                "Validity interval upper bound should not have been surpassed"
            ),
            Err(err) => match err {
                Babbage(BabbageError::BlockExceedsValInt) => (),
                _ => assert!(false, "Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Same as succesful_mainnet_tx, except that validation is called with an
    // Environment requesting fees that exceed those paid by the transaction.
    fn min_fee_unreached() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/babbage3.tx"));
        let mtx: MintedTx = babbage_minted_tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_babbage(&mtx);
        let tx_outs_info: &[(
            String,
            Value,
            Option<MintedDatumOption>,
            Option<CborWrap<MintedScriptRef>>,
        )] = &[(
            String::from(include_str!("../../test_data/babbage3.address")),
            Value::Coin(103324335),
            None,
            None,
        )];
        let utxos: UTxOs = mk_utxo_for_babbage_tx(&mtx.transaction_body, tx_outs_info);
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Babbage(BabbageProtParams {
                fee_policy: FeePolicy {
                    summand: 155381,
                    multiplier: 76, // This value was 44 during Babbage on mainnet.
                },
                max_tx_size: 16384,
                max_block_ex_mem: 62000000,
                max_block_ex_steps: 40000000000,
                max_tx_ex_mem: 14000000,
                max_tx_ex_steps: 10000000000,
                max_val_size: 5000,
                collateral_percent: 150,
                max_collateral_inputs: 3,
                coins_per_utxo_word: 4310,
            }),
            prot_magic: 764824073,
            block_slot: 72316896,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => assert!(false, "Fee should not be below minimum"),
            Err(err) => match err {
                Babbage(BabbageError::FeeBelowMin) => (),
                _ => assert!(false, "Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Same as successful_mainnet_tx_with_plutus_script, except that all collaterals
    // are removed before calling validation.
    fn no_collateral_inputs() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/babbage4.tx"));
        let mut mtx: MintedTx = babbage_minted_tx_from_cbor(&cbor_bytes);
        let tx_outs_info: &[(
            String,
            Value,
            Option<MintedDatumOption>,
            Option<CborWrap<MintedScriptRef>>,
        )] = &[
            (
                String::from(include_str!("../../test_data/babbage4.0.address")),
                Value::Coin(25000000),
                Some(PseudoDatumOption::Hash(
                    hex::decode("3E8C4B1D396BB8132E5097F5A2F012D97900CBC496A3745DB4226CEA4CB66465")
                        .unwrap()
                        .as_slice()
                        .into(),
                )),
                None,
            ),
            (
                String::from(include_str!("../../test_data/babbage4.1.address")),
                Value::Multiasset(
                    1795660,
                    KeyValuePairs::from(Vec::from([(
                        "787f0c946b98153500edc0a753e65457250544da8486b17c85708135"
                            .parse()
                            .unwrap(),
                        KeyValuePairs::from(Vec::from([(
                            Bytes::from(
                                hex::decode("506572666563744c6567656e64617279446572705365616c")
                                    .unwrap(),
                            ),
                            1,
                        )])),
                    )])),
                ),
                None,
                None,
            ),
        ];
        let mut utxos: UTxOs = mk_utxo_for_babbage_tx(&mtx.transaction_body, tx_outs_info);
        let collateral_info: &[(
            String,
            Value,
            Option<MintedDatumOption>,
            Option<CborWrap<MintedScriptRef>>,
        )] = &[(
            String::from(include_str!("../../test_data/babbage4.collateral.address")),
            Value::Coin(5000000),
            None,
            None,
        )];
        add_collateral_babbage(&mtx.transaction_body, &mut utxos, collateral_info);
        let mut tx_body: MintedTransactionBody = (*mtx.transaction_body).clone();
        tx_body.collateral = None;
        let mut tx_buf: Vec<u8> = Vec::new();
        let _ = encode(tx_body, &mut tx_buf);
        mtx.transaction_body =
            Decode::decode(&mut Decoder::new(&tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_babbage(&mtx);
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Babbage(BabbageProtParams {
                fee_policy: FeePolicy {
                    summand: 155381,
                    multiplier: 44,
                },
                max_tx_size: 16384,
                max_block_ex_mem: 62000000,
                max_block_ex_steps: 40000000000,
                max_tx_ex_mem: 14000000,
                max_tx_ex_steps: 10000000000,
                max_val_size: 5000,
                collateral_percent: 150,
                max_collateral_inputs: 3,
                coins_per_utxo_word: 4310,
            }),
            prot_magic: 764824073,
            block_slot: 72316896,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => assert!(false, "No collateral inputs"),
            Err(err) => match err {
                Babbage(BabbageError::CollateralMissing) => (),
                _ => assert!(false, "Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Same as successful_mainnet_tx_with_plutus_script, except that validation is
    // called on an environment which does not allow enough collateral inputs
    // for the transaction to be valid.
    fn too_many_collateral_inputs() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/babbage4.tx"));
        let mtx: MintedTx = babbage_minted_tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_babbage(&mtx);
        let tx_outs_info: &[(
            String,
            Value,
            Option<MintedDatumOption>,
            Option<CborWrap<MintedScriptRef>>,
        )] = &[
            (
                String::from(include_str!("../../test_data/babbage4.0.address")),
                Value::Coin(25000000),
                Some(PseudoDatumOption::Hash(
                    hex::decode("3E8C4B1D396BB8132E5097F5A2F012D97900CBC496A3745DB4226CEA4CB66465")
                        .unwrap()
                        .as_slice()
                        .into(),
                )),
                None,
            ),
            (
                String::from(include_str!("../../test_data/babbage4.1.address")),
                Value::Multiasset(
                    1795660,
                    KeyValuePairs::from(Vec::from([(
                        "787f0c946b98153500edc0a753e65457250544da8486b17c85708135"
                            .parse()
                            .unwrap(),
                        KeyValuePairs::from(Vec::from([(
                            Bytes::from(
                                hex::decode("506572666563744c6567656e64617279446572705365616c")
                                    .unwrap(),
                            ),
                            1,
                        )])),
                    )])),
                ),
                None,
                None,
            ),
        ];
        let mut utxos: UTxOs = mk_utxo_for_babbage_tx(&mtx.transaction_body, tx_outs_info);
        let collateral_info: &[(
            String,
            Value,
            Option<MintedDatumOption>,
            Option<CborWrap<MintedScriptRef>>,
        )] = &[(
            String::from(include_str!("../../test_data/babbage4.collateral.address")),
            Value::Coin(5000000),
            None,
            None,
        )];
        add_collateral_babbage(&mtx.transaction_body, &mut utxos, collateral_info);
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Babbage(BabbageProtParams {
                fee_policy: FeePolicy {
                    summand: 155381,
                    multiplier: 44,
                },
                max_tx_size: 16384,
                max_block_ex_mem: 62000000,
                max_block_ex_steps: 40000000000,
                max_tx_ex_mem: 14000000,
                max_tx_ex_steps: 10000000000,
                max_val_size: 5000,
                collateral_percent: 150,
                max_collateral_inputs: 0, // no collateral inputs are allowed
                coins_per_utxo_word: 4310,
            }),
            prot_magic: 764824073,
            block_slot: 72317003,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => assert!(false, "Number of collateral inputs should be within limits"),
            Err(err) => match err {
                Babbage(BabbageError::TooManyCollaterals) => (),
                _ => assert!(false, "Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Same as successful_mainnet_tx_with_plutus_script, except that the address of
    // a collateral inputs is altered into a script-locked one.
    fn collateral_is_not_verification_key_locked() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/babbage4.tx"));
        let mtx: MintedTx = babbage_minted_tx_from_cbor(&cbor_bytes);
        let tx_outs_info: &[(
            String,
            Value,
            Option<MintedDatumOption>,
            Option<CborWrap<MintedScriptRef>>,
        )] = &[
            (
                String::from(include_str!("../../test_data/babbage4.0.address")),
                Value::Coin(25000000),
                Some(PseudoDatumOption::Hash(
                    hex::decode("3E8C4B1D396BB8132E5097F5A2F012D97900CBC496A3745DB4226CEA4CB66465")
                        .unwrap()
                        .as_slice()
                        .into(),
                )),
                None,
            ),
            (
                String::from(include_str!("../../test_data/babbage4.1.address")),
                Value::Multiasset(
                    1795660,
                    KeyValuePairs::from(Vec::from([(
                        "787f0c946b98153500edc0a753e65457250544da8486b17c85708135"
                            .parse()
                            .unwrap(),
                        KeyValuePairs::from(Vec::from([(
                            Bytes::from(
                                hex::decode("506572666563744c6567656e64617279446572705365616c")
                                    .unwrap(),
                            ),
                            1,
                        )])),
                    )])),
                ),
                None,
                None,
            ),
        ];
        let mut utxos: UTxOs = mk_utxo_for_babbage_tx(&mtx.transaction_body, tx_outs_info);
        let old_address: Address = match hex::decode(String::from(include_str!(
            "../../test_data/babbage4.collateral.address"
        ))) {
            Ok(bytes_vec) => Address::from_bytes(bytes_vec.as_slice()).unwrap(),
            _ => panic!("Unable to parse collateral input address"),
        };
        let old_shelley_address: ShelleyAddress = match old_address {
            Address::Shelley(shelley_addr) => shelley_addr,
            _ => panic!("Unable to parse collateral input address"),
        };
        let altered_address: ShelleyAddress = ShelleyAddress::new(
            old_shelley_address.network(),
            ShelleyPaymentPart::Script(old_shelley_address.payment().as_hash().clone()),
            old_shelley_address.delegation().clone(),
        );
        let tx_in = mtx
            .transaction_body
            .collateral
            .clone()
            .unwrap()
            .pop()
            .unwrap();
        let multi_era_in: MultiEraInput =
            MultiEraInput::AlonzoCompatible(Box::new(Cow::Owned(tx_in.clone())));
        let multi_era_out: MultiEraOutput = MultiEraOutput::Babbage(Box::new(Cow::Owned(
            PseudoTransactionOutput::PostAlonzo(MintedPostAlonzoTransactionOutput {
                address: Bytes::try_from(altered_address.to_hex()).unwrap(),
                value: Value::Coin(5000000),
                datum_option: None,
                script_ref: None,
            }),
        )));
        utxos.insert(multi_era_in, multi_era_out);
        let metx: MultiEraTx = MultiEraTx::from_babbage(&mtx);
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Babbage(BabbageProtParams {
                fee_policy: FeePolicy {
                    summand: 155381,
                    multiplier: 44,
                },
                max_tx_size: 16384,
                max_block_ex_mem: 62000000,
                max_block_ex_steps: 40000000000,
                max_tx_ex_mem: 14000000,
                max_tx_ex_steps: 10000000000,
                max_val_size: 5000,
                collateral_percent: 150,
                max_collateral_inputs: 3,
                coins_per_utxo_word: 4310,
            }),
            prot_magic: 764824073,
            block_slot: 72316896,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => assert!(false, "Collateral inputs should be verification-key locked"),
            Err(err) => match err {
                Babbage(BabbageError::CollateralNotVKeyLocked) => (),
                _ => assert!(false, "Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Same as succesful_mainnet_tx_with_plutus_script, except that the balance
    // between assets in collateral inputs and assets in collateral return output
    // contains assets other than lovelace.
    fn collateral_with_other_assets() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/babbage4.tx"));
        let mtx: MintedTx = babbage_minted_tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_babbage(&mtx);
        let tx_outs_info: &[(
            String,
            Value,
            Option<MintedDatumOption>,
            Option<CborWrap<MintedScriptRef>>,
        )] = &[
            (
                String::from(include_str!("../../test_data/babbage4.0.address")),
                Value::Coin(25000000),
                Some(PseudoDatumOption::Hash(
                    hex::decode("3E8C4B1D396BB8132E5097F5A2F012D97900CBC496A3745DB4226CEA4CB66465")
                        .unwrap()
                        .as_slice()
                        .into(),
                )),
                None,
            ),
            (
                String::from(include_str!("../../test_data/babbage4.1.address")),
                Value::Multiasset(
                    1795660,
                    KeyValuePairs::from(Vec::from([(
                        "787f0c946b98153500edc0a753e65457250544da8486b17c85708135"
                            .parse()
                            .unwrap(),
                        KeyValuePairs::from(Vec::from([(
                            Bytes::from(
                                hex::decode("506572666563744c6567656e64617279446572705365616c")
                                    .unwrap(),
                            ),
                            1,
                        )])),
                    )])),
                ),
                None,
                None,
            ),
        ];
        let mut utxos: UTxOs = mk_utxo_for_babbage_tx(&mtx.transaction_body, tx_outs_info);
        let collateral_info: &[(
            String,
            Value,
            Option<MintedDatumOption>,
            Option<CborWrap<MintedScriptRef>>,
        )] = &[(
            String::from(include_str!("../../test_data/babbage4.collateral.address")),
            Value::Multiasset(
                5000000,
                KeyValuePairs::from(Vec::from([(
                    "b001076b34a87e7d48ec46703a6f50f93289582ad9bdbeff7f1e3295"
                        .parse()
                        .unwrap(),
                    KeyValuePairs::from(Vec::from([(
                        Bytes::from(hex::decode("4879706562656173747332343233").unwrap()),
                        1000,
                    )])),
                )])),
            ),
            None,
            None,
        )];
        add_collateral_babbage(&mtx.transaction_body, &mut utxos, collateral_info);
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Babbage(BabbageProtParams {
                fee_policy: FeePolicy {
                    summand: 155381,
                    multiplier: 44,
                },
                max_tx_size: 16384,
                max_block_ex_mem: 62000000,
                max_block_ex_steps: 40000000000,
                max_tx_ex_mem: 14000000,
                max_tx_ex_steps: 10000000000,
                max_val_size: 5000,
                collateral_percent: 150,
                max_collateral_inputs: 3,
                coins_per_utxo_word: 4310,
            }),
            prot_magic: 764824073,
            block_slot: 72317003,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => assert!(false, "Collateral balance should contained only lovelace"),
            Err(err) => match err {
                Babbage(BabbageError::NonLovelaceCollateral) => (),
                _ => assert!(false, "Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Same as successful_mainnet_tx_with_plutus_script, except that the number of
    // lovelace in the total collateral balance is insufficient.
    fn collateral_min_lovelace() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/babbage4.tx"));
        let mtx: MintedTx = babbage_minted_tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_babbage(&mtx);
        let tx_outs_info: &[(
            String,
            Value,
            Option<MintedDatumOption>,
            Option<CborWrap<MintedScriptRef>>,
        )] = &[
            (
                String::from(include_str!("../../test_data/babbage4.0.address")),
                Value::Coin(25000000),
                Some(PseudoDatumOption::Hash(
                    hex::decode("3E8C4B1D396BB8132E5097F5A2F012D97900CBC496A3745DB4226CEA4CB66465")
                        .unwrap()
                        .as_slice()
                        .into(),
                )),
                None,
            ),
            (
                String::from(include_str!("../../test_data/babbage4.1.address")),
                Value::Multiasset(
                    1795660,
                    KeyValuePairs::from(Vec::from([(
                        "787f0c946b98153500edc0a753e65457250544da8486b17c85708135"
                            .parse()
                            .unwrap(),
                        KeyValuePairs::from(Vec::from([(
                            Bytes::from(
                                hex::decode("506572666563744c6567656e64617279446572705365616c")
                                    .unwrap(),
                            ),
                            1,
                        )])),
                    )])),
                ),
                None,
                None,
            ),
        ];
        let mut utxos: UTxOs = mk_utxo_for_babbage_tx(&mtx.transaction_body, tx_outs_info);
        let collateral_info: &[(
            String,
            Value,
            Option<MintedDatumOption>,
            Option<CborWrap<MintedScriptRef>>,
        )] = &[(
            String::from(include_str!("../../test_data/babbage4.collateral.address")),
            Value::Coin(5000000),
            None,
            None,
        )];
        add_collateral_babbage(&mtx.transaction_body, &mut utxos, collateral_info);
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Babbage(BabbageProtParams {
                fee_policy: FeePolicy {
                    summand: 155381,
                    multiplier: 44,
                },
                max_tx_size: 16384,
                max_block_ex_mem: 62000000,
                max_block_ex_steps: 40000000000,
                max_tx_ex_mem: 14000000,
                max_tx_ex_steps: 10000000000,
                max_val_size: 5000,
                collateral_percent: 728, // This value was 150 during Babbage on mainnet.
                max_collateral_inputs: 3,
                coins_per_utxo_word: 4310,
            }),
            prot_magic: 764824073,
            block_slot: 72317003,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => assert!(
                false,
                "Collateral balance should contained the minimum lovelace"
            ),
            Err(err) => match err {
                Babbage(BabbageError::CollateralMinLovelace) => (),
                _ => assert!(false, "Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Same as successful_mainnet_tx_with_plutus_script, except that the annotated
    // collateral is wrong.
    fn collateral_annotation() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/babbage4.tx"));
        let mut mtx: MintedTx = babbage_minted_tx_from_cbor(&cbor_bytes);
        let tx_outs_info: &[(
            String,
            Value,
            Option<MintedDatumOption>,
            Option<CborWrap<MintedScriptRef>>,
        )] = &[
            (
                String::from(include_str!("../../test_data/babbage4.0.address")),
                Value::Coin(25000000),
                Some(PseudoDatumOption::Hash(
                    hex::decode("3E8C4B1D396BB8132E5097F5A2F012D97900CBC496A3745DB4226CEA4CB66465")
                        .unwrap()
                        .as_slice()
                        .into(),
                )),
                None,
            ),
            (
                String::from(include_str!("../../test_data/babbage4.1.address")),
                Value::Multiasset(
                    1795660,
                    KeyValuePairs::from(Vec::from([(
                        "787f0c946b98153500edc0a753e65457250544da8486b17c85708135"
                            .parse()
                            .unwrap(),
                        KeyValuePairs::from(Vec::from([(
                            Bytes::from(
                                hex::decode("506572666563744c6567656e64617279446572705365616c")
                                    .unwrap(),
                            ),
                            1,
                        )])),
                    )])),
                ),
                None,
                None,
            ),
        ];
        let mut utxos: UTxOs = mk_utxo_for_babbage_tx(&mtx.transaction_body, tx_outs_info);
        let collateral_info: &[(
            String,
            Value,
            Option<MintedDatumOption>,
            Option<CborWrap<MintedScriptRef>>,
        )] = &[(
            String::from(include_str!("../../test_data/babbage4.collateral.address")),
            Value::Coin(5000000),
            None,
            None,
        )];
        add_collateral_babbage(&mtx.transaction_body, &mut utxos, collateral_info);
        let mut tx_body: MintedTransactionBody = (*mtx.transaction_body).clone();
        tx_body.total_collateral = Some(5000001); // This is 1 more than the actual paid collateral
        let mut tx_buf: Vec<u8> = Vec::new();
        let _ = encode(tx_body, &mut tx_buf);
        mtx.transaction_body =
            Decode::decode(&mut Decoder::new(&tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_babbage(&mtx);
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Babbage(BabbageProtParams {
                fee_policy: FeePolicy {
                    summand: 155381,
                    multiplier: 44,
                },
                max_tx_size: 16384,
                max_block_ex_mem: 62000000,
                max_block_ex_steps: 40000000000,
                max_tx_ex_mem: 14000000,
                max_tx_ex_steps: 10000000000,
                max_val_size: 5000,
                collateral_percent: 150,
                max_collateral_inputs: 3,
                coins_per_utxo_word: 4310,
            }),
            prot_magic: 764824073,
            block_slot: 72317003,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => assert!(false, "Collateral annotation"),
            Err(err) => match err {
                Babbage(BabbageError::CollateralAnnotation) => (),
                _ => assert!(false, "Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Same as succesful_mainnet_tx, except that the fee is reduced by exactly 1,
    // and so the "preservation of value" property doesn't hold.
    fn preservation_of_value() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/babbage3.tx"));
        let mut mtx: MintedTx = babbage_minted_tx_from_cbor(&cbor_bytes);
        let tx_outs_info: &[(
            String,
            Value,
            Option<MintedDatumOption>,
            Option<CborWrap<MintedScriptRef>>,
        )] = &[(
            String::from(include_str!("../../test_data/babbage3.address")),
            Value::Coin(103324335),
            None,
            None,
        )];
        let utxos: UTxOs = mk_utxo_for_babbage_tx(&mtx.transaction_body, tx_outs_info);
        let mut tx_body: MintedTransactionBody = (*mtx.transaction_body).clone();
        tx_body.fee = tx_body.fee - 1;
        let mut tx_buf: Vec<u8> = Vec::new();
        let _ = encode(tx_body, &mut tx_buf);
        mtx.transaction_body =
            Decode::decode(&mut Decoder::new(&tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_babbage(&mtx);
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Babbage(BabbageProtParams {
                fee_policy: FeePolicy {
                    summand: 155381,
                    multiplier: 44,
                },
                max_tx_size: 16384,
                max_block_ex_mem: 62000000,
                max_block_ex_steps: 40000000000,
                max_tx_ex_mem: 14000000,
                max_tx_ex_steps: 10000000000,
                max_val_size: 5000,
                collateral_percent: 150,
                max_collateral_inputs: 3,
                coins_per_utxo_word: 4310,
            }),
            prot_magic: 764824073,
            block_slot: 72316896,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => assert!(false, "Preservation of value does not hold"),
            Err(err) => match err {
                Babbage(BabbageError::PreservationOfValue) => (),
                _ => assert!(false, "Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Same as successful_mainnet_tx, except that the minimum lovelace in an output
    // is unreached.
    fn min_lovelace_unreached() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/babbage3.tx"));
        let mtx: MintedTx = babbage_minted_tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_babbage(&mtx);
        let tx_outs_info: &[(
            String,
            Value,
            Option<MintedDatumOption>,
            Option<CborWrap<MintedScriptRef>>,
        )] = &[(
            String::from(include_str!("../../test_data/babbage3.address")),
            Value::Coin(103324335),
            None,
            None,
        )];
        let utxos: UTxOs = mk_utxo_for_babbage_tx(&mtx.transaction_body, tx_outs_info);
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Babbage(BabbageProtParams {
                fee_policy: FeePolicy {
                    summand: 155381,
                    multiplier: 44,
                },
                max_tx_size: 16384,
                max_block_ex_mem: 62000000,
                max_block_ex_steps: 40000000000,
                max_tx_ex_mem: 14000000,
                max_tx_ex_steps: 10000000000,
                max_val_size: 5000,
                collateral_percent: 150,
                max_collateral_inputs: 3,
                coins_per_utxo_word: 10000000, // This was 4310 during Alonzo on mainnet.
            }),
            prot_magic: 764824073,
            block_slot: 72316896,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => assert!(false, "Output minimum lovelace is unreached"),
            Err(err) => match err {
                Babbage(BabbageError::MinLovelaceUnreached) => (),
                _ => assert!(false, "Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Same as successful_mainnet_tx, except that the value size exceeds the
    // environment parameter.
    fn max_val_exceeded() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/babbage3.tx"));
        let mtx: MintedTx = babbage_minted_tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_babbage(&mtx);
        let tx_outs_info: &[(
            String,
            Value,
            Option<MintedDatumOption>,
            Option<CborWrap<MintedScriptRef>>,
        )] = &[(
            String::from(include_str!("../../test_data/babbage3.address")),
            Value::Coin(103324335),
            None,
            None,
        )];
        let utxos: UTxOs = mk_utxo_for_babbage_tx(&mtx.transaction_body, tx_outs_info);
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Babbage(BabbageProtParams {
                fee_policy: FeePolicy {
                    summand: 155381,
                    multiplier: 44,
                },
                max_tx_size: 16384,
                max_block_ex_mem: 62000000,
                max_block_ex_steps: 40000000000,
                max_tx_ex_mem: 14000000,
                max_tx_ex_steps: 10000000000,
                max_val_size: 0,
                collateral_percent: 150,
                max_collateral_inputs: 3,
                coins_per_utxo_word: 4310,
            }),
            prot_magic: 764824073,
            block_slot: 72316896,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => assert!(false, "Max value size exceeded"),
            Err(err) => match err {
                Babbage(BabbageError::MaxValSizeExceeded) => (),
                _ => assert!(false, "Unexpected error ({:?})", err),
            },
        }
    }
}
