pub mod common;

use common::*;
use pallas_addresses::{Address, Network, ShelleyAddress};
use pallas_applying::utils::PoolParam;
use pallas_applying::{
    utils::{
        AccountState, Environment, MultiEraProtocolParameters, ShelleyMAError, ShelleyProtParams,
        ValidationError::*,
    },
    validate_txs, CertState, UTxOs,
};
use pallas_codec::{
    minicbor::{
        decode::{Decode, Decoder},
        encode,
    },
    utils::{Bytes, Nullable},
};
use pallas_crypto::hash::Hash;
use pallas_primitives::alonzo::{
    Certificate, MintedTx, MintedWitnessSet, Nonce, NonceVariant, PoolKeyhash, PoolMetadata,
    RationalNumber, Relay, StakeCredential, TransactionBody, TransactionOutput, VKeyWitness, Value,
};
use pallas_traverse::{Era, MultiEraTx};
use std::str::FromStr;

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
        let acnt = AccountState {
            treasury: 261_254_564_000_000,
            reserves: 0,
        };

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
            acnt: Some(acnt),
        };
        let mut cert_state: CertState = CertState::default();
        match validate_txs(&[metx], &env, &utxos, &mut cert_state) {
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

        let acnt = AccountState {
            treasury: 261_254_564_000_000,
            reserves: 0,
        };

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
            acnt: Some(acnt),
        };
        let mut cert_state: CertState = CertState::default();
        match validate_txs(&[metx], &env, &utxos, &mut cert_state) {
            Ok(()) => (),
            Err(err) => panic!("Unexpected error ({:?})", err),
        }
    }

    #[test]
    // Same as successful_mainnet_shelley_tx_with_script, but changing "All" to "any" and
    // deleting one key-witness pair
    fn successful_mainnet_shelley_tx_with_changed_script() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/shelley4.tx"));
        let mut mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        // Delete one VKey witness.
        let mut tx_wits: MintedWitnessSet = mtx.transaction_witness_set.unwrap().clone();
        let wit: VKeyWitness = tx_wits.vkeywitness.unwrap().remove(1);
        tx_wits.vkeywitness = Some(Vec::from([wit]));
        let mut tx_buf: Vec<u8> = Vec::new();
        match encode(tx_wits, &mut tx_buf) {
            Ok(_) => (),
            Err(err) => panic!("Unable to encode Tx ({:?})", err),
        };
        mtx.transaction_witness_set =
            Decode::decode(&mut Decoder::new(tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Shelley);
        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(
                String::from("711245ed0e86bc58578e4b06958d5b0ef856ed42e5ee8fa811e0745aba"),
                Value::Coin(2000000),
                None,
            )],
        );
        let acnt = AccountState {
            treasury: 261_254_564_000_000,
            reserves: 0,
        };

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
            acnt: Some(acnt),
        };
        let mut cert_state: CertState = CertState::default();
        match validate_txs(&[metx], &env, &utxos, &mut cert_state) {
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

        let acnt = AccountState {
            treasury: 261_254_564_000_000,
            reserves: 0,
        };

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
            acnt: Some(acnt),
        };
        let mut cert_state: CertState = CertState::default();
        match validate_txs(&[metx], &env, &utxos, &mut cert_state) {
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

        let acnt = AccountState {
            treasury: 261_254_564_000_000,
            reserves: 0,
        };

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
            acnt: Some(acnt),
        };
        let mut cert_state: CertState = CertState::default();
        match validate_txs(&[metx], &env, &utxos, &mut cert_state) {
            Ok(()) => (),
            Err(err) => panic!("Unexpected error ({:?})", err),
        }
    }

    #[test]
    // Transaction hash:
    // ce8ba608357e31695ce7be1a4a9875f43b3fd264f106e455e870714f149af925
    fn successful_mainnet_mary_tx_with_pool_reg() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/mary2.tx"));
        let mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Mary);
        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(
                String::from("018e8f7a7073b8a95a4c1f1cf412b1042fca4945b89eb11754b3481b29fb2b631db76384f64dd94b47f97fc8c2a206764c17a1de7da2f70e83"),
                Value::Coin(1_507_817_955),
                None,
            )],
        );

        let acnt = AccountState {
            treasury: 261_254_564_000_000,
            reserves: 0,
        };

        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Shelley(ShelleyProtParams {
                minfee_b: 155381,
                minfee_a: 44,
                max_block_body_size: 65536,
                max_transaction_size: 16384,
                max_block_header_size: 1100,
                key_deposit: 2_000_000,
                pool_deposit: 500_000_000,
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
                protocol_version: (4, 0),
                min_utxo_value: 1_000_000,
                min_pool_cost: 340_000_000,
            }),
            prot_magic: 764824073,
            block_slot: 26342415,
            network_id: 1,
            acnt: Some(acnt),
        };
        let mut cert_state: CertState = CertState::default();
        let hash =
            Hash::from_str("FB2B631DB76384F64DD94B47F97FC8C2A206764C17A1DE7DA2F70E83").unwrap();
        cert_state
            .dstate
            .rewards
            .insert(StakeCredential::AddrKeyhash(hash), 0);

        match validate_txs(&[metx], &env, &utxos, &mut cert_state) {
            Ok(()) => (),
            Err(err) => panic!("Unexpected error ({:?})", err),
        };

        if !cert_state
            .pstate
            .pool_params
            .contains_key(&mary2_pool_operator())
        {
            panic!("Pool not registered or keyhash mismatch");
        }
    }

    const MARY3_UTXO: &str = "014faace6b1de3b825da7c7f4308917822049cdedb5868f7623f892d4e39cf0461807b986a6477205e376dac280d7f150eb497025f67c49757";

    #[test]
    // Transaction hash:
    // cc6a92cc0f4ea326439bac6b18bc7b424470c508a99b9aebc8fafc027d906465
    fn successful_mainnet_mary_tx_with_stk_deleg() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/mary3.tx"));
        let mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Mary);
        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(String::from(MARY3_UTXO), Value::Coin(627_760_000), None)],
        );

        let mut cert_state: CertState = CertState::default();
        cert_state
            .pstate
            .pool_params
            .insert(mary2_pool_operator(), mary2_pool_param());
        match validate_txs(&[metx], &mary3_env(), &utxos, &mut cert_state) {
            Ok(()) => (),
            Err(err) => panic!("Unexpected error ({:?})", err),
        }
    }

    fn mary2_pool_operator() -> PoolKeyhash {
        Hash::from_str("59EBE72AE96462018FBE04633100F90B3066688D85F00F3BD254707F").unwrap()
    }

    // Params for the pool registered in `successful_mainnet_mary_tx_with_pool_reg`
    fn mary2_pool_param() -> PoolParam {
        PoolParam {
            vrf_keyhash: Hash::from_str(
                "1EFB798F239B9B02DEB4636A3AB1962AF43512595FCB82276E11971E684E49B7",
            )
            .unwrap(),
            pledge: 1000000000,
            cost: 340000000,
            margin: RationalNumber {
                numerator: 3,
                denominator: 100,
            },
            reward_account: hex::decode(
                "E1FB2B631DB76384F64DD94B47F97FC8C2A206764C17A1DE7DA2F70E83",
            )
            .unwrap()
            .into(),
            pool_owners: Vec::from([Hash::from_str(
                "FB2B631DB76384F64DD94B47F97FC8C2A206764C17A1DE7DA2F70E83",
            )
            .unwrap()]),
            relays: [Relay::SingleHostAddr(
                Nullable::Some(3001),
                Nullable::Some(hex::decode("C22614BB").unwrap().into()),
                Nullable::Null,
            )]
            .to_vec(),
            pool_metadata: Nullable::Some(PoolMetadata {
                url: "https://cardapool.com/a.json".to_string(),
                hash: Hash::from_str(
                    "01F708549816C9A075FF96E9682C11A5F5C7F4E147862A663BDEECE0716AB76E",
                )
                .unwrap(),
            }),
        }
    }

    fn mary3_env() -> Environment {
        let acnt = AccountState {
            treasury: 374_930_989_230_000,
            reserves: 12_618_536_190_580_000,
        };

        Environment {
            prot_params: MultiEraProtocolParameters::Shelley(ShelleyProtParams {
                minfee_b: 155381,
                minfee_a: 44,
                max_block_body_size: 65536,
                max_transaction_size: 16384,
                max_block_header_size: 1100,
                key_deposit: 2_000_000,
                pool_deposit: 500_000_000,
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
                protocol_version: (4, 0),
                min_utxo_value: 1_000_000,
                min_pool_cost: 340_000_000,
            }),
            prot_magic: 764824073,
            block_slot: 29_035_358,
            network_id: 1,
            acnt: Some(acnt),
        }
    }

    #[test]
    // Transaction hash:
    // 99f621beaacefc14ad8912b777422600e707f75bf619b2af20e918b0fe53f882
    // A total of 10_797_095_002 lovelace is drawn from the Treasury.
    fn successful_mainnet_allegra_tx_with_mir() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/allegra1.tx"));
        let mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Mary);
        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(
                String::from("61b651c2062463499961b9cd594da399a5ec910fceb5c63f9eb55a224a"),
                Value::Coin(96_400_000),
                None,
            )],
        );

        let acnt = AccountState {
            treasury: 261_254_564_000_000,
            reserves: 0,
        };

        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Shelley(ShelleyProtParams {
                minfee_b: 155381,
                minfee_a: 44,
                max_block_body_size: 65536,
                max_transaction_size: 16384,
                max_block_header_size: 1100,
                key_deposit: 2_000_000,
                pool_deposit: 500_000_000,
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
                    numerator: 3,
                    denominator: 10,
                },
                extra_entropy: Nonce {
                    variant: NonceVariant::NeutralNonce,
                    hash: None,
                },
                protocol_version: (3, 0),
                min_utxo_value: 1_000_000,
                min_pool_cost: 340_000_000,
            }),
            prot_magic: 764824073,
            block_slot: 19282133,
            network_id: 1,
            acnt: Some(acnt),
        };
        let mut cert_state: CertState = CertState::default();
        match validate_txs(&[metx], &env, &utxos, &mut cert_state) {
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

        let acnt = AccountState {
            treasury: 261_254_564_000_000,
            reserves: 0,
        };

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
            acnt: Some(acnt),
        };
        let mut cert_state: CertState = CertState::default();
        match validate_txs(&[metx], &env, &utxos, &mut cert_state) {
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

        let acnt = AccountState {
            treasury: 261_254_564_000_000,
            reserves: 0,
        };

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
            acnt: Some(acnt),
        };
        let mut cert_state: CertState = CertState::default();
        match validate_txs(&[metx], &env, &utxos, &mut cert_state) {
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

        let acnt = AccountState {
            treasury: 261_254_564_000_000,
            reserves: 0,
        };

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
            acnt: Some(acnt),
        };
        let mut cert_state: CertState = CertState::default();
        match validate_txs(&[metx], &env, &utxos, &mut cert_state) {
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

        let acnt = AccountState {
            treasury: 261_254_564_000_000,
            reserves: 0,
        };

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
            acnt: Some(acnt),
        };
        let mut cert_state: CertState = CertState::default();
        match validate_txs(&[metx], &env, &utxos, &mut cert_state) {
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

        let acnt = AccountState {
            treasury: 261_254_564_000_000,
            reserves: 0,
        };

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
            acnt: Some(acnt),
        };
        let mut cert_state: CertState = CertState::default();
        match validate_txs(&[metx], &env, &utxos, &mut cert_state) {
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

        let acnt = AccountState {
            treasury: 261_254_564_000_000,
            reserves: 0,
        };

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
            acnt: Some(acnt),
        };
        let mut cert_state: CertState = CertState::default();
        match validate_txs(&[metx], &env, &utxos, &mut cert_state) {
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

        let acnt = AccountState {
            treasury: 261_254_564_000_000,
            reserves: 0,
        };

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
            acnt: Some(acnt),
        };
        let mut cert_state: CertState = CertState::default();
        match validate_txs(&[metx], &env, &utxos, &mut cert_state) {
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

        let acnt = AccountState {
            treasury: 261_254_564_000_000,
            reserves: 0,
        };

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
            acnt: Some(acnt),
        };
        let mut cert_state: CertState = CertState::default();
        match validate_txs(&[metx], &env, &utxos, &mut cert_state) {
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

        let acnt = AccountState {
            treasury: 261_254_564_000_000,
            reserves: 0,
        };

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
            acnt: Some(acnt),
        };
        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(
                String::from("0129bb156d52d014bb444a14138cbee36044c6faed37d0c2d49d2358315c465cbf8c5536970e8a29bb7adcda0d663b20007d481813694c64ef"),
                Value::Coin(2332267427205),
                None,
            )],
        );
        let mut cert_state: CertState = CertState::default();
        match validate_txs(&[metx], &env, &utxos, &mut cert_state) {
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

        let acnt = AccountState {
            treasury: 261_254_564_000_000,
            reserves: 0,
        };

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
            acnt: Some(acnt),
        };
        let mut cert_state: CertState = CertState::default();
        match validate_txs(&[metx], &env, &utxos, &mut cert_state) {
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

        let acnt = AccountState {
            treasury: 261_254_564_000_000,
            reserves: 0,
        };

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
            acnt: Some(acnt),
        };
        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(
                String::from("0129bb156d52d014bb444a14138cbee36044c6faed37d0c2d49d2358315c465cbf8c5536970e8a29bb7adcda0d663b20007d481813694c64ef"),
                Value::Coin(2332267427205),
                None,
            )],
        );
        let mut cert_state: CertState = CertState::default();
        match validate_txs(&[metx], &env, &utxos, &mut cert_state) {
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

        let acnt = AccountState {
            treasury: 261_254_564_000_000,
            reserves: 0,
        };

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
            acnt: Some(acnt),
        };
        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(
                String::from("0129bb156d52d014bb444a14138cbee36044c6faed37d0c2d49d2358315c465cbf8c5536970e8a29bb7adcda0d663b20007d481813694c64ef"),
                Value::Coin(2332267427205),
                None,
            )],
        );
        let mut cert_state: CertState = CertState::default();
        match validate_txs(&[metx], &env, &utxos, &mut cert_state) {
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

        let acnt = AccountState {
            treasury: 261_254_564_000_000,
            reserves: 0,
        };

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
            acnt: Some(acnt),
        };
        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(
                String::from("7165c197d565e88a20885e535f93755682444d3c02fd44dd70883fe89e"),
                Value::Coin(2000000),
                None,
            )],
        );
        let mut cert_state: CertState = CertState::default();
        match validate_txs(&[metx], &env, &utxos, &mut cert_state) {
            Ok(()) => panic!("Missing native script witness"),
            Err(err) => match err {
                ShelleyMA(ShelleyMAError::MissingScriptWitness) => (),
                _ => panic!("Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Like successful_mainnet_shelley_tx (hash:
    // 50eba65e73c8c5f7b09f4ea28cf15dce169f3d1c322ca3deff03725f51518bb2), but one
    // verification-key witness is removed
    // (the same one of successful_mainnet_shelley_tx_with_changed_script).
    fn missing_signature_native_script() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/shelley2.tx"));
        let mut mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        // Delete one VKey witness.
        let mut tx_wits: MintedWitnessSet = mtx.transaction_witness_set.unwrap().clone();
        let wit: VKeyWitness = tx_wits.vkeywitness.unwrap().remove(1);
        tx_wits.vkeywitness = Some(Vec::from([wit]));
        let mut tx_buf: Vec<u8> = Vec::new();
        match encode(tx_wits, &mut tx_buf) {
            Ok(_) => (),
            Err(err) => panic!("Unable to encode Tx ({:?})", err),
        };
        mtx.transaction_witness_set =
            Decode::decode(&mut Decoder::new(tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Shelley);

        let acnt = AccountState {
            treasury: 261_254_564_000_000,
            reserves: 0,
        };

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
            acnt: Some(acnt),
        };
        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(
                String::from("7165c197d565e88a20885e535f93755682444d3c02fd44dd70883fe89e"),
                Value::Coin(2000000),
                None,
            )],
        );
        let mut cert_state: CertState = CertState::default();
        match validate_txs(&[metx], &env, &utxos, &mut cert_state) {
            Ok(()) => panic!("The script is not satisfied"),
            Err(err) => match err {
                ShelleyMA(ShelleyMAError::ScriptDenial) => (),
                _ => panic!("Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Like `successful_mainnet_mary_tx_with_stk_deleg`,
    // but the pool to which the delegation occurs is not registered.
    fn unregistered_pool() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/mary3.tx"));
        let mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Mary);
        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(String::from(MARY3_UTXO), Value::Coin(627_760_000), None)],
        );

        let mut cert_state: CertState = CertState::default();
        match validate_txs(&[metx], &mary3_env(), &utxos, &mut cert_state) {
            Ok(()) => panic!("Pool is not registered"),
            Err(err) => match err {
                ShelleyMA(ShelleyMAError::PoolNotRegistered) => (),
                _ => panic!("Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Like `successful_mainnet_mary_tx_with_stk_deleg`,
    // but the order of the certificates (stake registration and delegation)
    // is flipped.
    fn delegation_before_registration() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/mary3.tx"));
        let mut mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        // Permute certificates
        let old_certs: Vec<Certificate> =
            mtx.transaction_body.certificates.as_ref().unwrap().clone();
        let new_certs: Option<Vec<Certificate>> =
            Some(Vec::from([old_certs[1].clone(), old_certs[0].clone()]));
        let mut tx_body: TransactionBody = mtx.transaction_body.unwrap().clone();
        tx_body.certificates = new_certs;
        let mut tx_buf: Vec<u8> = Vec::new();
        match encode(tx_body, &mut tx_buf) {
            Ok(_) => (),
            Err(err) => panic!("Unable to encode Tx ({:?})", err),
        };
        mtx.transaction_body =
            Decode::decode(&mut Decoder::new(tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Mary);

        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(String::from(MARY3_UTXO), Value::Coin(627_760_000), None)],
        );

        let mut cert_state: CertState = CertState::default();
        cert_state
            .pstate
            .pool_params
            .insert(mary2_pool_operator(), mary2_pool_param());
        match validate_txs(&[metx], &mary3_env(), &utxos, &mut cert_state) {
            Ok(()) => panic!("Staking key is not registered"),
            Err(err) => match err {
                ShelleyMA(ShelleyMAError::KeyNotRegistered) => (),
                _ => panic!("Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Same as successful_mainnet_allegra_tx_with_mir(),
    // but the the slot is advanced to a later moment.
    fn too_late_for_mir() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/allegra1.tx"));
        let mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Mary);
        let utxos: UTxOs = mk_utxo_for_alonzo_compatible_tx(
            &mtx.transaction_body,
            &[(
                String::from("61b651c2062463499961b9cd594da399a5ec910fceb5c63f9eb55a224a"),
                Value::Coin(96_400_000),
                None,
            )],
        );

        let acnt = AccountState {
            treasury: 261_254_564_000_000,
            reserves: 0,
        };

        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Shelley(ShelleyProtParams {
                minfee_b: 155381,
                minfee_a: 44,
                max_block_body_size: 65536,
                max_transaction_size: 16384,
                max_block_header_size: 1100,
                key_deposit: 2_000_000,
                pool_deposit: 500_000_000,
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
                    numerator: 3,
                    denominator: 10,
                },
                extra_entropy: Nonce {
                    variant: NonceVariant::NeutralNonce,
                    hash: None,
                },
                protocol_version: (3, 0),
                min_utxo_value: 1_000_000,
                min_pool_cost: 340_000_000,
            }),
            prot_magic: 764824073,
            block_slot: 19483200,
            network_id: 1,
            acnt: Some(acnt),
        };
        let mut cert_state: CertState = CertState::default();
        match validate_txs(&[metx], &env, &utxos, &mut cert_state) {
            Ok(()) => panic!("MIR after the stability window"),
            Err(err) => match err {
                ShelleyMA(ShelleyMAError::MIRCertificateTooLateinEpoch) => (),
                _ => panic!("Unexpected error ({:?})", err),
            },
        }
    }
}
