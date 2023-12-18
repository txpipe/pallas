pub mod common;

use common::*;
use pallas_addresses::{Address, Network, ShelleyAddress};
use pallas_applying::{
    types::{
        Environment, FeePolicy, MultiEraProtParams, ShelleyMAError::*, ShelleyProtParams,
        ValidationError::*,
    },
    validate, UTxOs,
};
use pallas_codec::{
    minicbor::{
        decode::{Decode, Decoder},
        encode,
    },
    utils::Bytes,
};
use pallas_primitives::alonzo::{
    MintedTx, MintedWitnessSet, TransactionBody, TransactionOutput, VKeyWitness, Value,
};
use pallas_traverse::{Era, MultiEraTx};

#[cfg(test)]
mod shelley_ma_tests {
    use super::*;

    #[test]
    // Transaction hash:
    // 50eba65e73c8c5f7b09f4ea28cf15dce169f3d1c322ca3deff03725f51518bb2
    fn successful_mainnet_shelley_tx() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/shelley1.tx"));
        let mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Shelley);
        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(
                String::from(include_str!("../../test_data/shelley1.address")),
                Value::Coin(2332267427205),
                None,
            )],
        );
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Shelley(ShelleyProtParams {
                fee_policy: FeePolicy {
                    summand: 155381,
                    multiplier: 44,
                },
                max_tx_size: 4096,
                min_lovelace: 1000000,
            }),
            prot_magic: 764824073,
            block_slot: 5281340,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => (),
            Err(err) => assert!(false, "Unexpected error ({:?})", err),
        }
    }

    #[test]
    // Transaction hash:
    // 4a3f86762383f1d228542d383ae7ac89cf75cf7ff84dec8148558ea92b0b92d0
    fn successful_mainnet_shelley_tx_with_script() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/shelley2.tx"));
        let mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Shelley);
        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(
                String::from(include_str!("../../test_data/shelley2.address")),
                Value::Coin(2000000),
                None,
            )],
        );
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Shelley(ShelleyProtParams {
                fee_policy: FeePolicy {
                    summand: 155381,
                    multiplier: 44,
                },
                max_tx_size: 4096,
                min_lovelace: 1000000,
            }),
            prot_magic: 764824073,
            block_slot: 17584925,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => (),
            Err(err) => assert!(false, "Unexpected error ({:?})", err),
        }
    }

    #[test]
    // Transaction hash:
    // c220e20cc480df9ce7cd871df491d7390c6a004b9252cf20f45fc3c968535b4a
    fn successful_mainnet_shelley_tx_with_metadata() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/shelley3.tx"));
        let mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Shelley);
        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(
                String::from(include_str!("../../test_data/shelley3.address")),
                Value::Coin(10000000),
                None,
            )],
        );
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Shelley(ShelleyProtParams {
                fee_policy: FeePolicy {
                    summand: 155381,
                    multiplier: 44,
                },
                max_tx_size: 4096,
                min_lovelace: 1000000,
            }),
            prot_magic: 764824073,
            block_slot: 5860488,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => (),
            Err(err) => assert!(false, "Unexpected error ({:?})", err),
        }
    }

    #[test]
    // Transaction hash:
    // b7b1046d1787ac6917f5bb5841e73b3f4bef8f0a6bf692d05ef18e1db9c3f519
    fn successful_mainnet_mary_tx_with_minting() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/mary1.tx"));
        let mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Mary);
        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(
                String::from(include_str!("../../test_data/mary1.address")),
                Value::Coin(3500000),
                None,
            )],
        );
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Shelley(ShelleyProtParams {
                fee_policy: FeePolicy {
                    summand: 155381,
                    multiplier: 44,
                },
                max_tx_size: 4096,
                min_lovelace: 1000000,
            }),
            prot_magic: 764824073,
            block_slot: 24381863,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => (),
            Err(err) => assert!(false, "Unexpected error ({:?})", err),
        }
    }

    #[test]
    // All inputs are removed.
    fn empty_ins() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/shelley1.tx"));
        let mut mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(
                String::from(include_str!("../../test_data/shelley1.address")),
                Value::Coin(2332267427205),
                None,
            )],
        );
        // Clear the set of inputs in the transaction.
        let mut tx_body: TransactionBody = (*mtx.transaction_body).clone();
        tx_body.inputs = Vec::new();
        let mut tx_buf: Vec<u8> = Vec::new();
        match encode(tx_body, &mut tx_buf) {
            Ok(_) => (),
            Err(err) => assert!(false, "Unable to encode Tx ({:?})", err),
        };
        mtx.transaction_body =
            Decode::decode(&mut Decoder::new(&tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Shelley);
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Shelley(ShelleyProtParams {
                fee_policy: FeePolicy {
                    summand: 155381,
                    multiplier: 44,
                },
                max_tx_size: 4096,
                min_lovelace: 1000000,
            }),
            prot_magic: 764824073,
            block_slot: 5281340,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => assert!(false, "Inputs set should not be empty"),
            Err(err) => match err {
                ShelleyMA(TxInsEmpty) => (),
                _ => assert!(false, "Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // The UTxO set is empty.
    fn unfound_utxo() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/shelley1.tx"));
        let mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Shelley);
        let utxos: UTxOs = UTxOs::new();
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Shelley(ShelleyProtParams {
                fee_policy: FeePolicy {
                    summand: 155381,
                    multiplier: 44,
                },
                max_tx_size: 4096,
                min_lovelace: 1000000,
            }),
            prot_magic: 764824073,
            block_slot: 5281340,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => assert!(false, "All inputs must be within the UTxO set"),
            Err(err) => match err {
                ShelleyMA(InputNotInUTxO) => (),
                _ => assert!(false, "Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Time-to-live is missing.
    fn missing_ttl() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/shelley1.tx"));
        let mut mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(
                String::from(include_str!("../../test_data/shelley1.address")),
                Value::Coin(2332267427205),
                None,
            )],
        );
        let mut tx_body: TransactionBody = (*mtx.transaction_body).clone();
        tx_body.ttl = None;
        let mut tx_buf: Vec<u8> = Vec::new();
        match encode(tx_body, &mut tx_buf) {
            Ok(_) => (),
            Err(err) => assert!(false, "Unable to encode Tx ({:?})", err),
        };
        mtx.transaction_body =
            Decode::decode(&mut Decoder::new(&tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Shelley);
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Shelley(ShelleyProtParams {
                fee_policy: FeePolicy {
                    summand: 155381,
                    multiplier: 44,
                },
                max_tx_size: 4096,
                min_lovelace: 1000000,
            }),
            prot_magic: 764824073,
            block_slot: 5281340,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => assert!(false, "TTL must always be present in Shelley transactions"),
            Err(err) => match err {
                ShelleyMA(AlonzoCompNotShelley) => (),
                _ => assert!(false, "Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Transaction's time-to-live is before block slot.
    fn ttl_exceeded() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/shelley1.tx"));
        let mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Shelley);
        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(
                String::from(include_str!("../../test_data/shelley1.address")),
                Value::Coin(2332267427205),
                None,
            )],
        );
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Shelley(ShelleyProtParams {
                fee_policy: FeePolicy {
                    summand: 155381,
                    multiplier: 44,
                },
                max_tx_size: 4096,
                min_lovelace: 1000000,
            }),
            prot_magic: 764824073,
            block_slot: 9999999,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => assert!(false, "TTL cannot be exceeded"),
            Err(err) => match err {
                ShelleyMA(TTLExceeded) => (),
                _ => assert!(false, "Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Transaction size exceeds max limit (namely, 0).
    fn max_tx_size_exceeded() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/shelley1.tx"));
        let mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Shelley);
        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(
                String::from(include_str!("../../test_data/shelley1.address")),
                Value::Coin(2332267427205),
                None,
            )],
        );
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Shelley(ShelleyProtParams {
                fee_policy: FeePolicy {
                    summand: 155381,
                    multiplier: 44,
                },
                max_tx_size: 0,
                min_lovelace: 1000000,
            }),
            prot_magic: 764824073,
            block_slot: 5281340,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => assert!(false, "Tx size exceeds max limit"),
            Err(err) => match err {
                ShelleyMA(MaxTxSizeExceeded) => (),
                _ => assert!(false, "Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Min lovelace per UTxO is too high (10000000000000 lovelace against
    // 2332262258756 lovelace in transaction output).
    fn output_below_min_lovelace() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/shelley1.tx"));
        let mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Shelley);
        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(
                String::from(include_str!("../../test_data/shelley1.address")),
                Value::Coin(2332267427205),
                None,
            )],
        );
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Shelley(ShelleyProtParams {
                fee_policy: FeePolicy {
                    summand: 155381,
                    multiplier: 44,
                },
                max_tx_size: 4096,
                min_lovelace: 10000000000000,
            }),
            prot_magic: 764824073,
            block_slot: 5281340,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => assert!(false, "Output amount must be above min lovelace value"),
            Err(err) => match err {
                ShelleyMA(MinLovelaceUnreached) => (),
                _ => assert!(false, "Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // The "preservation of value" property doesn't hold - the fee is reduced by
    // exactly 1.
    fn preservation_of_value() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/shelley1.tx"));
        let mut mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let mut tx_body: TransactionBody = (*mtx.transaction_body).clone();
        tx_body.fee = tx_body.fee - 1;
        let mut tx_buf: Vec<u8> = Vec::new();
        match encode(tx_body, &mut tx_buf) {
            Ok(_) => (),
            Err(err) => assert!(false, "Unable to encode Tx ({:?})", err),
        };
        mtx.transaction_body =
            Decode::decode(&mut Decoder::new(&tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Shelley);
        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(
                String::from(include_str!("../../test_data/shelley1.address")),
                Value::Coin(2332267427205),
                None,
            )],
        );
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Shelley(ShelleyProtParams {
                fee_policy: FeePolicy {
                    summand: 155381,
                    multiplier: 44,
                },
                max_tx_size: 4096,
                min_lovelace: 1000000,
            }),
            prot_magic: 764824073,
            block_slot: 5281340,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => assert!(false, "Preservation of value property doesn't hold"),
            Err(err) => match err {
                ShelleyMA(PreservationOfValue) => (),
                _ => assert!(false, "Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Fee policy imposes higher fees on the transaction.
    fn fee_below_minimum() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/shelley1.tx"));
        let mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Shelley);
        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(
                String::from(include_str!("../../test_data/shelley1.address")),
                Value::Coin(2332267427205),
                None,
            )],
        );
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Shelley(ShelleyProtParams {
                fee_policy: FeePolicy {
                    summand: 155381,
                    multiplier: 70, // This value was 44 during Shelley on mainnet.
                },
                max_tx_size: 4096,
                min_lovelace: 1000000,
            }),
            prot_magic: 764824073,
            block_slot: 5281340,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => assert!(false, "Fee should not be below minimum"),
            Err(err) => match err {
                ShelleyMA(FeesBelowMin) => (),
                _ => assert!(false, "Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // One of the output's address network ID is changed from the mainnet value to
    // the testnet one.
    fn wrong_network_id() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/shelley1.tx"));
        let mut mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        // Modify the first output address.
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
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Shelley);
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Shelley(ShelleyProtParams {
                fee_policy: FeePolicy {
                    summand: 155381,
                    multiplier: 44,
                },
                max_tx_size: 4096,
                min_lovelace: 1000000,
            }),
            prot_magic: 764824073,
            block_slot: 5281340,
            network_id: 1,
        };
        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(
                String::from(include_str!("../../test_data/shelley1.address")),
                Value::Coin(2332267427205),
                None,
            )],
        );
        match validate(&metx, &utxos, &env) {
            Ok(()) => assert!(false, "Output with wrong network ID should be rejected"),
            Err(err) => match err {
                ShelleyMA(WrongNetworkID) => (),
                _ => assert!(false, "Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Like successful_mainnet_shelley_tx_with_metadata (hash:
    // c220e20cc480df9ce7cd871df491d7390c6a004b9252cf20f45fc3c968535b4a)
    fn auxiliary_data_removed() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/shelley3.tx"));
        let mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Shelley);
        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(
                String::from(include_str!("../../test_data/shelley3.address")),
                Value::Coin(10000000),
                None,
            )],
        );
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Shelley(ShelleyProtParams {
                fee_policy: FeePolicy {
                    summand: 155381,
                    multiplier: 44,
                },
                max_tx_size: 4096,
                min_lovelace: 1000000,
            }),
            prot_magic: 764824073,
            block_slot: 5860488,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => (),
            Err(err) => assert!(false, "Unexpected error ({:?})", err),
        }
    }

    #[test]
    // Like successful_mainnet_shelley_tx (hash:
    // 50eba65e73c8c5f7b09f4ea28cf15dce169f3d1c322ca3deff03725f51518bb2), but the
    // verification-key witness is removed.
    fn missing_vk_witness() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/shelley1.tx"));
        let mut mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        // Modify the first output address.
        let mut tx_wits: MintedWitnessSet = (*mtx.transaction_witness_set).clone();
        tx_wits.vkeywitness = Some(Vec::new());
        let mut tx_buf: Vec<u8> = Vec::new();
        match encode(tx_wits, &mut tx_buf) {
            Ok(_) => (),
            Err(err) => assert!(false, "Unable to encode Tx ({:?})", err),
        };
        mtx.transaction_witness_set =
            Decode::decode(&mut Decoder::new(&tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Shelley);
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Shelley(ShelleyProtParams {
                fee_policy: FeePolicy {
                    summand: 155381,
                    multiplier: 44,
                },
                max_tx_size: 4096,
                min_lovelace: 1000000,
            }),
            prot_magic: 764824073,
            block_slot: 5281340,
            network_id: 1,
        };
        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(
                String::from(include_str!("../../test_data/shelley1.address")),
                Value::Coin(2332267427205),
                None,
            )],
        );
        match validate(&metx, &utxos, &env) {
            Ok(()) => assert!(false, "Missing verification key witness"),
            Err(err) => match err {
                ShelleyMA(MissingVKWitness) => (),
                _ => assert!(false, "Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Like successful_mainnet_shelley_tx (hash:
    // 50eba65e73c8c5f7b09f4ea28cf15dce169f3d1c322ca3deff03725f51518bb2), but the
    // signature inside the verification-key witness is changed.
    fn vk_witness_changed() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/shelley1.tx"));
        let mut mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        // Modify the first output address.
        let mut tx_wits: MintedWitnessSet = (*mtx.transaction_witness_set).clone();
        let mut wit: VKeyWitness = tx_wits.vkeywitness.clone().unwrap().pop().unwrap();
        let mut sig_as_vec: Vec<u8> = wit.signature.to_vec();
        sig_as_vec.pop();
        sig_as_vec.push(0u8);
        wit.signature = Bytes::from(sig_as_vec);
        tx_wits.vkeywitness = Some(Vec::from([wit]));
        let mut tx_buf: Vec<u8> = Vec::new();
        match encode(tx_wits, &mut tx_buf) {
            Ok(_) => (),
            Err(err) => assert!(false, "Unable to encode Tx ({:?})", err),
        };
        mtx.transaction_witness_set =
            Decode::decode(&mut Decoder::new(&tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Shelley);
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Shelley(ShelleyProtParams {
                fee_policy: FeePolicy {
                    summand: 155381,
                    multiplier: 44,
                },
                max_tx_size: 4096,
                min_lovelace: 1000000,
            }),
            prot_magic: 764824073,
            block_slot: 5281340,
            network_id: 1,
        };
        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(
                String::from(include_str!("../../test_data/shelley1.address")),
                Value::Coin(2332267427205),
                None,
            )],
        );
        match validate(&metx, &utxos, &env) {
            Ok(()) => assert!(false, "Missing verification key witness"),
            Err(err) => match err {
                ShelleyMA(WrongSignature) => (),
                _ => assert!(false, "Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Like successful_mainnet_shelley_tx_with_script(hash:
    // 4a3f86762383f1d228542d383ae7ac89cf75cf7ff84dec8148558ea92b0b92d0), but the
    // native-script witness is removed.
    fn missing_native_script_witness() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/shelley2.tx"));
        let mut mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        // Modify the first output address.
        let mut tx_wits: MintedWitnessSet = (*mtx.transaction_witness_set).clone();
        tx_wits.native_script = Some(Vec::new());
        let mut tx_buf: Vec<u8> = Vec::new();
        match encode(tx_wits, &mut tx_buf) {
            Ok(_) => (),
            Err(err) => assert!(false, "Unable to encode Tx ({:?})", err),
        };
        mtx.transaction_witness_set =
            Decode::decode(&mut Decoder::new(&tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Shelley);
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Shelley(ShelleyProtParams {
                fee_policy: FeePolicy {
                    summand: 155381,
                    multiplier: 44,
                },
                max_tx_size: 4096,
                min_lovelace: 1000000,
            }),
            prot_magic: 764824073,
            block_slot: 5281340,
            network_id: 1,
        };
        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(
                String::from(include_str!("../../test_data/shelley2.address")),
                Value::Coin(2000000),
                None,
            )],
        );
        match validate(&metx, &utxos, &env) {
            Ok(()) => assert!(false, "Missing native script witness"),
            Err(err) => match err {
                ShelleyMA(MissingScriptWitness) => (),
                _ => assert!(false, "Unexpected error ({:?})", err),
            },
        }
    }
}
