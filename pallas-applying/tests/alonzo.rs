pub mod common;

use common::*;
use pallas_applying::{
    utils::{AlonzoProtParams, Environment, FeePolicy, Language, MultiEraProtParams},
    validate, UTxOs,
};
use pallas_primitives::alonzo::{MintedTx, Value};
use pallas_traverse::{Era, MultiEraTx};

#[cfg(test)]
mod alonzo_tests {
    use super::*;

    #[test]
    // Transaction hash: 704b3b9c96f44cd5676e5dcb5dc0bb2555c66427625ccefe620101665da86868
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
            block_slot: 5281340,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => (),
            Err(err) => assert!(false, "Unexpected error ({:?})", err),
        }
    }
}
