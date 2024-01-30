pub mod common;

use common::*;
use hex;
use pallas_applying::{
    utils::{BabbageProtParams, Environment, FeePolicy, MultiEraProtParams},
    validate, UTxOs,
};
use pallas_codec::utils::{Bytes, CborWrap, KeyValuePairs};
use pallas_primitives::babbage::{
    MintedDatumOption, MintedScriptRef, MintedTx, PseudoDatumOption, Value,
};
use pallas_traverse::MultiEraTx;

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
}
