pub mod common;

use common::*;

use pallas_addresses::{Address, Network, ShelleyAddress, ShelleyPaymentPart};
use pallas_applying::{
    utils::{AlonzoError, Environment, ValidationError::*},
    validate, UTxOs,
};
use pallas_codec::{
    minicbor::{
        decode::{Decode, Decoder},
        encode,
    },
    utils::{Bytes, KeepRaw, KeyValuePairs, Nullable},
};
use pallas_primitives::alonzo::{
    AddrKeyhash, AlonzoProtParams, CostModel, ExUnitPrices, ExUnits, Language, MintedTx,
    MintedWitnessSet, NativeScript, NetworkId, Nonce, NonceVariant, PlutusData, RationalNumber,
    Redeemer, RedeemerTag, TransactionBody, TransactionOutput, VKeyWitness, Value,
};
use pallas_traverse::{Era, MultiEraInput, MultiEraOutput, MultiEraProtocolParameters, MultiEraTx};
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
                String::from("018c9ae79bca586ac36dcfdbbf4d2826c685a6969411c338c14973cc7f7bdb37706cd03711fe64747f8cfcfd574c7445cc0378781e77a8cc00"),
                Value::Coin(1549646822),
                None,
            )],
        );
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Alonzo(mk_params_epoch_334()),
            prot_magic: 764824073,
            block_slot: 44237276,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => (),
            Err(err) => panic!("Unexpected error ({:?})", err),
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
                    String::from("714a59ebd93ea53d1bbf7f82232c7b012700a0cf4bb78d879dabb1a20a"),
                    Value::Multiasset(
                        1724100,
                        KeyValuePairs::from(Vec::from([(
                            "b001076b34a87e7d48ec46703a6f50f93289582ad9bdbeff7f1e3295"
                                .parse()
                                .unwrap(),
                            KeyValuePairs::from(Vec::from([(
                                Bytes::from(hex::decode("4879706562656173747332343233").unwrap()),
                                1,
                            )])),
                        )])),
                    ),
                    Some(
                        hex::decode(
                            "0C125EDC771B9E590D96B3C7B01CC24F906BD552CECE6D861BFA5F23281E0BBE",
                        )
                        .unwrap()
                        .as_slice()
                        .into(),
                    ),
                ),
                (
                    // (tx hash, tx output index):
                    // (d2f9764fa93ae5bcabbb65c7a2f97d1e31188064ae3d2ba1462114453928dd99, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(20292207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (9fab354c2825376a943e505d13a3861e4d9ad3e177028d7bb2bbabce5453fa11, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(20292207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (3077a999b1d22cb1a4e5ee485adbde6a4596704a96384fbc9727028b8b28ba47, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(29792207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (b231aca45a38add7378d2ed7a0822626fee3396821e8791a5af5926807db962d, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(29792207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (11579a841b3c7a64aa057c9adf993ef42520570450499b0a724c7ef706b2a435, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(61233231),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (b857f98162b753d117464c499d53bbbfec5aa38b94bd624e295a7e3fddc77130, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(20292207),
                    None,
                ),
            ],
        );
        add_collateral_alonzo(
            &mtx.transaction_body,
            &mut utxos,
            &[(
                String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                Value::Coin(5000000),
                None,
            )],
        );
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Alonzo(mk_params_epoch_300()),
            prot_magic: 764824073,
            block_slot: 58924928,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => (),
            Err(err) => panic!("Unexpected error ({:?})", err),
        }
    }

    #[test]
    // Transaction hash:
    // e55dd217f14615f91b1ac5a31ee75ef1b7397cd5ded298fa38b38e0915dd77a2
    fn successful_mainnet_tx_with_minting() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/alonzo3.tx"));
        let mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Alonzo);
        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(
                String::from("612e137a27a74aca6caff726fb9da65c371ad2d7f1cc8645648fcc11d1"),
                Value::Coin(100107582),
                None,
            )],
        );
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Alonzo(mk_params_epoch_300()),
            prot_magic: 764824073,
            block_slot: 6447035,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => (),
            Err(err) => panic!("Unexpected error ({:?})", err),
        }
    }

    #[test]
    // Transaction hash:
    // 8b6debb3340e5dac098ddb25fa647a99de12a6c1987c98b17ae074d6917dba16
    fn successful_mainnet_tx_with_metadata() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/alonzo4.tx"));
        let mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Alonzo);
        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(
                String::from("01f64b141bfa7761c00a48a137b15d433af02c9275dbf52ea95566b59cb4f05ecc9fd8c9066ef7fd907db854c76caf6462b132ce133dc7cc44"),
                Value::Coin(3224834468),
                None,
            )],
        );
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Alonzo(mk_params_epoch_300()),
            prot_magic: 764824073,
            block_slot: 6447038,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => (),
            Err(err) => panic!("Unexpected error ({:?})", err),
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
                String::from("018c9ae79bca586ac36dcfdbbf4d2826c685a6969411c338c14973cc7f7bdb37706cd03711fe64747f8cfcfd574c7445cc0378781e77a8cc00"),
                Value::Coin(1549646822),
                None,
            )],
        );
        let mut tx_body: TransactionBody = (*mtx.transaction_body).clone();
        tx_body.inputs = Vec::new();
        let mut tx_buf: Vec<u8> = Vec::new();
        let _ = encode(tx_body, &mut tx_buf);
        mtx.transaction_body =
            Decode::decode(&mut Decoder::new(tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Alonzo);
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Alonzo(mk_params_epoch_334()),
            prot_magic: 764824073,
            block_slot: 44237276,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => panic!("Inputs set should not be empty"),
            Err(err) => match err {
                Alonzo(AlonzoError::TxInsEmpty) => (),
                _ => panic!("Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Same as successful_mainnet_tx, but validation is called with an empty
    // UTxO set.
    fn unfound_utxo_input() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/alonzo1.tx"));
        let mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let utxos: UTxOs = UTxOs::new();
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Alonzo);
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Alonzo(mk_params_epoch_334()),
            prot_magic: 764824073,
            block_slot: 44237276,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => panic!("All inputs should be within the UTxO set"),
            Err(err) => match err {
                Alonzo(AlonzoError::InputNotInUTxO) => (),
                _ => panic!("Unexpected error ({:?})", err),
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
                String::from("018c9ae79bca586ac36dcfdbbf4d2826c685a6969411c338c14973cc7f7bdb37706cd03711fe64747f8cfcfd574c7445cc0378781e77a8cc00"),
                Value::Coin(1549646822),
                None,
            )],
        );
        let mut tx_body: TransactionBody = (*mtx.transaction_body).clone();
        tx_body.validity_interval_start = Some(44237277); // One slot after the block.
        let mut tx_buf: Vec<u8> = Vec::new();
        let _ = encode(tx_body, &mut tx_buf);
        mtx.transaction_body =
            Decode::decode(&mut Decoder::new(tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Alonzo);
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Alonzo(mk_params_epoch_334()),
            prot_magic: 764824073,
            block_slot: 44237276,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => panic!("Validity interval lower bound should have been reached"),
            Err(err) => match err {
                Alonzo(AlonzoError::BlockPrecedesValInt) => (),
                _ => panic!("Unexpected error ({:?})", err),
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
                String::from("018c9ae79bca586ac36dcfdbbf4d2826c685a6969411c338c14973cc7f7bdb37706cd03711fe64747f8cfcfd574c7445cc0378781e77a8cc00"),
                Value::Coin(1549646822),
                None,
            )],
        );
        let mut tx_body: TransactionBody = (*mtx.transaction_body).clone();
        tx_body.ttl = Some(6447028); // One slot before the block.
        let mut tx_buf: Vec<u8> = Vec::new();
        let _ = encode(tx_body, &mut tx_buf);
        mtx.transaction_body =
            Decode::decode(&mut Decoder::new(tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Alonzo);
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Alonzo(mk_params_epoch_334()),
            prot_magic: 764824073,
            block_slot: 44237276,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => panic!("Validity interval upper bound should not have been surpassed"),
            Err(err) => match err {
                Alonzo(AlonzoError::BlockExceedsValInt) => (),
                _ => panic!("Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Same as succesful_mainnet_tx, except that validation is called with an
    // Environment requesting fees that exceed those paid by the transaction.
    fn min_fee_unreached() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/alonzo1.tx"));
        let mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Alonzo);
        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(
                String::from("018c9ae79bca586ac36dcfdbbf4d2826c685a6969411c338c14973cc7f7bdb37706cd03711fe64747f8cfcfd574c7445cc0378781e77a8cc00"),
                Value::Coin(1549646822),
                None,
            )],
        );
        let mut alonzo_prot_params: AlonzoProtParams = mk_params_epoch_334();
        alonzo_prot_params.minfee_a = 79; // This value was 44 during Alonzo on mainnet
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Alonzo(alonzo_prot_params),
            prot_magic: 764824073,
            block_slot: 44237276,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => panic!("Fee should not be below minimum"),
            Err(err) => match err {
                Alonzo(AlonzoError::FeeBelowMin) => (),
                _ => panic!("Unexpected error ({:?})", err),
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
                    String::from("714a59ebd93ea53d1bbf7f82232c7b012700a0cf4bb78d879dabb1a20a"),
                    Value::Multiasset(
                        1724100,
                        KeyValuePairs::from(Vec::from([(
                            "b001076b34a87e7d48ec46703a6f50f93289582ad9bdbeff7f1e3295"
                                .parse()
                                .unwrap(),
                            KeyValuePairs::from(Vec::from([(
                                Bytes::from(hex::decode("4879706562656173747332343233").unwrap()),
                                1,
                            )])),
                        )])),
                    ),
                    Some(
                        hex::decode(
                            "0C125EDC771B9E590D96B3C7B01CC24F906BD552CECE6D861BFA5F23281E0BBE",
                        )
                        .unwrap()
                        .as_slice()
                        .into(),
                    ),
                ),
                (
                    // (tx hash, tx output index):
                    // (d2f9764fa93ae5bcabbb65c7a2f97d1e31188064ae3d2ba1462114453928dd99, 0)    
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(20292207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (9fab354c2825376a943e505d13a3861e4d9ad3e177028d7bb2bbabce5453fa11, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(20292207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (3077a999b1d22cb1a4e5ee485adbde6a4596704a96384fbc9727028b8b28ba47, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(29792207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (b231aca45a38add7378d2ed7a0822626fee3396821e8791a5af5926807db962d, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(29792207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (11579a841b3c7a64aa057c9adf993ef42520570450499b0a724c7ef706b2a435, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(61233231),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (b857f98162b753d117464c499d53bbbfec5aa38b94bd624e295a7e3fddc77130, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(20292207),
                    None,
                ),
            ],
        );
        add_collateral_alonzo(
            &mtx.transaction_body,
            &mut utxos,
            &[(
                String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                Value::Coin(5000000),
                None,
            )],
        );
        let mut tx_body: TransactionBody = (*mtx.transaction_body).clone();
        tx_body.collateral = None;
        let mut tx_buf: Vec<u8> = Vec::new();
        let _ = encode(tx_body, &mut tx_buf);
        mtx.transaction_body =
            Decode::decode(&mut Decoder::new(tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Alonzo);
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Alonzo(mk_params_epoch_300()),
            prot_magic: 764824073,
            block_slot: 58924928,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => panic!("No collateral inputs"),
            Err(err) => match err {
                Alonzo(AlonzoError::CollateralMissing) => (),
                _ => panic!("Unexpected error ({:?})", err),
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
                    String::from("714a59ebd93ea53d1bbf7f82232c7b012700a0cf4bb78d879dabb1a20a"),
                    Value::Multiasset(
                        1724100,
                        KeyValuePairs::from(Vec::from([(
                            "b001076b34a87e7d48ec46703a6f50f93289582ad9bdbeff7f1e3295"
                                .parse()
                                .unwrap(),
                            KeyValuePairs::from(Vec::from([(
                                Bytes::from(hex::decode("4879706562656173747332343233").unwrap()),
                                1,
                            )])),
                        )])),
                    ),
                    Some(
                        hex::decode(
                            "0C125EDC771B9E590D96B3C7B01CC24F906BD552CECE6D861BFA5F23281E0BBE",
                        )
                        .unwrap()
                        .as_slice()
                        .into(),
                    ),
                ),
                (
                    // (tx hash, tx output index):
                    // (d2f9764fa93ae5bcabbb65c7a2f97d1e31188064ae3d2ba1462114453928dd99, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(20292207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (9fab354c2825376a943e505d13a3861e4d9ad3e177028d7bb2bbabce5453fa11, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(20292207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (3077a999b1d22cb1a4e5ee485adbde6a4596704a96384fbc9727028b8b28ba47, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(29792207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (b231aca45a38add7378d2ed7a0822626fee3396821e8791a5af5926807db962d, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(29792207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (11579a841b3c7a64aa057c9adf993ef42520570450499b0a724c7ef706b2a435, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(61233231),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (b857f98162b753d117464c499d53bbbfec5aa38b94bd624e295a7e3fddc77130, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(20292207),
                    None,
                ),
            ],
        );
        add_collateral_alonzo(
            &mtx.transaction_body,
            &mut utxos,
            &[(
                String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                Value::Coin(5000000),
                None,
            )],
        );
        let mut alonzo_prot_params: AlonzoProtParams = mk_params_epoch_300();
        alonzo_prot_params.max_collateral_inputs = 0; // This value was 3 during Alonzo on mainnet
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Alonzo(alonzo_prot_params),
            prot_magic: 764824073,
            block_slot: 58924928,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => panic!("Number of collateral inputs should be within limits"),
            Err(err) => match err {
                Alonzo(AlonzoError::TooManyCollaterals) => (),
                _ => panic!("Unexpected error ({:?})", err),
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
                    String::from("714a59ebd93ea53d1bbf7f82232c7b012700a0cf4bb78d879dabb1a20a"),
                    Value::Multiasset(
                        1724100,
                        KeyValuePairs::from(Vec::from([(
                            "b001076b34a87e7d48ec46703a6f50f93289582ad9bdbeff7f1e3295"
                                .parse()
                                .unwrap(),
                            KeyValuePairs::from(Vec::from([(
                                Bytes::from(hex::decode("4879706562656173747332343233").unwrap()),
                                1,
                            )])),
                        )])),
                    ),
                    Some(
                        hex::decode(
                            "0C125EDC771B9E590D96B3C7B01CC24F906BD552CECE6D861BFA5F23281E0BBE",
                        )
                        .unwrap()
                        .as_slice()
                        .into(),
                    ),
                ),
                (
                    // (tx hash, tx output index):
                    // (d2f9764fa93ae5bcabbb65c7a2f97d1e31188064ae3d2ba1462114453928dd99, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(20292207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (9fab354c2825376a943e505d13a3861e4d9ad3e177028d7bb2bbabce5453fa11, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(20292207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (3077a999b1d22cb1a4e5ee485adbde6a4596704a96384fbc9727028b8b28ba47, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(29792207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (b231aca45a38add7378d2ed7a0822626fee3396821e8791a5af5926807db962d, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(29792207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (11579a841b3c7a64aa057c9adf993ef42520570450499b0a724c7ef706b2a435, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(61233231),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (b857f98162b753d117464c499d53bbbfec5aa38b94bd624e295a7e3fddc77130, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(20292207),
                    None,
                ),
            ],
        );
        let old_address: Address = match hex::decode(String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da")) {
            Ok(bytes_vec) => Address::from_bytes(bytes_vec.as_slice()).unwrap(),
            _ => panic!("Unable to parse collateral input address"),
        };
        let old_shelley_address: ShelleyAddress = match old_address {
            Address::Shelley(shelley_addr) => shelley_addr,
            _ => panic!("Unable to parse collateral input address"),
        };
        let altered_address: ShelleyAddress = ShelleyAddress::new(
            old_shelley_address.network(),
            ShelleyPaymentPart::Script(*old_shelley_address.payment().as_hash()),
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
            prot_params: MultiEraProtocolParameters::Alonzo(mk_params_epoch_300()),
            prot_magic: 764824073,
            block_slot: 58924928,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => panic!("Collateral inputs should be verification-key locked"),
            Err(err) => match err {
                Alonzo(AlonzoError::CollateralNotVKeyLocked) => (),
                _ => panic!("Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Same as successful_mainnet_tx_with_plutus_script, except that the output
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
                    String::from("714a59ebd93ea53d1bbf7f82232c7b012700a0cf4bb78d879dabb1a20a"),
                    Value::Multiasset(
                        1724100,
                        KeyValuePairs::from(Vec::from([(
                            "b001076b34a87e7d48ec46703a6f50f93289582ad9bdbeff7f1e3295"
                                .parse()
                                .unwrap(),
                            KeyValuePairs::from(Vec::from([(
                                Bytes::from(hex::decode("4879706562656173747332343233").unwrap()),
                                1,
                            )])),
                        )])),
                    ),
                    Some(
                        hex::decode(
                            "0C125EDC771B9E590D96B3C7B01CC24F906BD552CECE6D861BFA5F23281E0BBE",
                        )
                        .unwrap()
                        .as_slice()
                        .into(),
                    ),
                ),
                (
                    // (tx hash, tx output index):
                    // (d2f9764fa93ae5bcabbb65c7a2f97d1e31188064ae3d2ba1462114453928dd99, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(20292207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (9fab354c2825376a943e505d13a3861e4d9ad3e177028d7bb2bbabce5453fa11, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(20292207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (3077a999b1d22cb1a4e5ee485adbde6a4596704a96384fbc9727028b8b28ba47, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(29792207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (b231aca45a38add7378d2ed7a0822626fee3396821e8791a5af5926807db962d, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(29792207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (11579a841b3c7a64aa057c9adf993ef42520570450499b0a724c7ef706b2a435, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(61233231),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (b857f98162b753d117464c499d53bbbfec5aa38b94bd624e295a7e3fddc77130, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(20292207),
                    None,
                ),
            ],
        );
        add_collateral_alonzo(
            &mtx.transaction_body,
            &mut utxos,
            &[(
                String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
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
            )],
        );
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Alonzo(mk_params_epoch_300()),
            prot_magic: 764824073,
            block_slot: 58924928,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => panic!("Collateral inputs should contain only lovelace"),
            Err(err) => match err {
                Alonzo(AlonzoError::NonLovelaceCollateral) => (),
                _ => panic!("Unexpected error ({:?})", err),
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
                    String::from("714a59ebd93ea53d1bbf7f82232c7b012700a0cf4bb78d879dabb1a20a"),
                    Value::Multiasset(
                        1724100,
                        KeyValuePairs::from(Vec::from([(
                            "b001076b34a87e7d48ec46703a6f50f93289582ad9bdbeff7f1e3295"
                                .parse()
                                .unwrap(),
                            KeyValuePairs::from(Vec::from([(
                                Bytes::from(hex::decode("4879706562656173747332343233").unwrap()),
                                1,
                            )])),
                        )])),
                    ),
                    Some(
                        hex::decode(
                            "0C125EDC771B9E590D96B3C7B01CC24F906BD552CECE6D861BFA5F23281E0BBE",
                        )
                        .unwrap()
                        .as_slice()
                        .into(),
                    ),
                ),
                (
                    // (tx hash, tx output index):
                    // (d2f9764fa93ae5bcabbb65c7a2f97d1e31188064ae3d2ba1462114453928dd99, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(20292207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (9fab354c2825376a943e505d13a3861e4d9ad3e177028d7bb2bbabce5453fa11, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(20292207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (3077a999b1d22cb1a4e5ee485adbde6a4596704a96384fbc9727028b8b28ba47, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(29792207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (b231aca45a38add7378d2ed7a0822626fee3396821e8791a5af5926807db962d, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(29792207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (11579a841b3c7a64aa057c9adf993ef42520570450499b0a724c7ef706b2a435, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(61233231),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (b857f98162b753d117464c499d53bbbfec5aa38b94bd624e295a7e3fddc77130, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(20292207),
                    None,
                ),
            ],
        );
        add_collateral_alonzo(
            &mtx.transaction_body,
            &mut utxos,
            &[(
                String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                Value::Coin(5000000),
                None,
            )],
        );
        let mut alonzo_prot_params: AlonzoProtParams = mk_params_epoch_300();
        alonzo_prot_params.collateral_percentage = 700; // This was 150 during Alonzo on mainnet.
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Alonzo(alonzo_prot_params),
            prot_magic: 764824073,
            block_slot: 58924928,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => panic!("Collateral inputs should contain the minimum lovelace"),
            Err(err) => match err {
                Alonzo(AlonzoError::CollateralMinLovelace) => (),
                _ => panic!("Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Same as succesful_mainnet_tx, except that the fee is reduced by exactly 1,
    // and so the "preservation of value" property does not hold.
    fn preservation_of_value() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/alonzo1.tx"));
        let mut mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(
                String::from("018c9ae79bca586ac36dcfdbbf4d2826c685a6969411c338c14973cc7f7bdb37706cd03711fe64747f8cfcfd574c7445cc0378781e77a8cc00"),
                Value::Coin(1549646822),
                None,
            )],
        );
        let mut tx_body: TransactionBody = (*mtx.transaction_body).clone();
        tx_body.fee -= 1;
        let mut tx_buf: Vec<u8> = Vec::new();
        let _ = encode(tx_body, &mut tx_buf);
        mtx.transaction_body =
            Decode::decode(&mut Decoder::new(tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Alonzo);
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Alonzo(mk_params_epoch_334()),
            prot_magic: 764824073,
            block_slot: 44237276,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => panic!("Preservation of value does not hold"),
            Err(err) => match err {
                Alonzo(AlonzoError::PreservationOfValue) => (),
                _ => panic!("Unexpected error ({:?})", err),
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
            tx_body.outputs.split_first().unwrap();
        let addr: ShelleyAddress =
            match Address::from_bytes(&Vec::<u8>::from(first_output.address.clone())) {
                Ok(Address::Shelley(sa)) => sa,
                Ok(_) => panic!("Decoded output address and found the wrong era"),
                Err(e) => panic!("Unable to parse output address({:?})", e),
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
        let _ = encode(tx_body, &mut tx_buf);
        mtx.transaction_body =
            Decode::decode(&mut Decoder::new(tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Alonzo);
        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(
                String::from("018c9ae79bca586ac36dcfdbbf4d2826c685a6969411c338c14973cc7f7bdb37706cd03711fe64747f8cfcfd574c7445cc0378781e77a8cc00"),
                Value::Coin(1549646822),
                None,
            )],
        );
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Alonzo(mk_params_epoch_334()),
            prot_magic: 764824073,
            block_slot: 44237276,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => panic!("Output network ID should match environment network ID"),
            Err(err) => match err {
                Alonzo(AlonzoError::OutputWrongNetworkID) => (),
                _ => panic!("Unexpected error ({:?})", err),
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
        // Note that NetworkId::One maps to 0 through
        // crate::utils::get_network_id_value, which is not correct in mainnet.
        tx_body.network_id = Some(NetworkId::One);
        let mut tx_buf: Vec<u8> = Vec::new();
        let _ = encode(tx_body, &mut tx_buf);
        mtx.transaction_body =
            Decode::decode(&mut Decoder::new(tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Alonzo);
        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(
                String::from("018c9ae79bca586ac36dcfdbbf4d2826c685a6969411c338c14973cc7f7bdb37706cd03711fe64747f8cfcfd574c7445cc0378781e77a8cc00"),
                Value::Coin(1549646822),
                None,
            )],
        );
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Alonzo(mk_params_epoch_334()),
            prot_magic: 764824073,
            block_slot: 44237276,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => panic!("Transaction network ID should match environment network ID"),
            Err(err) => match err {
                Alonzo(AlonzoError::TxWrongNetworkID) => (),
                _ => panic!("Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Same as successful_mainnet_tx_with_plutus_script, except that the Environment
    // execution values are below the ones associated with the transaction.
    fn tx_ex_units_exceeded() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/alonzo2.tx"));
        let mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Alonzo);
        let mut utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[
                (
                    // (tx hash, tx output index):
                    // (117325a52d60be3a1e4072af39d9e630bf61ce59d315d6c1bf4c4d140f8066ea, 0)
                    String::from("714a59ebd93ea53d1bbf7f82232c7b012700a0cf4bb78d879dabb1a20a"),
                    Value::Multiasset(
                        1724100,
                        KeyValuePairs::from(Vec::from([(
                            "b001076b34a87e7d48ec46703a6f50f93289582ad9bdbeff7f1e3295"
                                .parse()
                                .unwrap(),
                            KeyValuePairs::from(Vec::from([(
                                Bytes::from(hex::decode("4879706562656173747332343233").unwrap()),
                                1,
                            )])),
                        )])),
                    ),
                    Some(
                        hex::decode(
                            "0C125EDC771B9E590D96B3C7B01CC24F906BD552CECE6D861BFA5F23281E0BBE",
                        )
                        .unwrap()
                        .as_slice()
                        .into(),
                    ),
                ),
                (
                    // (tx hash, tx output index):
                    // (d2f9764fa93ae5bcabbb65c7a2f97d1e31188064ae3d2ba1462114453928dd99, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(20292207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (9fab354c2825376a943e505d13a3861e4d9ad3e177028d7bb2bbabce5453fa11, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(20292207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (3077a999b1d22cb1a4e5ee485adbde6a4596704a96384fbc9727028b8b28ba47, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(29792207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (b231aca45a38add7378d2ed7a0822626fee3396821e8791a5af5926807db962d, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(29792207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (11579a841b3c7a64aa057c9adf993ef42520570450499b0a724c7ef706b2a435, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(61233231),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (b857f98162b753d117464c499d53bbbfec5aa38b94bd624e295a7e3fddc77130, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(20292207),
                    None,
                ),
            ],
        );
        add_collateral_alonzo(
            &mtx.transaction_body,
            &mut utxos,
            &[(
                String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                Value::Coin(5000000),
                None,
            )],
        );
        let mut alonzo_prot_params: AlonzoProtParams = mk_params_epoch_300();
        alonzo_prot_params.max_tx_ex_units.mem = 4649575; // This is 1 lower than that of the transaction
        alonzo_prot_params.max_tx_ex_units.steps = 1765246503; // This is 1 lower than that of the transaction
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Alonzo(alonzo_prot_params),
            prot_magic: 764824073,
            block_slot: 58924928,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => panic!("Transaction ex units should be below maximum"),
            Err(err) => match err {
                Alonzo(AlonzoError::TxExUnitsExceeded) => (),
                _ => panic!("Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Same as successful_mainnet_tx, except that the Environment with which
    // validation is called demands the transaction to be smaller than it
    // actually is.
    fn max_tx_size_exceeded() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/alonzo1.tx"));
        let mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Alonzo);
        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(
                String::from("018c9ae79bca586ac36dcfdbbf4d2826c685a6969411c338c14973cc7f7bdb37706cd03711fe64747f8cfcfd574c7445cc0378781e77a8cc00"),
                Value::Coin(1549646822),
                None,
            )],
        );
        let mut alonzo_prot_params: AlonzoProtParams = mk_params_epoch_334();
        alonzo_prot_params.max_transaction_size = 158; // 1 byte less than the size of the tx
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Alonzo(alonzo_prot_params),
            prot_magic: 764824073,
            block_slot: 44237276,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => panic!(
                "Transaction size should not exceed the maximum allowed by the protocol parameter"
            ),
            Err(err) => match err {
                Alonzo(AlonzoError::MaxTxSizeExceeded) => (),
                _ => panic!("Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Same as successful_mainnet_tx_with_plutus_script, except that the list of
    // required signers is replaced with one containing a verification key hash
    // for which there exists no matching VKeyWitness.
    fn missing_required_signer() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/alonzo2.tx"));
        let mut mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let mut utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[
                (
                    // (tx hash, tx output index):
                    // (117325a52d60be3a1e4072af39d9e630bf61ce59d315d6c1bf4c4d140f8066ea, 0)
                    String::from("714a59ebd93ea53d1bbf7f82232c7b012700a0cf4bb78d879dabb1a20a"),
                    Value::Multiasset(
                        1724100,
                        KeyValuePairs::from(Vec::from([(
                            "b001076b34a87e7d48ec46703a6f50f93289582ad9bdbeff7f1e3295"
                                .parse()
                                .unwrap(),
                            KeyValuePairs::from(Vec::from([(
                                Bytes::from(hex::decode("4879706562656173747332343233").unwrap()),
                                1,
                            )])),
                        )])),
                    ),
                    Some(
                        hex::decode(
                            "0C125EDC771B9E590D96B3C7B01CC24F906BD552CECE6D861BFA5F23281E0BBE",
                        )
                        .unwrap()
                        .as_slice()
                        .into(),
                    ),
                ),
                (
                    // (tx hash, tx output index):
                    // (d2f9764fa93ae5bcabbb65c7a2f97d1e31188064ae3d2ba1462114453928dd99, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(20292207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (9fab354c2825376a943e505d13a3861e4d9ad3e177028d7bb2bbabce5453fa11, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(20292207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (3077a999b1d22cb1a4e5ee485adbde6a4596704a96384fbc9727028b8b28ba47, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(29792207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (b231aca45a38add7378d2ed7a0822626fee3396821e8791a5af5926807db962d, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(29792207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (11579a841b3c7a64aa057c9adf993ef42520570450499b0a724c7ef706b2a435, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(61233231),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (b857f98162b753d117464c499d53bbbfec5aa38b94bd624e295a7e3fddc77130, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(20292207),
                    None,
                ),
            ],
        );
        add_collateral_alonzo(
            &mtx.transaction_body,
            &mut utxos,
            &[(
                String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                Value::Coin(5000000),
                None,
            )],
        );
        let mut tx_body: TransactionBody = (*mtx.transaction_body).clone();
        // "c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be87" is replaced with
        // "c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c8100000000"
        let req_signer: AddrKeyhash = "c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c8100000000"
            .parse()
            .unwrap();
        tx_body.required_signers = Some(vec![req_signer]);
        let mut tx_buf: Vec<u8> = Vec::new();
        let _ = encode(tx_body, &mut tx_buf);
        mtx.transaction_body =
            Decode::decode(&mut Decoder::new(tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Alonzo);
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Alonzo(mk_params_epoch_300()),
            prot_magic: 764824073,
            block_slot: 58924928,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => panic!("All required signers should have signed the transaction"),
            Err(err) => match err {
                Alonzo(AlonzoError::ReqSignerMissing) => (),
                _ => panic!("Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Same as successful_mainnet_tx, except that the list of verification key is
    // empty.
    fn missing_vk_witness() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/alonzo1.tx"));
        let mut mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(
                String::from("018c9ae79bca586ac36dcfdbbf4d2826c685a6969411c338c14973cc7f7bdb37706cd03711fe64747f8cfcfd574c7445cc0378781e77a8cc00"),
                Value::Coin(1549646822),
                None,
            )],
        );
        let mut tx_wits: MintedWitnessSet = mtx.transaction_witness_set.unwrap().clone();
        tx_wits.vkeywitness = Some(vec![]);
        let mut tx_buf: Vec<u8> = Vec::new();
        let _ = encode(tx_wits, &mut tx_buf);
        mtx.transaction_witness_set =
            Decode::decode(&mut Decoder::new(tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Alonzo);
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Alonzo(mk_params_epoch_334()),
            prot_magic: 764824073,
            block_slot: 44237276,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => panic!("Missing verification key witness"),
            Err(err) => match err {
                Alonzo(AlonzoError::VKWitnessMissing) => (),
                _ => panic!("Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Same as successful_mainnet_tx, except that the signature of the only witness
    // of the transaction is modified.
    fn wrong_signature() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/alonzo1.tx"));
        let mut mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(
                String::from("018c9ae79bca586ac36dcfdbbf4d2826c685a6969411c338c14973cc7f7bdb37706cd03711fe64747f8cfcfd574c7445cc0378781e77a8cc00"),
                Value::Coin(1549646822),
                None,
            )],
        );
        let mut tx_wits: MintedWitnessSet = mtx.transaction_witness_set.unwrap().clone();
        let mut wit: VKeyWitness = tx_wits.vkeywitness.clone().unwrap().pop().unwrap();
        // "c50047bafa1adfbfd588d7c8be89f7ab17aecd47c4cc0ed5c1318caca57c8215d77d6878f0eb2bd2620b4ea552415a3028f98102275c9a564278d0f4e6425d02"
        // is replaced with
        // "c50047bafa1adfbfd588d7c8be89f7ab17aecd47c4cc0ed5c1318caca57c8215d77d6878f0eb2bd2620b4ea552415a3028f98102275c9a564278d0f400000000"
        wit.signature = hex::decode(
            "c50047bafa1adfbfd588d7c8be89f7ab17aecd47c4cc0ed5c1318caca57c8215d77d6878f0eb2bd2620b4ea552415a3028f98102275c9a564278d0f400000000"
        ).unwrap().into();
        tx_wits.vkeywitness = Some(vec![wit]);
        let mut tx_buf: Vec<u8> = Vec::new();
        let _ = encode(tx_wits, &mut tx_buf);
        mtx.transaction_witness_set =
            Decode::decode(&mut Decoder::new(tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Alonzo);
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Alonzo(mk_params_epoch_334()),
            prot_magic: 764824073,
            block_slot: 44237276,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => panic!("Witness signature should verify the transaction"),
            Err(err) => match err {
                Alonzo(AlonzoError::VKWrongSignature) => (),
                _ => panic!("Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Same as successful_mainnet_tx_with_plutus_script, except that the list of
    // plutus scripts is empty.
    fn missing_plutus_script() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/alonzo2.tx"));
        let mut mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let mut utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[
                (
                    // (tx hash, tx output index):
                    // (117325a52d60be3a1e4072af39d9e630bf61ce59d315d6c1bf4c4d140f8066ea, 0)
                    String::from("714a59ebd93ea53d1bbf7f82232c7b012700a0cf4bb78d879dabb1a20a"),
                    Value::Multiasset(
                        1724100,
                        KeyValuePairs::from(Vec::from([(
                            "b001076b34a87e7d48ec46703a6f50f93289582ad9bdbeff7f1e3295"
                                .parse()
                                .unwrap(),
                            KeyValuePairs::from(Vec::from([(
                                Bytes::from(hex::decode("4879706562656173747332343233").unwrap()),
                                1,
                            )])),
                        )])),
                    ),
                    Some(
                        hex::decode(
                            "0C125EDC771B9E590D96B3C7B01CC24F906BD552CECE6D861BFA5F23281E0BBE",
                        )
                        .unwrap()
                        .as_slice()
                        .into(),
                    ),
                ),
                (
                    // (tx hash, tx output index):
                    // (d2f9764fa93ae5bcabbb65c7a2f97d1e31188064ae3d2ba1462114453928dd99, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(20292207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (9fab354c2825376a943e505d13a3861e4d9ad3e177028d7bb2bbabce5453fa11, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(20292207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (3077a999b1d22cb1a4e5ee485adbde6a4596704a96384fbc9727028b8b28ba47, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(29792207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (b231aca45a38add7378d2ed7a0822626fee3396821e8791a5af5926807db962d, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(29792207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (11579a841b3c7a64aa057c9adf993ef42520570450499b0a724c7ef706b2a435, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(61233231),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (b857f98162b753d117464c499d53bbbfec5aa38b94bd624e295a7e3fddc77130, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(20292207),
                    None,
                ),
            ],
        );
        add_collateral_alonzo(
            &mtx.transaction_body,
            &mut utxos,
            &[(
                String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                Value::Coin(5000000),
                None,
            )],
        );
        let mut tx_wits: MintedWitnessSet = mtx.transaction_witness_set.unwrap().clone();
        tx_wits.plutus_script = Some(Vec::new());
        let mut tx_buf: Vec<u8> = Vec::new();
        let _ = encode(tx_wits, &mut tx_buf);
        mtx.transaction_witness_set =
            Decode::decode(&mut Decoder::new(tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Alonzo);
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Alonzo(mk_params_epoch_300()),
            prot_magic: 764824073,
            block_slot: 58924928,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => panic!("Missing Plutus script"),
            Err(err) => match err {
                Alonzo(AlonzoError::ScriptWitnessMissing) => (),
                _ => panic!("Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Same as successful_mainnet_tx_with_plutus_script, except that the list of
    // plutus scripts contains an extra script.
    fn extra_plutus_script() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/alonzo2.tx"));
        let mut mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let mut utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[
                (
                    // (tx hash, tx output index):
                    // (117325a52d60be3a1e4072af39d9e630bf61ce59d315d6c1bf4c4d140f8066ea, 0)
                    String::from("714a59ebd93ea53d1bbf7f82232c7b012700a0cf4bb78d879dabb1a20a"),
                    Value::Multiasset(
                        1724100,
                        KeyValuePairs::from(Vec::from([(
                            "b001076b34a87e7d48ec46703a6f50f93289582ad9bdbeff7f1e3295"
                                .parse()
                                .unwrap(),
                            KeyValuePairs::from(Vec::from([(
                                Bytes::from(hex::decode("4879706562656173747332343233").unwrap()),
                                1,
                            )])),
                        )])),
                    ),
                    Some(
                        hex::decode(
                            "0C125EDC771B9E590D96B3C7B01CC24F906BD552CECE6D861BFA5F23281E0BBE",
                        )
                        .unwrap()
                        .as_slice()
                        .into(),
                    ),
                ),
                (
                    // (tx hash, tx output index):
                    // (d2f9764fa93ae5bcabbb65c7a2f97d1e31188064ae3d2ba1462114453928dd99, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(20292207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (9fab354c2825376a943e505d13a3861e4d9ad3e177028d7bb2bbabce5453fa11, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(20292207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (3077a999b1d22cb1a4e5ee485adbde6a4596704a96384fbc9727028b8b28ba47, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(29792207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (b231aca45a38add7378d2ed7a0822626fee3396821e8791a5af5926807db962d, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(29792207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (11579a841b3c7a64aa057c9adf993ef42520570450499b0a724c7ef706b2a435, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(61233231),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (b857f98162b753d117464c499d53bbbfec5aa38b94bd624e295a7e3fddc77130, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(20292207),
                    None,
                ),
            ],
        );
        add_collateral_alonzo(
            &mtx.transaction_body,
            &mut utxos,
            &[(
                String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                Value::Coin(5000000),
                None,
            )],
        );
        let mut tx_wits: MintedWitnessSet = mtx.transaction_witness_set.unwrap().clone();
        let native_script: NativeScript = NativeScript::InvalidBefore(0u64);
        let mut encode_native_script_buf: Vec<u8> = Vec::new();
        let _ = encode(native_script, &mut encode_native_script_buf);
        let keep_raw_native_script: KeepRaw<NativeScript> = Decode::decode(
            &mut Decoder::new(encode_native_script_buf.as_slice()),
            &mut (),
        )
        .unwrap();
        tx_wits.native_script = Some(vec![keep_raw_native_script]);
        let mut tx_buf: Vec<u8> = Vec::new();
        let _ = encode(tx_wits, &mut tx_buf);
        mtx.transaction_witness_set =
            Decode::decode(&mut Decoder::new(tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Alonzo);
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Alonzo(mk_params_epoch_300()),
            prot_magic: 764824073,
            block_slot: 58924928,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => panic!("Unneeded Plutus script"),
            Err(err) => match err {
                Alonzo(AlonzoError::UnneededNativeScript) => (),
                _ => panic!("Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Same as successful_mainnet_tx_with_minting, except that minting is not
    // supported by the corresponding native script.
    fn minting_lacks_policy() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/alonzo3.tx"));
        let mut mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(
                String::from("612e137a27a74aca6caff726fb9da65c371ad2d7f1cc8645648fcc11d1"),
                Value::Coin(100107582),
                None,
            )],
        );
        let mut tx_wits: MintedWitnessSet = mtx.transaction_witness_set.unwrap().clone();
        tx_wits.native_script = Some(Vec::new());
        let mut tx_buf: Vec<u8> = Vec::new();
        let _ = encode(tx_wits, &mut tx_buf);
        mtx.transaction_witness_set =
            Decode::decode(&mut Decoder::new(tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Alonzo);
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Alonzo(mk_params_epoch_300()),
            prot_magic: 764824073,
            block_slot: 6447035,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => panic!("Minting policy is not supported by a matching native script"),
            Err(err) => match err {
                Alonzo(AlonzoError::MintingLacksPolicy) => (),
                _ => panic!("Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Same as successful_mainnet_tx_with_plutus_script, except that the datum of
    // the input script UTxO is removed from the MintedWitnessSet.
    fn missing_input_datum() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/alonzo2.tx"));
        let mut mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let mut utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[
                (
                    // (tx hash, tx output index):
                    // (117325a52d60be3a1e4072af39d9e630bf61ce59d315d6c1bf4c4d140f8066ea, 0)
                    String::from("714a59ebd93ea53d1bbf7f82232c7b012700a0cf4bb78d879dabb1a20a"),
                    Value::Multiasset(
                        1724100,
                        KeyValuePairs::from(Vec::from([(
                            "b001076b34a87e7d48ec46703a6f50f93289582ad9bdbeff7f1e3295"
                                .parse()
                                .unwrap(),
                            KeyValuePairs::from(Vec::from([(
                                Bytes::from(hex::decode("4879706562656173747332343233").unwrap()),
                                1,
                            )])),
                        )])),
                    ),
                    Some(
                        hex::decode(
                            "0C125EDC771B9E590D96B3C7B01CC24F906BD552CECE6D861BFA5F23281E0BBE",
                        )
                        .unwrap()
                        .as_slice()
                        .into(),
                    ),
                ),
                (
                    // (tx hash, tx output index):
                    // (d2f9764fa93ae5bcabbb65c7a2f97d1e31188064ae3d2ba1462114453928dd99, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(20292207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (9fab354c2825376a943e505d13a3861e4d9ad3e177028d7bb2bbabce5453fa11, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(20292207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (3077a999b1d22cb1a4e5ee485adbde6a4596704a96384fbc9727028b8b28ba47, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(29792207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (b231aca45a38add7378d2ed7a0822626fee3396821e8791a5af5926807db962d, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(29792207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (11579a841b3c7a64aa057c9adf993ef42520570450499b0a724c7ef706b2a435, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(61233231),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (b857f98162b753d117464c499d53bbbfec5aa38b94bd624e295a7e3fddc77130, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(20292207),
                    None,
                ),
            ],
        );
        add_collateral_alonzo(
            &mtx.transaction_body,
            &mut utxos,
            &[(
                String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                Value::Coin(5000000),
                None,
            )],
        );
        let mut tx_wits: MintedWitnessSet = mtx.transaction_witness_set.unwrap().clone();
        tx_wits.plutus_data = None;
        let mut tx_buf: Vec<u8> = Vec::new();
        let _ = encode(tx_wits, &mut tx_buf);
        mtx.transaction_witness_set =
            Decode::decode(&mut Decoder::new(tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Alonzo);
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Alonzo(mk_params_epoch_300()),
            prot_magic: 764824073,
            block_slot: 58924928,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => panic!("Missing datum"),
            Err(err) => match err {
                Alonzo(AlonzoError::DatumMissing) => (),
                _ => panic!("Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Same as successful_mainnet_tx_with_plutus_script, except that the list of
    // PlutusData is extended with an unnecessary new element.
    fn extra_input_datum() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/alonzo2.tx"));
        let mut mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let mut utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[
                (
                    // (tx hash, tx output index):
                    // (117325a52d60be3a1e4072af39d9e630bf61ce59d315d6c1bf4c4d140f8066ea, 0)
                    String::from("714a59ebd93ea53d1bbf7f82232c7b012700a0cf4bb78d879dabb1a20a"),
                    Value::Multiasset(
                        1724100,
                        KeyValuePairs::from(Vec::from([(
                            "b001076b34a87e7d48ec46703a6f50f93289582ad9bdbeff7f1e3295"
                                .parse()
                                .unwrap(),
                            KeyValuePairs::from(Vec::from([(
                                Bytes::from(hex::decode("4879706562656173747332343233").unwrap()),
                                1,
                            )])),
                        )])),
                    ),
                    Some(
                        hex::decode(
                            "0C125EDC771B9E590D96B3C7B01CC24F906BD552CECE6D861BFA5F23281E0BBE",
                        )
                        .unwrap()
                        .as_slice()
                        .into(),
                    ),
                ),
                (
                    // (tx hash, tx output index):
                    // (d2f9764fa93ae5bcabbb65c7a2f97d1e31188064ae3d2ba1462114453928dd99, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(20292207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (9fab354c2825376a943e505d13a3861e4d9ad3e177028d7bb2bbabce5453fa11, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(20292207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (3077a999b1d22cb1a4e5ee485adbde6a4596704a96384fbc9727028b8b28ba47, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(29792207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (b231aca45a38add7378d2ed7a0822626fee3396821e8791a5af5926807db962d, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(29792207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (11579a841b3c7a64aa057c9adf993ef42520570450499b0a724c7ef706b2a435, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(61233231),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (b857f98162b753d117464c499d53bbbfec5aa38b94bd624e295a7e3fddc77130, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(20292207),
                    None,
                ),
            ],
        );
        add_collateral_alonzo(
            &mtx.transaction_body,
            &mut utxos,
            &[(
                String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                Value::Coin(5000000),
                None,
            )],
        );
        let mut tx_wits: MintedWitnessSet = mtx.transaction_witness_set.unwrap().clone();
        let old_datum: KeepRaw<PlutusData> = tx_wits.plutus_data.unwrap().pop().unwrap();
        let new_datum: PlutusData = PlutusData::Array(Vec::new());
        let mut new_datum_buf: Vec<u8> = Vec::new();
        let _ = encode(new_datum, &mut new_datum_buf);
        let keep_raw_new_datum: KeepRaw<PlutusData> =
            Decode::decode(&mut Decoder::new(new_datum_buf.as_slice()), &mut ()).unwrap();
        tx_wits.plutus_data = Some(vec![old_datum, keep_raw_new_datum]);
        let mut tx_buf: Vec<u8> = Vec::new();
        let _ = encode(tx_wits, &mut tx_buf);
        mtx.transaction_witness_set =
            Decode::decode(&mut Decoder::new(tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Alonzo);
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Alonzo(mk_params_epoch_300()),
            prot_magic: 764824073,
            block_slot: 58924928,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => panic!("Unneeded datum"),
            Err(err) => match err {
                Alonzo(AlonzoError::UnneededDatum) => (),
                _ => panic!("Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Same as successful_mainnet_tx_with_plutus_script, except that the list of
    // Redeemers is extended with an unnecessary new element.
    fn extra_redeemer() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/alonzo2.tx"));
        let mut mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let mut utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[
                (
                    // (tx hash, tx output index):
                    // (117325a52d60be3a1e4072af39d9e630bf61ce59d315d6c1bf4c4d140f8066ea, 0)
                    String::from("714a59ebd93ea53d1bbf7f82232c7b012700a0cf4bb78d879dabb1a20a"),
                    Value::Multiasset(
                        1724100,
                        KeyValuePairs::from(Vec::from([(
                            "b001076b34a87e7d48ec46703a6f50f93289582ad9bdbeff7f1e3295"
                                .parse()
                                .unwrap(),
                            KeyValuePairs::from(Vec::from([(
                                Bytes::from(hex::decode("4879706562656173747332343233").unwrap()),
                                1,
                            )])),
                        )])),
                    ),
                    Some(
                        hex::decode(
                            "0C125EDC771B9E590D96B3C7B01CC24F906BD552CECE6D861BFA5F23281E0BBE",
                        )
                        .unwrap()
                        .as_slice()
                        .into(),
                    ),
                ),
                (
                    // (tx hash, tx output index):
                    // (d2f9764fa93ae5bcabbb65c7a2f97d1e31188064ae3d2ba1462114453928dd99, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(20292207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (9fab354c2825376a943e505d13a3861e4d9ad3e177028d7bb2bbabce5453fa11, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(20292207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (3077a999b1d22cb1a4e5ee485adbde6a4596704a96384fbc9727028b8b28ba47, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(29792207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (b231aca45a38add7378d2ed7a0822626fee3396821e8791a5af5926807db962d, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(29792207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (11579a841b3c7a64aa057c9adf993ef42520570450499b0a724c7ef706b2a435, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(61233231),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (b857f98162b753d117464c499d53bbbfec5aa38b94bd624e295a7e3fddc77130, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(20292207),
                    None,
                ),
            ],
        );
        add_collateral_alonzo(
            &mtx.transaction_body,
            &mut utxos,
            &[(
                String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                Value::Coin(5000000),
                None,
            )],
        );
        let mut tx_wits: MintedWitnessSet = mtx.transaction_witness_set.unwrap().clone();
        let old_redeemer: Redeemer = tx_wits.redeemer.unwrap().pop().unwrap();
        let new_redeemer: Redeemer = Redeemer {
            tag: RedeemerTag::Spend,
            index: 15,
            data: PlutusData::Array(Vec::new()),
            ex_units: ExUnits { mem: 0, steps: 0 },
        };
        tx_wits.redeemer = Some(vec![old_redeemer, new_redeemer]);
        let mut tx_buf: Vec<u8> = Vec::new();
        let _ = encode(tx_wits, &mut tx_buf);
        mtx.transaction_witness_set =
            Decode::decode(&mut Decoder::new(tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Alonzo);
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Alonzo(mk_params_epoch_300()),
            prot_magic: 764824073,
            block_slot: 58924928,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => panic!("Unneeded redeemer"),
            Err(err) => match err {
                Alonzo(AlonzoError::UnneededRedeemer) => (),
                _ => panic!("Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Same as successful_mainnet_tx_with_plutus_script, except that the list of
    // Redeemers is empty.
    fn missing_redeemer() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/alonzo2.tx"));
        let mut mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let mut utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[
                (
                    // (tx hash, tx output index):
                    // (117325a52d60be3a1e4072af39d9e630bf61ce59d315d6c1bf4c4d140f8066ea, 0)
                    String::from("714a59ebd93ea53d1bbf7f82232c7b012700a0cf4bb78d879dabb1a20a"),
                    Value::Multiasset(
                        1724100,
                        KeyValuePairs::from(Vec::from([(
                            "b001076b34a87e7d48ec46703a6f50f93289582ad9bdbeff7f1e3295"
                                .parse()
                                .unwrap(),
                            KeyValuePairs::from(Vec::from([(
                                Bytes::from(hex::decode("4879706562656173747332343233").unwrap()),
                                1,
                            )])),
                        )])),
                    ),
                    Some(
                        hex::decode(
                            "0C125EDC771B9E590D96B3C7B01CC24F906BD552CECE6D861BFA5F23281E0BBE",
                        )
                        .unwrap()
                        .as_slice()
                        .into(),
                    ),
                ),
                (
                    // (tx hash, tx output index):
                    // (d2f9764fa93ae5bcabbb65c7a2f97d1e31188064ae3d2ba1462114453928dd99, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(20292207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (9fab354c2825376a943e505d13a3861e4d9ad3e177028d7bb2bbabce5453fa11, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(20292207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (3077a999b1d22cb1a4e5ee485adbde6a4596704a96384fbc9727028b8b28ba47, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(29792207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (b231aca45a38add7378d2ed7a0822626fee3396821e8791a5af5926807db962d, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(29792207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (11579a841b3c7a64aa057c9adf993ef42520570450499b0a724c7ef706b2a435, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(61233231),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (b857f98162b753d117464c499d53bbbfec5aa38b94bd624e295a7e3fddc77130, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(20292207),
                    None,
                ),
            ],
        );
        add_collateral_alonzo(
            &mtx.transaction_body,
            &mut utxos,
            &[(
                String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                Value::Coin(5000000),
                None,
            )],
        );
        let mut tx_wits: MintedWitnessSet = mtx.transaction_witness_set.unwrap().clone();
        tx_wits.redeemer = None;
        let mut tx_buf: Vec<u8> = Vec::new();
        let _ = encode(tx_wits, &mut tx_buf);
        mtx.transaction_witness_set =
            Decode::decode(&mut Decoder::new(tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Alonzo);
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Alonzo(mk_params_epoch_300()),
            prot_magic: 764824073,
            block_slot: 58924928,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => panic!("Redeemer missing"),
            Err(err) => match err {
                Alonzo(AlonzoError::RedeemerMissing) => (),
                _ => panic!("Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Same as successful_mainnet_tx_with_metadata, except that the AuxiliaryData is
    // removed.
    fn auxiliary_data_removed() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/alonzo4.tx"));
        let mut mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        mtx.auxiliary_data = Nullable::Null;
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Alonzo);
        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(
                String::from("01f64b141bfa7761c00a48a137b15d433af02c9275dbf52ea95566b59cb4f05ecc9fd8c9066ef7fd907db854c76caf6462b132ce133dc7cc44"),
                Value::Coin(3224834468),
                None,
            )],
        );
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Alonzo(mk_params_epoch_300()),
            prot_magic: 764824073,
            block_slot: 6447038,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => panic!("Transaction auxiliary data removed"),
            Err(err) => match err {
                Alonzo(AlonzoError::MetadataHash) => (),
                _ => panic!("Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Same as successful_mainnet_tx, except that the minimum lovelace in an output
    // is unreached.
    fn min_lovelace_unreached() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/alonzo1.tx"));
        let mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Alonzo);
        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(
                String::from("018c9ae79bca586ac36dcfdbbf4d2826c685a6969411c338c14973cc7f7bdb37706cd03711fe64747f8cfcfd574c7445cc0378781e77a8cc00"),
                Value::Coin(1549646822),
                None,
            )],
        );
        let mut alonzo_prot_params: AlonzoProtParams = mk_params_epoch_334();
        alonzo_prot_params.ada_per_utxo_byte = 10000000; // This was 34482 during Alonzo on mainnet.
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Alonzo(alonzo_prot_params),
            prot_magic: 764824073,
            block_slot: 44237276,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => panic!("Output minimum lovelace is unreached"),
            Err(err) => match err {
                Alonzo(AlonzoError::MinLovelaceUnreached) => (),
                _ => panic!("Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Same as successful_mainnet_tx, except that the value size exceeds the
    // environment parameter.
    fn max_val_exceeded() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/alonzo1.tx"));
        let mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Alonzo);
        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(
                String::from("018c9ae79bca586ac36dcfdbbf4d2826c685a6969411c338c14973cc7f7bdb37706cd03711fe64747f8cfcfd574c7445cc0378781e77a8cc00"),
                Value::Coin(1549646822),
                None,
            )],
        );
        let mut alonzo_prot_params: AlonzoProtParams = mk_params_epoch_334();
        alonzo_prot_params.max_value_size = 0; // This was 5000 during Alonzo on mainnet
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Alonzo(alonzo_prot_params),
            prot_magic: 764824073,
            block_slot: 44237276,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => panic!("Max value size exceeded"),
            Err(err) => match err {
                Alonzo(AlonzoError::MaxValSizeExceeded) => (),
                _ => panic!("Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Same as successful_mainnet_tx_with_plutus_script, except that the redeemers
    // list is modified in such a way that all other checks pass, but the
    // integrity hash related to script execution no longer matches the one
    // contained in the TransactionBody.
    fn script_integrity_hash() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/alonzo2.tx"));
        let mut mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let mut utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[
                (
                    // (tx hash, tx output index):
                    // (117325a52d60be3a1e4072af39d9e630bf61ce59d315d6c1bf4c4d140f8066ea, 0)
                    String::from("714a59ebd93ea53d1bbf7f82232c7b012700a0cf4bb78d879dabb1a20a"),
                    Value::Multiasset(
                        1724100,
                        KeyValuePairs::from(Vec::from([(
                            "b001076b34a87e7d48ec46703a6f50f93289582ad9bdbeff7f1e3295"
                                .parse()
                                .unwrap(),
                            KeyValuePairs::from(Vec::from([(
                                Bytes::from(hex::decode("4879706562656173747332343233").unwrap()),
                                1,
                            )])),
                        )])),
                    ),
                    Some(
                        hex::decode(
                            "0C125EDC771B9E590D96B3C7B01CC24F906BD552CECE6D861BFA5F23281E0BBE",
                        )
                        .unwrap()
                        .as_slice()
                        .into(),
                    ),
                ),
                (
                    // (tx hash, tx output index):
                    // (d2f9764fa93ae5bcabbb65c7a2f97d1e31188064ae3d2ba1462114453928dd99, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(20292207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (9fab354c2825376a943e505d13a3861e4d9ad3e177028d7bb2bbabce5453fa11, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(20292207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (3077a999b1d22cb1a4e5ee485adbde6a4596704a96384fbc9727028b8b28ba47, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(29792207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (b231aca45a38add7378d2ed7a0822626fee3396821e8791a5af5926807db962d, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(29792207),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (11579a841b3c7a64aa057c9adf993ef42520570450499b0a724c7ef706b2a435, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(61233231),
                    None,
                ),
                (
                    // (tx hash, tx output index):
                    // (b857f98162b753d117464c499d53bbbfec5aa38b94bd624e295a7e3fddc77130, 0)
                    String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                    Value::Coin(20292207),
                    None,
                ),
            ],
        );
        add_collateral_alonzo(
            &mtx.transaction_body,
            &mut utxos,
            &[(
                String::from("01c81ffcbc08ff49965d74f90c391541ff1cc2b043ffe41c81d840be8729f2ae5ed49a1734823ba37fd09923f5f7d494ae0efa23dd98ce02da"),
                Value::Coin(5000000),
                None,
            )],
        );
        let mut tx_witness_set: MintedWitnessSet = (*mtx.transaction_witness_set).clone();
        let mut redeemer: Redeemer = tx_witness_set.redeemer.unwrap().pop().unwrap();
        redeemer.ex_units = ExUnits { mem: 0, steps: 0 };
        tx_witness_set.redeemer = Some(vec![redeemer]);
        let mut tx_witness_set_buf: Vec<u8> = Vec::new();
        let _ = encode(tx_witness_set, &mut tx_witness_set_buf);
        mtx.transaction_witness_set =
            Decode::decode(&mut Decoder::new(tx_witness_set_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Alonzo);
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Alonzo(mk_params_epoch_300()),
            prot_magic: 764824073,
            block_slot: 58924928,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => panic!("Wrong script integrity hash"),
            Err(err) => match err {
                Alonzo(AlonzoError::ScriptIntegrityHash) => (),
                _ => panic!("Unexpected error ({:?})", err),
            },
        }
    }

    fn mk_params_epoch_334() -> AlonzoProtParams {
        AlonzoProtParams {
            minfee_a: 44,
            minfee_b: 155381,
            max_block_body_size: 65536,
            max_transaction_size: 16384,
            max_block_header_size: 1100,
            key_deposit: 2000000,
            pool_deposit: 500000000,
            maximum_epoch: 18,
            desired_number_of_stake_pools: 500,
            pool_pledge_influence: RationalNumber {
                numerator: 3,
                denominator: 10,
            },
            expansion_rate: RationalNumber {
                numerator: 3,
                denominator: 1000,
            },
            treasury_growth_rate: RationalNumber {
                numerator: 2,
                denominator: 10,
            },
            decentralization_constant: RationalNumber {
                numerator: 0,
                denominator: 1,
            },
            extra_entropy: Nonce {
                variant: NonceVariant::NeutralNonce,
                hash: None,
            },
            protocol_version: (6, 0),
            min_pool_cost: 340000000,
            ada_per_utxo_byte: 34482,
            cost_models_for_script_languages: KeyValuePairs::<Language, CostModel>::from(vec![(
                Language::PlutusV1,
                vec![
                    197209, 0, 1, 1, 396231, 621, 0, 1, 150000, 1000, 0, 1, 150000, 32, 2477736,
                    29175, 4, 29773, 100, 29773, 100, 29773, 100, 29773, 100, 29773, 100, 29773,
                    100, 100, 100, 29773, 100, 150000, 32, 150000, 32, 150000, 32, 150000, 1000, 0,
                    1, 150000, 32, 150000, 1000, 0, 8, 148000, 425507, 118, 0, 1, 1, 150000, 1000,
                    0, 8, 150000, 112536, 247, 1, 150000, 10000, 1, 136542, 1326, 1, 1000, 150000,
                    1000, 1, 150000, 32, 150000, 32, 150000, 32, 1, 1, 150000, 1, 150000, 4,
                    103599, 248, 1, 103599, 248, 1, 145276, 1366, 1, 179690, 497, 1, 150000, 32,
                    150000, 32, 150000, 32, 150000, 32, 150000, 32, 150000, 32, 148000, 425507,
                    118, 0, 1, 1, 61516, 11218, 0, 1, 150000, 32, 148000, 425507, 118, 0, 1, 1,
                    148000, 425507, 118, 0, 1, 1, 2477736, 29175, 4, 0, 82363, 4, 150000, 5000, 0,
                    1, 150000, 32, 197209, 0, 1, 1, 150000, 32, 150000, 32, 150000, 32, 150000, 32,
                    150000, 32, 150000, 32, 150000, 32, 3345831, 1, 1,
                ],
            )]),
            execution_costs: ExUnitPrices {
                mem_price: RationalNumber {
                    numerator: 577,
                    denominator: 10000,
                },
                step_price: RationalNumber {
                    numerator: 721,
                    denominator: 10000000,
                },
            },
            max_tx_ex_units: ExUnits {
                mem: 10000000,
                steps: 10000000000,
            },
            max_block_ex_units: ExUnits {
                mem: 50000000,
                steps: 40000000000,
            },
            max_value_size: 5000,
            collateral_percentage: 150,
            max_collateral_inputs: 3,
        }
    }

    fn mk_params_epoch_300() -> AlonzoProtParams {
        AlonzoProtParams {
            minfee_a: 44,
            minfee_b: 155381,
            max_block_body_size: 81920,
            max_transaction_size: 16384,
            max_block_header_size: 1100,
            key_deposit: 2000000,
            pool_deposit: 500000000,
            maximum_epoch: 18,
            desired_number_of_stake_pools: 500,
            pool_pledge_influence: RationalNumber {
                numerator: 3,
                denominator: 10,
            },
            expansion_rate: RationalNumber {
                numerator: 3,
                denominator: 1000,
            },
            treasury_growth_rate: RationalNumber {
                numerator: 2,
                denominator: 10,
            },
            decentralization_constant: RationalNumber {
                numerator: 0,
                denominator: 1,
            },
            extra_entropy: Nonce {
                variant: NonceVariant::NeutralNonce,
                hash: None,
            },
            protocol_version: (6, 0),
            min_pool_cost: 340000000,
            ada_per_utxo_byte: 34482,
            cost_models_for_script_languages: KeyValuePairs::<Language, CostModel>::from(vec![(
                Language::PlutusV1,
                vec![
                    197209, 0, 1, 1, 396231, 621, 0, 1, 150000, 1000, 0, 1, 150000, 32, 2477736,
                    29175, 4, 29773, 100, 29773, 100, 29773, 100, 29773, 100, 29773, 100, 29773,
                    100, 100, 100, 29773, 100, 150000, 32, 150000, 32, 150000, 32, 150000, 1000, 0,
                    1, 150000, 32, 150000, 1000, 0, 8, 148000, 425507, 118, 0, 1, 1, 150000, 1000,
                    0, 8, 150000, 112536, 247, 1, 150000, 10000, 1, 136542, 1326, 1, 1000, 150000,
                    1000, 1, 150000, 32, 150000, 32, 150000, 32, 1, 1, 150000, 1, 150000, 4,
                    103599, 248, 1, 103599, 248, 1, 145276, 1366, 1, 179690, 497, 1, 150000, 32,
                    150000, 32, 150000, 32, 150000, 32, 150000, 32, 150000, 32, 148000, 425507,
                    118, 0, 1, 1, 61516, 11218, 0, 1, 150000, 32, 148000, 425507, 118, 0, 1, 1,
                    148000, 425507, 118, 0, 1, 1, 2477736, 29175, 4, 0, 82363, 4, 150000, 5000, 0,
                    1, 150000, 32, 197209, 0, 1, 1, 150000, 32, 150000, 32, 150000, 32, 150000, 32,
                    150000, 32, 150000, 32, 150000, 32, 3345831, 1, 1,
                ],
            )]),
            execution_costs: ExUnitPrices {
                mem_price: RationalNumber {
                    numerator: 577,
                    denominator: 10000,
                },
                step_price: RationalNumber {
                    numerator: 721,
                    denominator: 10000000,
                },
            },
            max_tx_ex_units: ExUnits {
                mem: 14000000,
                steps: 10000000000,
            },
            max_block_ex_units: ExUnits {
                mem: 62000000,
                steps: 40000000000,
            },
            max_value_size: 5000,
            collateral_percentage: 150,
            max_collateral_inputs: 3,
        }
    }
}
