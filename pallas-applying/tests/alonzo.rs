pub mod common;

use common::*;
use pallas_addresses::{Address, Network, ShelleyAddress, ShelleyPaymentPart};
use pallas_applying::{
    utils::{
        AlonzoError::*, AlonzoProtParams, Environment, FeePolicy, Language, MultiEraProtParams,
        ValidationError::*,
    },
    validate, UTxOs,
};
use pallas_codec::{
    minicbor::{
        decode::{Decode, Decoder},
        encode,
    },
    utils::{Bytes, KeyValuePairs},
};
use pallas_crypto::hash::Hash;
use pallas_primitives::alonzo::{MintedTx, NetworkId, TransactionBody, TransactionOutput, Value};
use pallas_traverse::{Era, MultiEraInput, MultiEraOutput, MultiEraTx};
use std::borrow::Cow;

#[cfg(test)]
mod alonzo_tests {
    use super::*;

    #[test]
    // Transaction hash:
    // 704b3b9c96f44cd5676e5dcb5dc0bb2555c66427625ccefe620101665da86868
    fn successful_mainnet_tx() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/alonzo1.tx"));
        let mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Alonzo);
        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(
                String::from(include_str!("../../test_data/alonzo1.address")),
                Value::Coin(1549646822),
                None,
            )],
        );
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Alonzo(AlonzoProtParams {
                fee_policy: FeePolicy {
                    summand: 155381,
                    multiplier: 44,
                },
                max_tx_size: 16384,
                languages: vec![Language::PlutusV1, Language::PlutusV2],
                max_block_ex_mem: 50000000,
                max_block_ex_steps: 40000000000,
                max_tx_ex_mem: 10000000,
                max_tx_ex_steps: 10000000000,
                max_val_size: 5000,
                collateral_percent: 150,
                max_collateral_inputs: 3,
                coints_per_utxo_word: 34482,
            }),
            prot_magic: 764824073,
            block_slot: 44237276,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => (),
            Err(err) => assert!(false, "Unexpected error ({:?})", err),
        }
    }

    #[test]
    // Transaction hash:
    // 65160f403d2c7419784ae997d32b93a6679d81468af8173ccd7949df6704f7ba
    fn successful_mainnet_tx_with_plutus_script() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/alonzo2.tx"));
        let mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Alonzo);
        let mut utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[
                (
                    // (tx hash, tx output index):
                    // (117325a52d60be3a1e4072af39d9e630bf61ce59d315d6c1bf4c4d140f8066ea, 0)
                    String::from(include_str!("../../test_data/alonzo2.0.address")),
                    Value::Multiasset(
                        1724100,
                        KeyValuePairs::from(Vec::from([(
                            Hash::<28>::new([
                                176, 1, 7, 107, 52, 168, 126, 125, 72, 236, 70, 112, 58, 111, 80,
                                249, 50, 137, 88, 42, 217, 189, 190, 255, 127, 30, 50, 149,
                            ]),
                            KeyValuePairs::from(Vec::from([(
                                Bytes::from(hex::decode("4879706562656173747332343233").unwrap()),
                                1,
                            )])),
                        )])),
                    ),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (d2f9764fa93ae5bcabbb65c7a2f97d1e31188064ae3d2ba1462114453928dd99, 0)
                    String::from(include_str!("../../test_data/alonzo2.1.address")),
                    Value::Coin(20292207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (9fab354c2825376a943e505d13a3861e4d9ad3e177028d7bb2bbabce5453fa11, 0)
                    String::from(include_str!("../../test_data/alonzo2.2.address")),
                    Value::Coin(20292207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (3077a999b1d22cb1a4e5ee485adbde6a4596704a96384fbc9727028b8b28ba47, 0)
                    String::from(include_str!("../../test_data/alonzo2.3.address")),
                    Value::Coin(29792207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (b231aca45a38add7378d2ed7a0822626fee3396821e8791a5af5926807db962d, 0)
                    String::from(include_str!("../../test_data/alonzo2.4.address")),
                    Value::Coin(29792207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (11579a841b3c7a64aa057c9adf993ef42520570450499b0a724c7ef706b2a435, 0)
                    String::from(include_str!("../../test_data/alonzo2.5.address")),
                    Value::Coin(61233231),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (b857f98162b753d117464c499d53bbbfec5aa38b94bd624e295a7e3fddc77130, 0)
                    String::from(include_str!("../../test_data/alonzo2.6.address")),
                    Value::Coin(20292207),
                    None,
                ),
            ],
        );
        add_collateral(
            &mtx.transaction_body,
            &mut utxos,
            &[(
                String::from(include_str!("../../test_data/alonzo2.collateral.address")),
                Value::Coin(5000000),
                None,
            )],
        );
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Alonzo(AlonzoProtParams {
                fee_policy: FeePolicy {
                    summand: 155381,
                    multiplier: 44,
                },
                max_tx_size: 16384,
                languages: vec![Language::PlutusV1, Language::PlutusV2],
                max_block_ex_mem: 50000000,
                max_block_ex_steps: 40000000000,
                max_tx_ex_mem: 10000000,
                max_tx_ex_steps: 10000000000,
                max_val_size: 5000,
                collateral_percent: 150,
                max_collateral_inputs: 3,
                coints_per_utxo_word: 34482,
            }),
            prot_magic: 764824073,
            block_slot: 58924928,
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
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/alonzo1.tx"));
        let mut mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(
                String::from(include_str!("../../test_data/alonzo1.address")),
                Value::Coin(1549646822),
                None,
            )],
        );
        let mut tx_body: TransactionBody = (*mtx.transaction_body).clone();
        tx_body.inputs = Vec::new();
        let mut tx_buf: Vec<u8> = Vec::new();
        match encode(tx_body, &mut tx_buf) {
            Ok(_) => (),
            Err(err) => assert!(false, "Unable to encode Tx ({:?})", err),
        };
        mtx.transaction_body =
            Decode::decode(&mut Decoder::new(&tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Alonzo);
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Alonzo(AlonzoProtParams {
                fee_policy: FeePolicy {
                    summand: 155381,
                    multiplier: 44,
                },
                max_tx_size: 16384,
                languages: vec![Language::PlutusV1, Language::PlutusV2],
                max_block_ex_mem: 50000000,
                max_block_ex_steps: 40000000000,
                max_tx_ex_mem: 10000000,
                max_tx_ex_steps: 10000000000,
                max_val_size: 5000,
                collateral_percent: 150,
                max_collateral_inputs: 3,
                coints_per_utxo_word: 34482,
            }),
            prot_magic: 764824073,
            block_slot: 44237276,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => assert!(false, "Inputs set should not be empty"),
            Err(err) => match err {
                Alonzo(TxInsEmpty) => (),
                _ => assert!(false, "Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Same as successful_mainnet_tx, but the validation is called with an empty
    // UTxO set.
    fn unfound_utxo_input() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/alonzo1.tx"));
        let mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let utxos: UTxOs = UTxOs::new();
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Alonzo);
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Alonzo(AlonzoProtParams {
                fee_policy: FeePolicy {
                    summand: 155381,
                    multiplier: 44,
                },
                max_tx_size: 16384,
                languages: vec![Language::PlutusV1, Language::PlutusV2],
                max_block_ex_mem: 50000000,
                max_block_ex_steps: 40000000000,
                max_tx_ex_mem: 10000000,
                max_tx_ex_steps: 10000000000,
                max_val_size: 5000,
                collateral_percent: 150,
                max_collateral_inputs: 3,
                coints_per_utxo_word: 34482,
            }),
            prot_magic: 764824073,
            block_slot: 44237276,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => assert!(false, "All inputs should be within the UTxO set"),
            Err(err) => match err {
                Alonzo(InputNotInUTxO) => (),
                _ => assert!(false, "Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Same as succesful_mainnet_tx, except that the lower bound of the validity
    // interval is greater than the block slot.
    fn validity_interval_lower_bound_unreached() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/alonzo1.tx"));
        let mut mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(
                String::from(include_str!("../../test_data/alonzo1.address")),
                Value::Coin(1549646822),
                None,
            )],
        );
        let mut tx_body: TransactionBody = (*mtx.transaction_body).clone();
        tx_body.validity_interval_start = Some(44237277); // One slot after the block.
        let mut tx_buf: Vec<u8> = Vec::new();
        match encode(tx_body, &mut tx_buf) {
            Ok(_) => (),
            Err(err) => assert!(false, "Unable to encode Tx ({:?})", err),
        };
        mtx.transaction_body =
            Decode::decode(&mut Decoder::new(&tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Alonzo);
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Alonzo(AlonzoProtParams {
                fee_policy: FeePolicy {
                    summand: 155381,
                    multiplier: 44,
                },
                max_tx_size: 16384,
                languages: vec![Language::PlutusV1, Language::PlutusV2],
                max_block_ex_mem: 50000000,
                max_block_ex_steps: 40000000000,
                max_tx_ex_mem: 10000000,
                max_tx_ex_steps: 10000000000,
                max_val_size: 5000,
                collateral_percent: 150,
                max_collateral_inputs: 3,
                coints_per_utxo_word: 34482,
            }),
            prot_magic: 764824073,
            block_slot: 44237276,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => assert!(
                false,
                "Validity interval lower bound should have been reached",
            ),
            Err(err) => match err {
                Alonzo(BlockPrecedesValInt) => (),
                _ => assert!(false, "Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Same as succesful_mainnet_tx, except that the upper bound of the validity
    // interval is lower than the block slot.
    fn validity_interval_upper_bound_surpassed() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/alonzo1.tx"));
        let mut mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(
                String::from(include_str!("../../test_data/alonzo1.address")),
                Value::Coin(1549646822),
                None,
            )],
        );
        let mut tx_body: TransactionBody = (*mtx.transaction_body).clone();
        tx_body.ttl = Some(6447028); // One slot before the block.
        let mut tx_buf: Vec<u8> = Vec::new();
        match encode(tx_body, &mut tx_buf) {
            Ok(_) => (),
            Err(err) => assert!(false, "Unable to encode Tx ({:?})", err),
        };
        mtx.transaction_body =
            Decode::decode(&mut Decoder::new(&tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Alonzo);
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Alonzo(AlonzoProtParams {
                fee_policy: FeePolicy {
                    summand: 155381,
                    multiplier: 44,
                },
                max_tx_size: 16384,
                languages: vec![Language::PlutusV1, Language::PlutusV2],
                max_block_ex_mem: 50000000,
                max_block_ex_steps: 40000000000,
                max_tx_ex_mem: 10000000,
                max_tx_ex_steps: 10000000000,
                max_val_size: 5000,
                collateral_percent: 150,
                max_collateral_inputs: 3,
                coints_per_utxo_word: 34482,
            }),
            prot_magic: 764824073,
            block_slot: 44237276,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => assert!(
                false,
                "Validity interval upper bound should not have been surpassed",
            ),
            Err(err) => match err {
                Alonzo(BlockExceedsValInt) => (),
                _ => assert!(false, "Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Same as succesful_mainnet_tx, except that validation is called with an
    // Environment requesting fees that exceed those paid by the transaction.
    fn min_fees_unreached() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/alonzo1.tx"));
        let mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Alonzo);
        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(
                String::from(include_str!("../../test_data/alonzo1.address")),
                Value::Coin(1549646822),
                None,
            )],
        );
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Alonzo(AlonzoProtParams {
                fee_policy: FeePolicy {
                    summand: 155381,
                    multiplier: 79, // This value was 44 during Alonzo on mainnet.
                },
                max_tx_size: 16384,
                languages: vec![Language::PlutusV1, Language::PlutusV2],
                max_block_ex_mem: 50000000,
                max_block_ex_steps: 40000000000,
                max_tx_ex_mem: 10000000,
                max_tx_ex_steps: 10000000000,
                max_val_size: 5000,
                collateral_percent: 150,
                max_collateral_inputs: 3,
                coints_per_utxo_word: 34482,
            }),
            prot_magic: 764824073,
            block_slot: 44237276,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => assert!(false, "Fee should not be below minimum"),
            Err(err) => match err {
                Alonzo(FeeBelowMin) => (),
                _ => assert!(false, "Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Same as successful_mainnet_tx_with_plutus_script, except that all collaterals
    // are removed before calling validation.
    fn no_collateral_inputs() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/alonzo2.tx"));
        let mut mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let mut utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[
                (
                    // (tx hash, tx output index):
                    // (117325a52d60be3a1e4072af39d9e630bf61ce59d315d6c1bf4c4d140f8066ea, 0)
                    String::from(include_str!("../../test_data/alonzo2.0.address")),
                    Value::Multiasset(
                        1724100,
                        KeyValuePairs::from(Vec::from([(
                            Hash::<28>::new([
                                176, 1, 7, 107, 52, 168, 126, 125, 72, 236, 70, 112, 58, 111, 80,
                                249, 50, 137, 88, 42, 217, 189, 190, 255, 127, 30, 50, 149,
                            ]),
                            KeyValuePairs::from(Vec::from([(
                                Bytes::from(hex::decode("4879706562656173747332343233").unwrap()),
                                1,
                            )])),
                        )])),
                    ),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (d2f9764fa93ae5bcabbb65c7a2f97d1e31188064ae3d2ba1462114453928dd99, 0)
                    String::from(include_str!("../../test_data/alonzo2.1.address")),
                    Value::Coin(20292207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (9fab354c2825376a943e505d13a3861e4d9ad3e177028d7bb2bbabce5453fa11, 0)
                    String::from(include_str!("../../test_data/alonzo2.2.address")),
                    Value::Coin(20292207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (3077a999b1d22cb1a4e5ee485adbde6a4596704a96384fbc9727028b8b28ba47, 0)
                    String::from(include_str!("../../test_data/alonzo2.3.address")),
                    Value::Coin(29792207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (b231aca45a38add7378d2ed7a0822626fee3396821e8791a5af5926807db962d, 0)
                    String::from(include_str!("../../test_data/alonzo2.4.address")),
                    Value::Coin(29792207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (11579a841b3c7a64aa057c9adf993ef42520570450499b0a724c7ef706b2a435, 0)
                    String::from(include_str!("../../test_data/alonzo2.5.address")),
                    Value::Coin(61233231),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (b857f98162b753d117464c499d53bbbfec5aa38b94bd624e295a7e3fddc77130, 0)
                    String::from(include_str!("../../test_data/alonzo2.6.address")),
                    Value::Coin(20292207),
                    None,
                ),
            ],
        );
        add_collateral(
            &mtx.transaction_body,
            &mut utxos,
            &[(
                String::from(include_str!("../../test_data/alonzo2.collateral.address")),
                Value::Coin(5000000),
                None,
            )],
        );
        let mut tx_body: TransactionBody = (*mtx.transaction_body).clone();
        tx_body.collateral = None;
        let mut tx_buf: Vec<u8> = Vec::new();
        match encode(tx_body, &mut tx_buf) {
            Ok(_) => (),
            Err(err) => assert!(false, "Unable to encode Tx ({:?})", err),
        };
        mtx.transaction_body =
            Decode::decode(&mut Decoder::new(&tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Alonzo);
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Alonzo(AlonzoProtParams {
                fee_policy: FeePolicy {
                    summand: 155381,
                    multiplier: 44,
                },
                max_tx_size: 16384,
                languages: vec![Language::PlutusV1, Language::PlutusV2],
                max_block_ex_mem: 50000000,
                max_block_ex_steps: 40000000000,
                max_tx_ex_mem: 10000000,
                max_tx_ex_steps: 10000000000,
                max_val_size: 5000,
                collateral_percent: 150,
                max_collateral_inputs: 3,
                coints_per_utxo_word: 34482,
            }),
            prot_magic: 764824073,
            block_slot: 58924928,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => assert!(false, "No collateral inputs"),
            Err(err) => match err {
                Alonzo(CollateralMissing) => (),
                _ => assert!(false, "Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Same as successful_mainnet_tx_with_plutus_script, except that validation is
    // called on an environment which does not allow enough collateral inputs
    // for the transaction to be valid.
    fn too_many_collateral_inputs() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/alonzo2.tx"));
        let mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Alonzo);
        let mut utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[
                (
                    // (tx hash, tx output index):
                    // (117325a52d60be3a1e4072af39d9e630bf61ce59d315d6c1bf4c4d140f8066ea, 0)
                    String::from(include_str!("../../test_data/alonzo2.0.address")),
                    Value::Multiasset(
                        1724100,
                        KeyValuePairs::from(Vec::from([(
                            Hash::<28>::new([
                                176, 1, 7, 107, 52, 168, 126, 125, 72, 236, 70, 112, 58, 111, 80,
                                249, 50, 137, 88, 42, 217, 189, 190, 255, 127, 30, 50, 149,
                            ]),
                            KeyValuePairs::from(Vec::from([(
                                Bytes::from(hex::decode("4879706562656173747332343233").unwrap()),
                                1,
                            )])),
                        )])),
                    ),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (d2f9764fa93ae5bcabbb65c7a2f97d1e31188064ae3d2ba1462114453928dd99, 0)
                    String::from(include_str!("../../test_data/alonzo2.1.address")),
                    Value::Coin(20292207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (9fab354c2825376a943e505d13a3861e4d9ad3e177028d7bb2bbabce5453fa11, 0)
                    String::from(include_str!("../../test_data/alonzo2.2.address")),
                    Value::Coin(20292207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (3077a999b1d22cb1a4e5ee485adbde6a4596704a96384fbc9727028b8b28ba47, 0)
                    String::from(include_str!("../../test_data/alonzo2.3.address")),
                    Value::Coin(29792207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (b231aca45a38add7378d2ed7a0822626fee3396821e8791a5af5926807db962d, 0)
                    String::from(include_str!("../../test_data/alonzo2.4.address")),
                    Value::Coin(29792207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (11579a841b3c7a64aa057c9adf993ef42520570450499b0a724c7ef706b2a435, 0)
                    String::from(include_str!("../../test_data/alonzo2.5.address")),
                    Value::Coin(61233231),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (b857f98162b753d117464c499d53bbbfec5aa38b94bd624e295a7e3fddc77130, 0)
                    String::from(include_str!("../../test_data/alonzo2.6.address")),
                    Value::Coin(20292207),
                    None,
                ),
            ],
        );
        add_collateral(
            &mtx.transaction_body,
            &mut utxos,
            &[(
                String::from(include_str!("../../test_data/alonzo2.collateral.address")),
                Value::Coin(5000000),
                None,
            )],
        );
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Alonzo(AlonzoProtParams {
                fee_policy: FeePolicy {
                    summand: 155381,
                    multiplier: 44,
                },
                max_tx_size: 16384,
                languages: vec![Language::PlutusV1, Language::PlutusV2],
                max_block_ex_mem: 50000000,
                max_block_ex_steps: 40000000000,
                max_tx_ex_mem: 10000000,
                max_tx_ex_steps: 10000000000,
                max_val_size: 5000,
                collateral_percent: 150,
                max_collateral_inputs: 0, // no collateral inputs are allowed
                coints_per_utxo_word: 34482,
            }),
            prot_magic: 764824073,
            block_slot: 58924928,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => assert!(false, "Number of collateral inputs should be within limits"),
            Err(err) => match err {
                Alonzo(TooManyCollaterals) => (),
                _ => assert!(false, "Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Same as successful_mainnet_tx_with_plutus_script, except that the address of
    // a collateral inputs is altered into a script-locked one.
    fn collateral_is_not_verification_key_locked() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/alonzo2.tx"));
        let mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let mut utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[
                (
                    // (tx hash, tx output index):
                    // (117325a52d60be3a1e4072af39d9e630bf61ce59d315d6c1bf4c4d140f8066ea, 0)
                    String::from(include_str!("../../test_data/alonzo2.0.address")),
                    Value::Multiasset(
                        1724100,
                        KeyValuePairs::from(Vec::from([(
                            Hash::<28>::new([
                                176, 1, 7, 107, 52, 168, 126, 125, 72, 236, 70, 112, 58, 111, 80,
                                249, 50, 137, 88, 42, 217, 189, 190, 255, 127, 30, 50, 149,
                            ]),
                            KeyValuePairs::from(Vec::from([(
                                Bytes::from(hex::decode("4879706562656173747332343233").unwrap()),
                                1,
                            )])),
                        )])),
                    ),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (d2f9764fa93ae5bcabbb65c7a2f97d1e31188064ae3d2ba1462114453928dd99, 0)
                    String::from(include_str!("../../test_data/alonzo2.1.address")),
                    Value::Coin(20292207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (9fab354c2825376a943e505d13a3861e4d9ad3e177028d7bb2bbabce5453fa11, 0)
                    String::from(include_str!("../../test_data/alonzo2.2.address")),
                    Value::Coin(20292207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (3077a999b1d22cb1a4e5ee485adbde6a4596704a96384fbc9727028b8b28ba47, 0)
                    String::from(include_str!("../../test_data/alonzo2.3.address")),
                    Value::Coin(29792207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (b231aca45a38add7378d2ed7a0822626fee3396821e8791a5af5926807db962d, 0)
                    String::from(include_str!("../../test_data/alonzo2.4.address")),
                    Value::Coin(29792207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (11579a841b3c7a64aa057c9adf993ef42520570450499b0a724c7ef706b2a435, 0)
                    String::from(include_str!("../../test_data/alonzo2.5.address")),
                    Value::Coin(61233231),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (b857f98162b753d117464c499d53bbbfec5aa38b94bd624e295a7e3fddc77130, 0)
                    String::from(include_str!("../../test_data/alonzo2.6.address")),
                    Value::Coin(20292207),
                    None,
                ),
            ],
        );
        let old_address: Address = match hex::decode(String::from(include_str!(
            "../../test_data/alonzo2.collateral.address"
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
        let multi_era_out: MultiEraOutput =
            MultiEraOutput::AlonzoCompatible(Box::new(Cow::Owned(TransactionOutput {
                address: Bytes::try_from(altered_address.to_hex()).unwrap(),
                amount: Value::Coin(5000000),
                datum_hash: None,
            })));
        utxos.insert(multi_era_in, multi_era_out);
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Alonzo);
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Alonzo(AlonzoProtParams {
                fee_policy: FeePolicy {
                    summand: 155381,
                    multiplier: 44,
                },
                max_tx_size: 16384,
                languages: vec![Language::PlutusV1, Language::PlutusV2],
                max_block_ex_mem: 50000000,
                max_block_ex_steps: 40000000000,
                max_tx_ex_mem: 10000000,
                max_tx_ex_steps: 10000000000,
                max_val_size: 5000,
                collateral_percent: 150,
                max_collateral_inputs: 3,
                coints_per_utxo_word: 34482,
            }),
            prot_magic: 764824073,
            block_slot: 58924928,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => assert!(false, "Collateral inputs should be verification-key locked"),
            Err(err) => match err {
                Alonzo(CollateralNotVKeyLocked) => (),
                _ => assert!(false, "Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Same as sucessful_mainnet_tx_with_plutus_script, except that the output
    // associated to the collateral input contains assets other than lovelace.
    fn collateral_with_other_assets() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/alonzo2.tx"));
        let mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Alonzo);
        let mut utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[
                (
                    // (tx hash, tx output index):
                    // (117325a52d60be3a1e4072af39d9e630bf61ce59d315d6c1bf4c4d140f8066ea, 0)
                    String::from(include_str!("../../test_data/alonzo2.0.address")),
                    Value::Multiasset(
                        1724100,
                        KeyValuePairs::from(Vec::from([(
                            Hash::<28>::new([
                                176, 1, 7, 107, 52, 168, 126, 125, 72, 236, 70, 112, 58, 111, 80,
                                249, 50, 137, 88, 42, 217, 189, 190, 255, 127, 30, 50, 149,
                            ]),
                            KeyValuePairs::from(Vec::from([(
                                Bytes::from(hex::decode("4879706562656173747332343233").unwrap()),
                                1,
                            )])),
                        )])),
                    ),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (d2f9764fa93ae5bcabbb65c7a2f97d1e31188064ae3d2ba1462114453928dd99, 0)
                    String::from(include_str!("../../test_data/alonzo2.1.address")),
                    Value::Coin(20292207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (9fab354c2825376a943e505d13a3861e4d9ad3e177028d7bb2bbabce5453fa11, 0)
                    String::from(include_str!("../../test_data/alonzo2.2.address")),
                    Value::Coin(20292207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (3077a999b1d22cb1a4e5ee485adbde6a4596704a96384fbc9727028b8b28ba47, 0)
                    String::from(include_str!("../../test_data/alonzo2.3.address")),
                    Value::Coin(29792207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (b231aca45a38add7378d2ed7a0822626fee3396821e8791a5af5926807db962d, 0)
                    String::from(include_str!("../../test_data/alonzo2.4.address")),
                    Value::Coin(29792207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (11579a841b3c7a64aa057c9adf993ef42520570450499b0a724c7ef706b2a435, 0)
                    String::from(include_str!("../../test_data/alonzo2.5.address")),
                    Value::Coin(61233231),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (b857f98162b753d117464c499d53bbbfec5aa38b94bd624e295a7e3fddc77130, 0)
                    String::from(include_str!("../../test_data/alonzo2.6.address")),
                    Value::Coin(20292207),
                    None,
                ),
            ],
        );
        add_collateral(
            &mtx.transaction_body,
            &mut utxos,
            &[(
                String::from(include_str!("../../test_data/alonzo2.collateral.address")),
                Value::Multiasset(
                    5000000,
                    KeyValuePairs::from(Vec::from([(
                        Hash::<28>::new([
                            176, 1, 7, 107, 52, 168, 126, 125, 72, 236, 70, 112, 58, 111, 80, 249,
                            50, 137, 88, 42, 217, 189, 190, 255, 127, 30, 50, 149,
                        ]),
                        KeyValuePairs::from(Vec::from([(
                            Bytes::from(hex::decode("4879706562656173747332343233").unwrap()),
                            1000,
                        )])),
                    )])),
                ),
                None,
            )],
        );
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Alonzo(AlonzoProtParams {
                fee_policy: FeePolicy {
                    summand: 155381,
                    multiplier: 44,
                },
                max_tx_size: 16384,
                languages: vec![Language::PlutusV1, Language::PlutusV2],
                max_block_ex_mem: 50000000,
                max_block_ex_steps: 40000000000,
                max_tx_ex_mem: 10000000,
                max_tx_ex_steps: 10000000000,
                max_val_size: 5000,
                collateral_percent: 150,
                max_collateral_inputs: 3,
                coints_per_utxo_word: 34482,
            }),
            prot_magic: 764824073,
            block_slot: 58924928,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => assert!(false, "Collateral inputs should contain only lovelace"),
            Err(err) => match err {
                Alonzo(NonLovelaceCollateral) => (),
                _ => assert!(false, "Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Same as successful_mainnet_tx_with_plutus_script, except that the lovelace in
    // the collateral input is insufficient.
    fn collateral_without_min_lovelace() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/alonzo2.tx"));
        let mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Alonzo);
        let mut utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[
                (
                    // (tx hash, tx output index):
                    // (117325a52d60be3a1e4072af39d9e630bf61ce59d315d6c1bf4c4d140f8066ea, 0)
                    String::from(include_str!("../../test_data/alonzo2.0.address")),
                    Value::Multiasset(
                        1724100,
                        KeyValuePairs::from(Vec::from([(
                            Hash::<28>::new([
                                176, 1, 7, 107, 52, 168, 126, 125, 72, 236, 70, 112, 58, 111, 80,
                                249, 50, 137, 88, 42, 217, 189, 190, 255, 127, 30, 50, 149,
                            ]),
                            KeyValuePairs::from(Vec::from([(
                                Bytes::from(hex::decode("4879706562656173747332343233").unwrap()),
                                1,
                            )])),
                        )])),
                    ),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (d2f9764fa93ae5bcabbb65c7a2f97d1e31188064ae3d2ba1462114453928dd99, 0)
                    String::from(include_str!("../../test_data/alonzo2.1.address")),
                    Value::Coin(20292207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (9fab354c2825376a943e505d13a3861e4d9ad3e177028d7bb2bbabce5453fa11, 0)
                    String::from(include_str!("../../test_data/alonzo2.2.address")),
                    Value::Coin(20292207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (3077a999b1d22cb1a4e5ee485adbde6a4596704a96384fbc9727028b8b28ba47, 0)
                    String::from(include_str!("../../test_data/alonzo2.3.address")),
                    Value::Coin(29792207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (b231aca45a38add7378d2ed7a0822626fee3396821e8791a5af5926807db962d, 0)
                    String::from(include_str!("../../test_data/alonzo2.4.address")),
                    Value::Coin(29792207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (11579a841b3c7a64aa057c9adf993ef42520570450499b0a724c7ef706b2a435, 0)
                    String::from(include_str!("../../test_data/alonzo2.5.address")),
                    Value::Coin(61233231),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (b857f98162b753d117464c499d53bbbfec5aa38b94bd624e295a7e3fddc77130, 0)
                    String::from(include_str!("../../test_data/alonzo2.6.address")),
                    Value::Coin(20292207),
                    None,
                ),
            ],
        );
        add_collateral(
            &mtx.transaction_body,
            &mut utxos,
            &[(
                String::from(include_str!("../../test_data/alonzo2.collateral.address")),
                Value::Coin(5000000),
                None,
            )],
        );
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Alonzo(AlonzoProtParams {
                fee_policy: FeePolicy {
                    summand: 155381,
                    multiplier: 44,
                },
                max_tx_size: 16384,
                languages: vec![Language::PlutusV1, Language::PlutusV2],
                max_block_ex_mem: 50000000,
                max_block_ex_steps: 40000000000,
                max_tx_ex_mem: 10000000,
                max_tx_ex_steps: 10000000000,
                max_val_size: 5000,
                collateral_percent: 700,
                max_collateral_inputs: 3,
                coints_per_utxo_word: 34482,
            }),
            prot_magic: 764824073,
            block_slot: 58924928,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => assert!(
                false,
                "Collateral inputs should contain the minimum lovelace"
            ),
            Err(err) => match err {
                Alonzo(CollateralMinLovelace) => (),
                _ => assert!(false, "Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Same as succesful_mainnet_tx, except that the fee is reduced by exactly 1,
    // and so the "preservation of value" property doesn't hold.
    fn preservation_of_value() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/alonzo1.tx"));
        let mut mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(
                String::from(include_str!("../../test_data/alonzo1.address")),
                Value::Coin(1549646822),
                None,
            )],
        );
        let mut tx_body: TransactionBody = (*mtx.transaction_body).clone();
        tx_body.fee = tx_body.fee - 1;
        let mut tx_buf: Vec<u8> = Vec::new();
        match encode(tx_body, &mut tx_buf) {
            Ok(_) => (),
            Err(err) => assert!(false, "Unable to encode Tx ({:?})", err),
        };
        mtx.transaction_body =
            Decode::decode(&mut Decoder::new(&tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Alonzo);
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Alonzo(AlonzoProtParams {
                fee_policy: FeePolicy {
                    summand: 155381,
                    multiplier: 44,
                },
                max_tx_size: 16384,
                languages: vec![Language::PlutusV1, Language::PlutusV2],
                max_block_ex_mem: 50000000,
                max_block_ex_steps: 40000000000,
                max_tx_ex_mem: 10000000,
                max_tx_ex_steps: 10000000000,
                max_val_size: 5000,
                collateral_percent: 150,
                max_collateral_inputs: 3,
                coints_per_utxo_word: 34482,
            }),
            prot_magic: 764824073,
            block_slot: 44237276,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => assert!(false, "Preservation of value doesn't hold"),
            Err(err) => match err {
                Alonzo(PreservationOfValue) => (),
                _ => assert!(false, "Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Same as successful_mainnet_tx, except that the first output's transaction
    // network ID is altered.
    fn output_network_ids() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/alonzo1.tx"));
        let mut mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let mut tx_body: TransactionBody = (*mtx.transaction_body).clone();
        let (first_output, rest): (&TransactionOutput, &[TransactionOutput]) =
            (&tx_body.outputs).split_first().unwrap();
        let addr: ShelleyAddress =
            match Address::from_bytes(&Vec::<u8>::from(first_output.address.clone())) {
                Ok(Address::Shelley(sa)) => sa,
                Ok(_) => panic!("Decoded output address and found the wrong era"),
                Err(e) => panic!("Unable to parse output address ({:?})", e),
            };
        let altered_address: ShelleyAddress = ShelleyAddress::new(
            Network::Testnet,
            addr.payment().clone(),
            addr.delegation().clone(),
        );
        let altered_output: TransactionOutput = TransactionOutput {
            address: Bytes::from(altered_address.to_vec()),
            amount: first_output.amount.clone(),
            datum_hash: first_output.datum_hash,
        };
        let mut new_outputs = Vec::from(rest);
        new_outputs.insert(0, altered_output);
        tx_body.outputs = new_outputs;
        let mut tx_buf: Vec<u8> = Vec::new();
        match encode(tx_body, &mut tx_buf) {
            Ok(_) => (),
            Err(err) => assert!(false, "Unable to encode Tx ({:?})", err),
        };
        mtx.transaction_body =
            Decode::decode(&mut Decoder::new(&tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Alonzo);
        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(
                String::from(include_str!("../../test_data/alonzo1.address")),
                Value::Coin(1549646822),
                None,
            )],
        );
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Alonzo(AlonzoProtParams {
                fee_policy: FeePolicy {
                    summand: 155381,
                    multiplier: 44,
                },
                max_tx_size: 16384,
                languages: vec![Language::PlutusV1, Language::PlutusV2],
                max_block_ex_mem: 50000000,
                max_block_ex_steps: 40000000000,
                max_tx_ex_mem: 10000000,
                max_tx_ex_steps: 10000000000,
                max_val_size: 5000,
                collateral_percent: 150,
                max_collateral_inputs: 3,
                coints_per_utxo_word: 34482,
            }),
            prot_magic: 764824073,
            block_slot: 44237276,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => assert!(
                false,
                "Transaction network ID should match environment network_id"
            ),
            Err(err) => match err {
                Alonzo(OutputWrongNetworkID) => (),
                _ => assert!(false, "Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Same as successful_mainnet_tx, except that the transaction's network ID is
    // altered.
    fn tx_network_id() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/alonzo1.tx"));
        let mut mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let mut tx_body: TransactionBody = (*mtx.transaction_body).clone();
        tx_body.network_id = Some(NetworkId::Two);
        let mut tx_buf: Vec<u8> = Vec::new();
        match encode(tx_body, &mut tx_buf) {
            Ok(_) => (),
            Err(err) => assert!(false, "Unable to encode Tx ({:?})", err),
        };
        mtx.transaction_body =
            Decode::decode(&mut Decoder::new(&tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Alonzo);
        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(
                String::from(include_str!("../../test_data/alonzo1.address")),
                Value::Coin(1549646822),
                None,
            )],
        );
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Alonzo(AlonzoProtParams {
                fee_policy: FeePolicy {
                    summand: 155381,
                    multiplier: 44,
                },
                max_tx_size: 16384,
                languages: vec![Language::PlutusV1, Language::PlutusV2],
                max_block_ex_mem: 50000000,
                max_block_ex_steps: 40000000000,
                max_tx_ex_mem: 10000000,
                max_tx_ex_steps: 10000000000,
                max_val_size: 5000,
                collateral_percent: 150,
                max_collateral_inputs: 3,
                coints_per_utxo_word: 34482,
            }),
            prot_magic: 764824073,
            block_slot: 44237276,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => assert!(
                false,
                "Transaction network ID should match environment network_id"
            ),
            Err(err) => match err {
                Alonzo(TxWrongNetworkID) => (),
                _ => assert!(false, "Unexpected error ({:?})", err),
            },
        }
    }
}
