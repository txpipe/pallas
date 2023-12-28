pub mod common;

use common::*;
use pallas_applying::{
    utils::{
        AlonzoError::*, AlonzoProtParams, Environment, FeePolicy, Language, MultiEraProtParams,
        ValidationError::*,
    },
    validate, UTxOs,
};
use pallas_codec::minicbor::{
    decode::{Decode, Decoder},
    encode,
};
use pallas_primitives::alonzo::{MintedTx, TransactionBody, Value};
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
            block_slot: 6447029,
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
            block_slot: 6447029,
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
    // Same as successful_mainnet_tx, but the validation is called with an empty UTxO set.
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
            block_slot: 6447029,
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
    // Same as succesful_mainnet_tx, except that the lower bound of the validity interval is greater
    // than the block slot.
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
        tx_body.validity_interval_start = Some(6447030); // One slot after the block.
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
            block_slot: 6447029,
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
    // Same as succesful_mainnet_tx, except that the upper bound of the validity interval is lower
    // than the block slot.
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
            block_slot: 6447029,
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
    // Same as succesful_mainnet_tx, except that validation is called with an Environment requesting
    // fees that exceed those paid by the transaction.
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
            block_slot: 6447029,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => assert!(false, "Fee should not be below minimum"),
            Err(err) => match err {
                Alonzo(FeesBelowMin) => (),
                _ => assert!(false, "Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Same as succesful_mainnet_tx, except that the fee is reduced by exactly 1, and so the
    // "preservation of value" property doesn't hold.
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
            block_slot: 6447029,
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
}
