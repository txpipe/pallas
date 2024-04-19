pub mod common;

use common::*;
use pallas_addresses::{Address, Network, ShelleyAddress};
use pallas_applying::{
    utils::{
        Environment, MultiEraProtocolParameters, ShelleyMAError, ShelleyProtParams,
        ValidationError::*,
    },
    validate, UTxOs,
};
use pallas_codec::{
    minicbor::{
        decode::{Decode, Decoder},
        encode,
    },
    utils::{Bytes, Nullable},
};
use pallas_primitives::alonzo::{
    MintedTx, MintedWitnessSet, Nonce, NonceVariant, RationalNumber, TransactionBody,
    TransactionOutput, VKeyWitness, Value,
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
                String::from("0129bb156d52d014bb444a14138cbee36044c6faed37d0c2d49d2358315c465cbf8c5536970e8a29bb7adcda0d663b20007d481813694c64ef"),
                Value::Coin(2332267427205),
                None,
            )],
        );
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Shelley(ShelleyProtParams {
                minfee_b: 155381,
                minfee_a: 44,
                max_block_body_size: 65536,
                max_transaction_size: 4096,
                max_block_header_size: 1100,
                key_deposit: 2000000,
                pool_deposit: 500000000,
                maximum_epoch: 18,
                desired_number_of_stake_pools: 150,
                pool_pledge_influence: RationalNumber {
                    // FIX: this is a made-up value.
                    numerator: 1,
                    denominator: 1,
                },
                expansion_rate: RationalNumber {
                    // FIX: this is a made-up value.
                    numerator: 1,
                    denominator: 1,
                },
                treasury_growth_rate: RationalNumber {
                    // FIX: this is a made-up value.
                    numerator: 1,
                    denominator: 1,
                },
                decentralization_constant: RationalNumber {
                    numerator: 1,
                    denominator: 1,
                },
                extra_entropy: Nonce {
                    variant: NonceVariant::NeutralNonce,
                    hash: None,
                },
                protocol_version: (0, 2),
                min_utxo_value: 1000000,
                min_pool_cost: 340000000,
            }),
            prot_magic: 764824073,
            block_slot: 5281340,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => (),
            Err(err) => panic!("Unexpected error ({:?})", err),
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
                String::from("7165c197d565e88a20885e535f93755682444d3c02fd44dd70883fe89e"),
                Value::Coin(2000000),
                None,
            )],
        );
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Shelley(ShelleyProtParams {
                minfee_b: 155381,
                minfee_a: 44,
                max_block_body_size: 65536,
                max_transaction_size: 4096,
                max_block_header_size: 1100,
                key_deposit: 2000000,
                pool_deposit: 500000000,
                maximum_epoch: 18,
                desired_number_of_stake_pools: 150,
                pool_pledge_influence: RationalNumber {
                    // FIX: this is a made-up value.
                    numerator: 1,
                    denominator: 1,
                },
                expansion_rate: RationalNumber {
                    // FIX: this is a made-up value.
                    numerator: 1,
                    denominator: 1,
                },
                treasury_growth_rate: RationalNumber {
                    // FIX: this is a made-up value.
                    numerator: 1,
                    denominator: 1,
                },
                decentralization_constant: RationalNumber {
                    numerator: 1,
                    denominator: 1,
                },
                extra_entropy: Nonce {
                    variant: NonceVariant::NeutralNonce,
                    hash: None,
                },
                protocol_version: (0, 2),
                min_utxo_value: 1000000,
                min_pool_cost: 340000000,
            }),
            prot_magic: 764824073,
            block_slot: 17584925,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => (),
            Err(err) => panic!("Unexpected error ({:?})", err),
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
                String::from("61c96001f4a4e10567ac18be3c47663a00a858f51c56779e94993d30ef"),
                Value::Coin(10000000),
                None,
            )],
        );
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Shelley(ShelleyProtParams {
                minfee_b: 155381,
                minfee_a: 44,
                max_block_body_size: 65536,
                max_transaction_size: 4096,
                max_block_header_size: 1100,
                key_deposit: 2000000,
                pool_deposit: 500000000,
                maximum_epoch: 18,
                desired_number_of_stake_pools: 150,
                pool_pledge_influence: RationalNumber {
                    // FIX: this is a made-up value.
                    numerator: 1,
                    denominator: 1,
                },
                expansion_rate: RationalNumber {
                    // FIX: this is a made-up value.
                    numerator: 1,
                    denominator: 1,
                },
                treasury_growth_rate: RationalNumber {
                    // FIX: this is a made-up value.
                    numerator: 1,
                    denominator: 1,
                },
                decentralization_constant: RationalNumber {
                    numerator: 1,
                    denominator: 1,
                },
                extra_entropy: Nonce {
                    variant: NonceVariant::NeutralNonce,
                    hash: None,
                },
                protocol_version: (0, 2),
                min_utxo_value: 1000000,
                min_pool_cost: 340000000,
            }),
            prot_magic: 764824073,
            block_slot: 5860488,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => (),
            Err(err) => panic!("Unexpected error ({:?})", err),
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
                String::from("611489ac0c22c04abc9c6de7f95d71e1ba2c95c9b4e2f6f2900f682285"),
                Value::Coin(3500000),
                None,
            )],
        );
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Shelley(ShelleyProtParams {
                minfee_b: 155381,
                minfee_a: 44,
                max_block_body_size: 65536,
                max_transaction_size: 4096,
                max_block_header_size: 1100,
                key_deposit: 2000000,
                pool_deposit: 500000000,
                maximum_epoch: 18,
                desired_number_of_stake_pools: 150,
                pool_pledge_influence: RationalNumber {
                    // FIX: this is a made-up value.
                    numerator: 1,
                    denominator: 1,
                },
                expansion_rate: RationalNumber {
                    // FIX: this is a made-up value.
                    numerator: 1,
                    denominator: 1,
                },
                treasury_growth_rate: RationalNumber {
                    // FIX: this is a made-up value.
                    numerator: 1,
                    denominator: 1,
                },
                decentralization_constant: RationalNumber {
                    numerator: 1,
                    denominator: 1,
                },
                extra_entropy: Nonce {
                    variant: NonceVariant::NeutralNonce,
                    hash: None,
                },
                protocol_version: (0, 2),
                min_utxo_value: 1000000,
                min_pool_cost: 340000000,
            }),
            prot_magic: 764824073,
            block_slot: 24381863,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => (),
            Err(err) => panic!("Unexpected error ({:?})", err),
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
                String::from("0129bb156d52d014bb444a14138cbee36044c6faed37d0c2d49d2358315c465cbf8c5536970e8a29bb7adcda0d663b20007d481813694c64ef"),
                Value::Coin(2332267427205),
                None,
            )],
        );
        // Clear the set of inputs in the transaction.
        let mut tx_body: TransactionBody = mtx.transaction_body.unwrap().clone();
        tx_body.inputs = Vec::new();
        let mut tx_buf: Vec<u8> = Vec::new();
        match encode(tx_body, &mut tx_buf) {
            Ok(_) => (),
            Err(err) => panic!("Unable to encode Tx ({:?})", err),
        };
        mtx.transaction_body =
            Decode::decode(&mut Decoder::new(tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Shelley);
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Shelley(ShelleyProtParams {
                minfee_b: 155381,
                minfee_a: 44,
                max_block_body_size: 65536,
                max_transaction_size: 4096,
                max_block_header_size: 1100,
                key_deposit: 2000000,
                pool_deposit: 500000000,
                maximum_epoch: 18,
                desired_number_of_stake_pools: 150,
                pool_pledge_influence: RationalNumber {
                    // FIX: this is a made-up value.
                    numerator: 1,
                    denominator: 1,
                },
                expansion_rate: RationalNumber {
                    // FIX: this is a made-up value.
                    numerator: 1,
                    denominator: 1,
                },
                treasury_growth_rate: RationalNumber {
                    // FIX: this is a made-up value.
                    numerator: 1,
                    denominator: 1,
                },
                decentralization_constant: RationalNumber {
                    numerator: 1,
                    denominator: 1,
                },
                extra_entropy: Nonce {
                    variant: NonceVariant::NeutralNonce,
                    hash: None,
                },
                protocol_version: (0, 2),
                min_utxo_value: 1000000,
                min_pool_cost: 340000000,
            }),
            prot_magic: 764824073,
            block_slot: 5281340,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => panic!("Inputs set should not be empty"),
            Err(err) => match err {
                ShelleyMA(ShelleyMAError::TxInsEmpty) => (),
                _ => panic!("Unexpected error ({:?})", err),
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
            prot_params: MultiEraProtocolParameters::Shelley(ShelleyProtParams {
                minfee_b: 155381,
                minfee_a: 44,
                max_block_body_size: 65536,
                max_transaction_size: 4096,
                max_block_header_size: 1100,
                key_deposit: 2000000,
                pool_deposit: 500000000,
                maximum_epoch: 18,
                desired_number_of_stake_pools: 150,
                pool_pledge_influence: RationalNumber {
                    // FIX: this is a made-up value.
                    numerator: 1,
                    denominator: 1,
                },
                expansion_rate: RationalNumber {
                    // FIX: this is a made-up value.
                    numerator: 1,
                    denominator: 1,
                },
                treasury_growth_rate: RationalNumber {
                    // FIX: this is a made-up value.
                    numerator: 1,
                    denominator: 1,
                },
                decentralization_constant: RationalNumber {
                    numerator: 1,
                    denominator: 1,
                },
                extra_entropy: Nonce {
                    variant: NonceVariant::NeutralNonce,
                    hash: None,
                },
                protocol_version: (0, 2),
                min_utxo_value: 1000000,
                min_pool_cost: 340000000,
            }),
            prot_magic: 764824073,
            block_slot: 5281340,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => panic!("All inputs must be within the UTxO set"),
            Err(err) => match err {
                ShelleyMA(ShelleyMAError::InputNotInUTxO) => (),
                _ => panic!("Unexpected error ({:?})", err),
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
                String::from("0129bb156d52d014bb444a14138cbee36044c6faed37d0c2d49d2358315c465cbf8c5536970e8a29bb7adcda0d663b20007d481813694c64ef"),
                Value::Coin(2332267427205),
                None,
            )],
        );
        let mut tx_body: TransactionBody = mtx.transaction_body.unwrap().clone();
        tx_body.ttl = None;
        let mut tx_buf: Vec<u8> = Vec::new();
        match encode(tx_body, &mut tx_buf) {
            Ok(_) => (),
            Err(err) => panic!("Unable to encode Tx ({:?})", err),
        };
        mtx.transaction_body =
            Decode::decode(&mut Decoder::new(tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Shelley);
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Shelley(ShelleyProtParams {
                minfee_b: 155381,
                minfee_a: 44,
                max_block_body_size: 65536,
                max_transaction_size: 4096,
                max_block_header_size: 1100,
                key_deposit: 2000000,
                pool_deposit: 500000000,
                maximum_epoch: 18,
                desired_number_of_stake_pools: 150,
                pool_pledge_influence: RationalNumber {
                    // FIX: this is a made-up value.
                    numerator: 1,
                    denominator: 1,
                },
                expansion_rate: RationalNumber {
                    // FIX: this is a made-up value.
                    numerator: 1,
                    denominator: 1,
                },
                treasury_growth_rate: RationalNumber {
                    // FIX: this is a made-up value.
                    numerator: 1,
                    denominator: 1,
                },
                decentralization_constant: RationalNumber {
                    numerator: 1,
                    denominator: 1,
                },
                extra_entropy: Nonce {
                    variant: NonceVariant::NeutralNonce,
                    hash: None,
                },
                protocol_version: (0, 2),
                min_utxo_value: 1000000,
                min_pool_cost: 340000000,
            }),
            prot_magic: 764824073,
            block_slot: 5281340,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => panic!("TTL must always be present in Shelley transactions"),
            Err(err) => match err {
                ShelleyMA(ShelleyMAError::AlonzoCompNotShelley) => (),
                _ => panic!("Unexpected error ({:?})", err),
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
                String::from("0129bb156d52d014bb444a14138cbee36044c6faed37d0c2d49d2358315c465cbf8c5536970e8a29bb7adcda0d663b20007d481813694c64ef"),
                Value::Coin(2332267427205),
                None,
            )],
        );
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Shelley(ShelleyProtParams {
                minfee_b: 155381,
                minfee_a: 44,
                max_block_body_size: 65536,
                max_transaction_size: 4096,
                max_block_header_size: 1100,
                key_deposit: 2000000,
                pool_deposit: 500000000,
                maximum_epoch: 18,
                desired_number_of_stake_pools: 150,
                pool_pledge_influence: RationalNumber {
                    // FIX: this is a made-up value.
                    numerator: 1,
                    denominator: 1,
                },
                expansion_rate: RationalNumber {
                    // FIX: this is a made-up value.
                    numerator: 1,
                    denominator: 1,
                },
                treasury_growth_rate: RationalNumber {
                    // FIX: this is a made-up value.
                    numerator: 1,
                    denominator: 1,
                },
                decentralization_constant: RationalNumber {
                    numerator: 1,
                    denominator: 1,
                },
                extra_entropy: Nonce {
                    variant: NonceVariant::NeutralNonce,
                    hash: None,
                },
                protocol_version: (0, 2),
                min_utxo_value: 1000000,
                min_pool_cost: 340000000,
            }),
            prot_magic: 764824073,
            block_slot: 9999999,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => panic!("TTL cannot be exceeded"),
            Err(err) => match err {
                ShelleyMA(ShelleyMAError::TTLExceeded) => (),
                _ => panic!("Unexpected error ({:?})", err),
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
                String::from("0129bb156d52d014bb444a14138cbee36044c6faed37d0c2d49d2358315c465cbf8c5536970e8a29bb7adcda0d663b20007d481813694c64ef"),
                Value::Coin(2332267427205),
                None,
            )],
        );
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Shelley(ShelleyProtParams {
                minfee_b: 155381,
                minfee_a: 44,
                max_block_body_size: 65536,
                max_transaction_size: 0,
                max_block_header_size: 1100,
                key_deposit: 2000000,
                pool_deposit: 500000000,
                maximum_epoch: 18,
                desired_number_of_stake_pools: 150,
                pool_pledge_influence: RationalNumber {
                    // FIX: this is a made-up value.
                    numerator: 1,
                    denominator: 1,
                },
                expansion_rate: RationalNumber {
                    // FIX: this is a made-up value.
                    numerator: 1,
                    denominator: 1,
                },
                treasury_growth_rate: RationalNumber {
                    // FIX: this is a made-up value.
                    numerator: 1,
                    denominator: 1,
                },
                decentralization_constant: RationalNumber {
                    numerator: 1,
                    denominator: 1,
                },
                extra_entropy: Nonce {
                    variant: NonceVariant::NeutralNonce,
                    hash: None,
                },
                protocol_version: (0, 2),
                min_utxo_value: 1000000,
                min_pool_cost: 340000000,
            }),
            prot_magic: 764824073,
            block_slot: 5281340,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => panic!("Tx size exceeds max limit"),
            Err(err) => match err {
                ShelleyMA(ShelleyMAError::MaxTxSizeExceeded) => (),
                _ => panic!("Unexpected error ({:?})", err),
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
                String::from("0129bb156d52d014bb444a14138cbee36044c6faed37d0c2d49d2358315c465cbf8c5536970e8a29bb7adcda0d663b20007d481813694c64ef"),
                Value::Coin(2332267427205),
                None,
            )],
        );
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Shelley(ShelleyProtParams {
                minfee_b: 155381,
                minfee_a: 44,
                max_block_body_size: 65536,
                max_transaction_size: 4096,
                max_block_header_size: 1100,
                key_deposit: 2000000,
                pool_deposit: 500000000,
                maximum_epoch: 18,
                desired_number_of_stake_pools: 150,
                pool_pledge_influence: RationalNumber {
                    // FIX: this is a made-up value.
                    numerator: 1,
                    denominator: 1,
                },
                expansion_rate: RationalNumber {
                    // FIX: this is a made-up value.
                    numerator: 1,
                    denominator: 1,
                },
                treasury_growth_rate: RationalNumber {
                    // FIX: this is a made-up value.
                    numerator: 1,
                    denominator: 1,
                },
                decentralization_constant: RationalNumber {
                    numerator: 1,
                    denominator: 1,
                },
                extra_entropy: Nonce {
                    variant: NonceVariant::NeutralNonce,
                    hash: None,
                },
                protocol_version: (0, 2),
                min_utxo_value: 10000000000000,
                min_pool_cost: 340000000,
            }),
            prot_magic: 764824073,
            block_slot: 5281340,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => panic!("Output amount must be above min lovelace value"),
            Err(err) => match err {
                ShelleyMA(ShelleyMAError::MinLovelaceUnreached) => (),
                _ => panic!("Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // The "preservation of value" property doesn't hold - the fee is reduced by
    // exactly 1.
    fn preservation_of_value() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/shelley1.tx"));
        let mut mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let mut tx_body: TransactionBody = mtx.transaction_body.unwrap().clone();
        tx_body.fee -= 1;
        let mut tx_buf: Vec<u8> = Vec::new();
        match encode(tx_body, &mut tx_buf) {
            Ok(_) => (),
            Err(err) => panic!("Unable to encode Tx ({:?})", err),
        };
        mtx.transaction_body =
            Decode::decode(&mut Decoder::new(tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Shelley);
        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(
                String::from("0129bb156d52d014bb444a14138cbee36044c6faed37d0c2d49d2358315c465cbf8c5536970e8a29bb7adcda0d663b20007d481813694c64ef"),
                Value::Coin(2332267427205),
                None,
            )],
        );
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Shelley(ShelleyProtParams {
                minfee_b: 155381,
                minfee_a: 44,
                max_block_body_size: 65536,
                max_transaction_size: 4096,
                max_block_header_size: 1100,
                key_deposit: 2000000,
                pool_deposit: 500000000,
                maximum_epoch: 18,
                desired_number_of_stake_pools: 150,
                pool_pledge_influence: RationalNumber {
                    // FIX: this is a made-up value.
                    numerator: 1,
                    denominator: 1,
                },
                expansion_rate: RationalNumber {
                    // FIX: this is a made-up value.
                    numerator: 1,
                    denominator: 1,
                },
                treasury_growth_rate: RationalNumber {
                    // FIX: this is a made-up value.
                    numerator: 1,
                    denominator: 1,
                },
                decentralization_constant: RationalNumber {
                    numerator: 1,
                    denominator: 1,
                },
                extra_entropy: Nonce {
                    variant: NonceVariant::NeutralNonce,
                    hash: None,
                },
                protocol_version: (0, 2),
                min_utxo_value: 1000000,
                min_pool_cost: 340000000,
            }),
            prot_magic: 764824073,
            block_slot: 5281340,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => panic!("Preservation of value property doesn't hold"),
            Err(err) => match err {
                ShelleyMA(ShelleyMAError::PreservationOfValue) => (),
                _ => panic!("Unexpected error ({:?})", err),
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
                String::from("0129bb156d52d014bb444a14138cbee36044c6faed37d0c2d49d2358315c465cbf8c5536970e8a29bb7adcda0d663b20007d481813694c64ef"),
                Value::Coin(2332267427205),
                None,
            )],
        );
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Shelley(ShelleyProtParams {
                minfee_b: 155381,
                minfee_a: 70,
                max_block_body_size: 65536,
                max_transaction_size: 4096,
                max_block_header_size: 1100,
                key_deposit: 2000000,
                pool_deposit: 500000000,
                maximum_epoch: 18,
                desired_number_of_stake_pools: 150,
                pool_pledge_influence: RationalNumber {
                    // FIX: this is a made-up value.
                    numerator: 1,
                    denominator: 1,
                },
                expansion_rate: RationalNumber {
                    // FIX: this is a made-up value.
                    numerator: 1,
                    denominator: 1,
                },
                treasury_growth_rate: RationalNumber {
                    // FIX: this is a made-up value.
                    numerator: 1,
                    denominator: 1,
                },
                decentralization_constant: RationalNumber {
                    numerator: 1,
                    denominator: 1,
                },
                extra_entropy: Nonce {
                    variant: NonceVariant::NeutralNonce,
                    hash: None,
                },
                protocol_version: (0, 2),
                min_utxo_value: 1000000,
                min_pool_cost: 340000000,
            }),
            prot_magic: 764824073,
            block_slot: 5281340,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => panic!("Fee should not be below minimum"),
            Err(err) => match err {
                ShelleyMA(ShelleyMAError::FeesBelowMin) => (),
                _ => panic!("Unexpected error ({:?})", err),
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
        let mut tx_body: TransactionBody = mtx.transaction_body.unwrap().clone();
        let (first_output, rest): (&TransactionOutput, &[TransactionOutput]) =
            (tx_body.outputs).split_first().unwrap();
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
            Err(err) => panic!("Unable to encode Tx ({:?})", err),
        };
        mtx.transaction_body =
            Decode::decode(&mut Decoder::new(tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Shelley);
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Shelley(ShelleyProtParams {
                minfee_b: 155381,
                minfee_a: 44,
                max_block_body_size: 65536,
                max_transaction_size: 4096,
                max_block_header_size: 1100,
                key_deposit: 2000000,
                pool_deposit: 500000000,
                maximum_epoch: 18,
                desired_number_of_stake_pools: 150,
                pool_pledge_influence: RationalNumber {
                    // FIX: this is a made-up value.
                    numerator: 1,
                    denominator: 1,
                },
                expansion_rate: RationalNumber {
                    // FIX: this is a made-up value.
                    numerator: 1,
                    denominator: 1,
                },
                treasury_growth_rate: RationalNumber {
                    // FIX: this is a made-up value.
                    numerator: 1,
                    denominator: 1,
                },
                decentralization_constant: RationalNumber {
                    numerator: 1,
                    denominator: 1,
                },
                extra_entropy: Nonce {
                    variant: NonceVariant::NeutralNonce,
                    hash: None,
                },
                protocol_version: (0, 2),
                min_utxo_value: 1000000,
                min_pool_cost: 340000000,
            }),
            prot_magic: 764824073,
            block_slot: 5281340,
            network_id: 1,
        };
        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(
                String::from("0129bb156d52d014bb444a14138cbee36044c6faed37d0c2d49d2358315c465cbf8c5536970e8a29bb7adcda0d663b20007d481813694c64ef"),
                Value::Coin(2332267427205),
                None,
            )],
        );
        match validate(&metx, &utxos, &env) {
            Ok(()) => panic!("Output with wrong network ID should be rejected"),
            Err(err) => match err {
                ShelleyMA(ShelleyMAError::WrongNetworkID) => (),
                _ => panic!("Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Same as successful_mainnet_shelley_tx_with_metadata (hash:
    // c220e20cc480df9ce7cd871df491d7390c6a004b9252cf20f45fc3c968535b4a), except
    // that the AuxiliaryData is removed.
    fn auxiliary_data_removed() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/shelley3.tx"));
        let mut mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        mtx.auxiliary_data = Nullable::Null;
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Shelley);
        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(
                String::from("61c96001f4a4e10567ac18be3c47663a00a858f51c56779e94993d30ef"),
                Value::Coin(10000000),
                None,
            )],
        );
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Shelley(ShelleyProtParams {
                minfee_b: 155381,
                minfee_a: 44,
                max_block_body_size: 65536,
                max_transaction_size: 4096,
                max_block_header_size: 1100,
                key_deposit: 2000000,
                pool_deposit: 500000000,
                maximum_epoch: 18,
                desired_number_of_stake_pools: 150,
                pool_pledge_influence: RationalNumber {
                    // FIX: this is a made-up value.
                    numerator: 1,
                    denominator: 1,
                },
                expansion_rate: RationalNumber {
                    // FIX: this is a made-up value.
                    numerator: 1,
                    denominator: 1,
                },
                treasury_growth_rate: RationalNumber {
                    // FIX: this is a made-up value.
                    numerator: 1,
                    denominator: 1,
                },
                decentralization_constant: RationalNumber {
                    numerator: 1,
                    denominator: 1,
                },
                extra_entropy: Nonce {
                    variant: NonceVariant::NeutralNonce,
                    hash: None,
                },
                protocol_version: (0, 2),
                min_utxo_value: 1000000,
                min_pool_cost: 340000000,
            }),
            prot_magic: 764824073,
            block_slot: 5860488,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => panic!("Output with wrong network ID should be rejected"),
            Err(err) => match err {
                ShelleyMA(ShelleyMAError::MetadataHash) => (),
                _ => panic!("Unexpected error ({:?})", err),
            },
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
        let mut tx_wits: MintedWitnessSet = mtx.transaction_witness_set.unwrap().clone();
        tx_wits.vkeywitness = Some(Vec::new());
        let mut tx_buf: Vec<u8> = Vec::new();
        match encode(tx_wits, &mut tx_buf) {
            Ok(_) => (),
            Err(err) => panic!("Unable to encode Tx ({:?})", err),
        };
        mtx.transaction_witness_set =
            Decode::decode(&mut Decoder::new(tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Shelley);
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Shelley(ShelleyProtParams {
                minfee_b: 155381,
                minfee_a: 44,
                max_block_body_size: 65536,
                max_transaction_size: 4096,
                max_block_header_size: 1100,
                key_deposit: 2000000,
                pool_deposit: 500000000,
                maximum_epoch: 18,
                desired_number_of_stake_pools: 150,
                pool_pledge_influence: RationalNumber {
                    // FIX: this is a made-up value.
                    numerator: 1,
                    denominator: 1,
                },
                expansion_rate: RationalNumber {
                    // FIX: this is a made-up value.
                    numerator: 1,
                    denominator: 1,
                },
                treasury_growth_rate: RationalNumber {
                    // FIX: this is a made-up value.
                    numerator: 1,
                    denominator: 1,
                },
                decentralization_constant: RationalNumber {
                    numerator: 1,
                    denominator: 1,
                },
                extra_entropy: Nonce {
                    variant: NonceVariant::NeutralNonce,
                    hash: None,
                },
                protocol_version: (0, 2),
                min_utxo_value: 1000000,
                min_pool_cost: 340000000,
            }),
            prot_magic: 764824073,
            block_slot: 5281340,
            network_id: 1,
        };
        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(
                String::from("0129bb156d52d014bb444a14138cbee36044c6faed37d0c2d49d2358315c465cbf8c5536970e8a29bb7adcda0d663b20007d481813694c64ef"),
                Value::Coin(2332267427205),
                None,
            )],
        );
        match validate(&metx, &utxos, &env) {
            Ok(()) => panic!("Missing verification key witness"),
            Err(err) => match err {
                ShelleyMA(ShelleyMAError::MissingVKWitness) => (),
                _ => panic!("Unexpected error ({:?})", err),
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
        let mut tx_wits: MintedWitnessSet = mtx.transaction_witness_set.unwrap().clone();
        let mut wit: VKeyWitness = tx_wits.vkeywitness.clone().unwrap().pop().unwrap();
        let mut sig_as_vec: Vec<u8> = wit.signature.to_vec();
        sig_as_vec.pop();
        sig_as_vec.push(0u8);
        wit.signature = Bytes::from(sig_as_vec);
        tx_wits.vkeywitness = Some(Vec::from([wit]));
        let mut tx_buf: Vec<u8> = Vec::new();
        match encode(tx_wits, &mut tx_buf) {
            Ok(_) => (),
            Err(err) => panic!("Unable to encode Tx ({:?})", err),
        };
        mtx.transaction_witness_set =
            Decode::decode(&mut Decoder::new(tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Shelley);
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Shelley(ShelleyProtParams {
                minfee_b: 155381,
                minfee_a: 44,
                max_block_body_size: 65536,
                max_transaction_size: 4096,
                max_block_header_size: 1100,
                key_deposit: 2000000,
                pool_deposit: 500000000,
                maximum_epoch: 18,
                desired_number_of_stake_pools: 150,
                pool_pledge_influence: RationalNumber {
                    // FIX: this is a made-up value.
                    numerator: 1,
                    denominator: 1,
                },
                expansion_rate: RationalNumber {
                    // FIX: this is a made-up value.
                    numerator: 1,
                    denominator: 1,
                },
                treasury_growth_rate: RationalNumber {
                    // FIX: this is a made-up value.
                    numerator: 1,
                    denominator: 1,
                },
                decentralization_constant: RationalNumber {
                    numerator: 1,
                    denominator: 1,
                },
                extra_entropy: Nonce {
                    variant: NonceVariant::NeutralNonce,
                    hash: None,
                },
                protocol_version: (0, 2),
                min_utxo_value: 1000000,
                min_pool_cost: 340000000,
            }),
            prot_magic: 764824073,
            block_slot: 5281340,
            network_id: 1,
        };
        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(
                String::from("0129bb156d52d014bb444a14138cbee36044c6faed37d0c2d49d2358315c465cbf8c5536970e8a29bb7adcda0d663b20007d481813694c64ef"),
                Value::Coin(2332267427205),
                None,
            )],
        );
        match validate(&metx, &utxos, &env) {
            Ok(()) => panic!("Missing verification key witness"),
            Err(err) => match err {
                ShelleyMA(ShelleyMAError::WrongSignature) => (),
                _ => panic!("Unexpected error ({:?})", err),
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
        let mut tx_wits: MintedWitnessSet = mtx.transaction_witness_set.unwrap().clone();
        tx_wits.native_script = Some(Vec::new());
        let mut tx_buf: Vec<u8> = Vec::new();
        match encode(tx_wits, &mut tx_buf) {
            Ok(_) => (),
            Err(err) => panic!("Unable to encode Tx ({:?})", err),
        };
        mtx.transaction_witness_set =
            Decode::decode(&mut Decoder::new(tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Shelley);
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Shelley(ShelleyProtParams {
                minfee_b: 155381,
                minfee_a: 44,
                max_block_body_size: 65536,
                max_transaction_size: 4096,
                max_block_header_size: 1100,
                key_deposit: 2000000,
                pool_deposit: 500000000,
                maximum_epoch: 18,
                desired_number_of_stake_pools: 150,
                pool_pledge_influence: RationalNumber {
                    // FIX: this is a made-up value.
                    numerator: 1,
                    denominator: 1,
                },
                expansion_rate: RationalNumber {
                    // FIX: this is a made-up value.
                    numerator: 1,
                    denominator: 1,
                },
                treasury_growth_rate: RationalNumber {
                    // FIX: this is a made-up value.
                    numerator: 1,
                    denominator: 1,
                },
                decentralization_constant: RationalNumber {
                    numerator: 1,
                    denominator: 1,
                },
                extra_entropy: Nonce {
                    variant: NonceVariant::NeutralNonce,
                    hash: None,
                },
                protocol_version: (0, 2),
                min_utxo_value: 1000000,
                min_pool_cost: 340000000,
            }),
            prot_magic: 764824073,
            block_slot: 5281340,
            network_id: 1,
        };
        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(
                String::from("7165c197d565e88a20885e535f93755682444d3c02fd44dd70883fe89e"),
                Value::Coin(2000000),
                None,
            )],
        );
        match validate(&metx, &utxos, &env) {
            Ok(()) => panic!("Missing native script witness"),
            Err(err) => match err {
                ShelleyMA(ShelleyMAError::MissingScriptWitness) => (),
                _ => panic!("Unexpected error ({:?})", err),
            },
        }
    }
}
