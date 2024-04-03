pub mod common;

use common::{cbor_to_bytes, minted_tx_payload_from_cbor, mk_utxo_for_byron_tx};
use pallas_applying::{
    utils::{ByronError, Environment, ValidationError::*},
    validate, UTxOs,
};

use pallas_codec::{
    minicbor::{
        decode::{Decode, Decoder},
        encode,
    },
    utils::{CborWrap, MaybeIndefArray},
};
use pallas_primitives::byron::{ByronProtParams, MintedTxPayload, Twit, Tx, TxOut, Witnesses};
use pallas_traverse::{MultiEraProtocolParameters, MultiEraTx};
use std::vec::Vec;

#[cfg(test)]
mod byron_tests {
    use super::*;

    #[test]
    // Transaction hash:
    // a9e4413a5fb61a7a43c7df006ffcaaf3f2ffc9541f54757023968c5a8f8294fd
    fn successful_mainnet_tx_with_genesis_utxos() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/byron2.tx"));
        let mtxp: MintedTxPayload = minted_tx_payload_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_byron(&mtxp);
        let utxos: UTxOs = mk_utxo_for_byron_tx(
            &mtxp.transaction,
            &[(
                String::from(include_str!("../../test_data/byron2.address")),
                19999000000,
            )],
        );
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Byron(ByronProtParams {
                script_version: 0,
                slot_duration: 20000,
                max_block_size: 2000000,
                max_header_size: 2000000,
                max_tx_size: 4096,
                max_proposal_size: 700,
                mpc_thd: 20000000000000,
                heavy_del_thd: 300000000000,
                update_vote_thd: 1000000000000,
                update_proposal_thd: 100000000000000,
                update_implicit: 10000,
                soft_fork_rule: (900000000000000, 600000000000000, 50000000000000),
                summand: 155381,
                multiplier: 44,
                unlock_stake_epoch: 18446744073709551615,
            }),
            prot_magic: 764824073,
            block_slot: 6341,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => (),
            Err(err) => panic!("Unexpected error ({:?})", err),
        }
    }

    #[test]
    // Transaction hash:
    // a06e5a0150e09f8983be2deafab9e04afc60d92e7110999eb672c903343f1e26
    fn successful_mainnet_tx() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/byron1.tx"));
        let mtxp: MintedTxPayload = minted_tx_payload_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_byron(&mtxp);
        let utxos: UTxOs = mk_utxo_for_byron_tx(
            &mtxp.transaction,
            &[(
                String::from(include_str!("../../test_data/byron1.address")),
                19999000000,
            )],
        );
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Byron(ByronProtParams {
                script_version: 0,
                slot_duration: 20000,
                max_block_size: 2000000,
                max_header_size: 2000000,
                max_tx_size: 4096,
                max_proposal_size: 700,
                mpc_thd: 20000000000000,
                heavy_del_thd: 300000000000,
                update_vote_thd: 1000000000000,
                update_proposal_thd: 100000000000000,
                update_implicit: 10000,
                soft_fork_rule: (900000000000000, 600000000000000, 50000000000000),
                summand: 155381,
                multiplier: 44,
                unlock_stake_epoch: 18446744073709551615,
            }),
            prot_magic: 764824073,
            block_slot: 3241381,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => (),
            Err(err) => panic!("Unexpected error ({:?})", err),
        }
    }

    #[test]
    // Identical to successful_mainnet_tx, except that all inputs are removed.
    fn empty_ins() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/byron1.tx"));
        let mut mtxp: MintedTxPayload = minted_tx_payload_from_cbor(&cbor_bytes);
        let utxos: UTxOs = mk_utxo_for_byron_tx(
            &mtxp.transaction,
            &[(
                String::from(include_str!("../../test_data/byron1.address")),
                19999000000,
            )],
        );
        // Clear the set of inputs in the transaction.
        let mut tx: Tx = (*mtxp.transaction).clone();
        tx.inputs = MaybeIndefArray::Def(Vec::new());
        let mut tx_buf: Vec<u8> = Vec::new();
        match encode(tx, &mut tx_buf) {
            Ok(_) => (),
            Err(err) => panic!("Unable to encode Tx ({:?})", err),
        };
        mtxp.transaction = Decode::decode(&mut Decoder::new(tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_byron(&mtxp);
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Byron(ByronProtParams {
                script_version: 0,
                slot_duration: 20000,
                max_block_size: 2000000,
                max_header_size: 2000000,
                max_tx_size: 4096,
                max_proposal_size: 700,
                mpc_thd: 20000000000000,
                heavy_del_thd: 300000000000,
                update_vote_thd: 1000000000000,
                update_proposal_thd: 100000000000000,
                update_implicit: 10000,
                soft_fork_rule: (900000000000000, 600000000000000, 50000000000000),
                summand: 155381,
                multiplier: 44,
                unlock_stake_epoch: 18446744073709551615,
            }),
            prot_magic: 764824073,
            block_slot: 3241381,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => panic!("Inputs set should not be empty"),
            Err(err) => match err {
                Byron(ByronError::TxInsEmpty) => (),
                _ => panic!("Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Identical to successful_mainnet_tx, except that all outputs are removed.
    fn empty_outs() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/byron1.tx"));
        let mut mtxp: MintedTxPayload = minted_tx_payload_from_cbor(&cbor_bytes);
        // Clear the set of outputs in the transaction.
        let mut tx: Tx = (*mtxp.transaction).clone();
        tx.outputs = MaybeIndefArray::Def(Vec::new());
        let mut tx_buf: Vec<u8> = Vec::new();
        match encode(tx, &mut tx_buf) {
            Ok(_) => (),
            Err(err) => panic!("Unable to encode Tx ({:?})", err),
        };
        mtxp.transaction = Decode::decode(&mut Decoder::new(tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_byron(&mtxp);
        let utxos: UTxOs = mk_utxo_for_byron_tx(
            &mtxp.transaction,
            &[(
                String::from(include_str!("../../test_data/byron1.address")),
                19999000000,
            )],
        );
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Byron(ByronProtParams {
                script_version: 0,
                slot_duration: 20000,
                max_block_size: 2000000,
                max_header_size: 2000000,
                max_tx_size: 4096,
                max_proposal_size: 700,
                mpc_thd: 20000000000000,
                heavy_del_thd: 300000000000,
                update_vote_thd: 1000000000000,
                update_proposal_thd: 100000000000000,
                update_implicit: 10000,
                soft_fork_rule: (900000000000000, 600000000000000, 50000000000000),
                summand: 155381,
                multiplier: 44,
                unlock_stake_epoch: 18446744073709551615,
            }),
            prot_magic: 764824073,
            block_slot: 3241381,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => panic!("Outputs set should not be empty"),
            Err(err) => match err {
                Byron(ByronError::TxOutsEmpty) => (),
                _ => panic!("Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // The transaction is valid, but the UTxO set is empty.
    fn unfound_utxo() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/byron1.tx"));
        let mtxp: MintedTxPayload = minted_tx_payload_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_byron(&mtxp);
        let utxos: UTxOs = UTxOs::new();
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Byron(ByronProtParams {
                script_version: 0,
                slot_duration: 20000,
                max_block_size: 2000000,
                max_header_size: 2000000,
                max_tx_size: 4096,
                max_proposal_size: 700,
                mpc_thd: 20000000000000,
                heavy_del_thd: 300000000000,
                update_vote_thd: 1000000000000,
                update_proposal_thd: 100000000000000,
                update_implicit: 10000,
                soft_fork_rule: (900000000000000, 600000000000000, 50000000000000),
                summand: 155381,
                multiplier: 44,
                unlock_stake_epoch: 18446744073709551615,
            }),
            prot_magic: 764824073,
            block_slot: 3241381,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => panic!("All inputs must be within the UTxO set"),
            Err(err) => match err {
                Byron(ByronError::InputNotInUTxO) => (),
                _ => panic!("Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // All lovelace in one of the outputs was removed.
    fn output_without_lovelace() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/byron1.tx"));
        let mut mtxp: MintedTxPayload = minted_tx_payload_from_cbor(&cbor_bytes);
        // Remove lovelace from output.
        let mut tx: Tx = (*mtxp.transaction).clone();
        let altered_tx_out: TxOut = TxOut {
            address: tx.outputs[0].address.clone(),
            amount: 0,
        };

        let new_tx_outs: Vec<TxOut> = vec![tx.outputs[1].clone(), altered_tx_out];
        tx.outputs = MaybeIndefArray::Indef(new_tx_outs);
        let mut tx_buf: Vec<u8> = Vec::new();
        match encode(tx, &mut tx_buf) {
            Ok(_) => (),
            Err(err) => panic!("Unable to encode Tx ({:?})", err),
        };
        mtxp.transaction = Decode::decode(&mut Decoder::new(tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_byron(&mtxp);
        let utxos: UTxOs = mk_utxo_for_byron_tx(
            &mtxp.transaction,
            &[(
                String::from(include_str!("../../test_data/byron1.address")),
                19999000000,
            )],
        );
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Byron(ByronProtParams {
                script_version: 0,
                slot_duration: 20000,
                max_block_size: 2000000,
                max_header_size: 2000000,
                max_tx_size: 4096,
                max_proposal_size: 700,
                mpc_thd: 20000000000000,
                heavy_del_thd: 300000000000,
                update_vote_thd: 1000000000000,
                update_proposal_thd: 100000000000000,
                update_implicit: 10000,
                soft_fork_rule: (900000000000000, 600000000000000, 50000000000000),
                summand: 155381,
                multiplier: 44,
                unlock_stake_epoch: 18446744073709551615,
            }),
            prot_magic: 764824073,
            block_slot: 3241381,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => panic!("All outputs must contain lovelace"),
            Err(err) => match err {
                Byron(ByronError::OutputWithoutLovelace) => (),
                _ => panic!("Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Expected fees are increased by increasing the protocol parameters.
    fn not_enough_fees() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/byron1.tx"));
        let mtxp: MintedTxPayload = minted_tx_payload_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_byron(&mtxp);
        let utxos: UTxOs = mk_utxo_for_byron_tx(
            &mtxp.transaction,
            &[(
                String::from(include_str!("../../test_data/byron1.address")),
                19999000000,
            )],
        );
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Byron(ByronProtParams {
                script_version: 0,
                slot_duration: 20000,
                max_block_size: 2000000,
                max_header_size: 2000000,
                max_tx_size: 4096,
                max_proposal_size: 700,
                mpc_thd: 20000000000000,
                heavy_del_thd: 300000000000,
                update_vote_thd: 1000000000000,
                update_proposal_thd: 100000000000000,
                update_implicit: 10000,
                soft_fork_rule: (900000000000000, 600000000000000, 50000000000000),
                summand: 1000,
                multiplier: 1000,
                unlock_stake_epoch: 18446744073709551615,
            }),
            prot_magic: 764824073,
            block_slot: 3241381,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => panic!("Fees should not be below minimum"),
            Err(err) => match err {
                Byron(ByronError::FeesBelowMin) => (),
                _ => panic!("Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // Tx size limit set by protocol parameters is established at 0.
    fn tx_size_exceeds_max() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/byron1.tx"));
        let mtxp: MintedTxPayload = minted_tx_payload_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_byron(&mtxp);
        let utxos: UTxOs = mk_utxo_for_byron_tx(
            &mtxp.transaction,
            &[(
                String::from(include_str!("../../test_data/byron1.address")),
                19999000000,
            )],
        );
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Byron(ByronProtParams {
                script_version: 0,
                slot_duration: 20000,
                max_block_size: 2000000,
                max_header_size: 2000000,
                max_tx_size: 0,
                max_proposal_size: 700,
                mpc_thd: 20000000000000,
                heavy_del_thd: 300000000000,
                update_vote_thd: 1000000000000,
                update_proposal_thd: 100000000000000,
                update_implicit: 10000,
                soft_fork_rule: (900000000000000, 600000000000000, 50000000000000),
                summand: 155381,
                multiplier: 44,
                unlock_stake_epoch: 18446744073709551615,
            }),
            prot_magic: 764824073,
            block_slot: 3241381,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => panic!("Transaction size cannot exceed protocol limit"),
            Err(err) => match err {
                Byron(ByronError::MaxTxSizeExceeded) => (),
                _ => panic!("Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // The input to the transaction does not have a matching witness.
    fn missing_witness() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/byron1.tx"));
        let mut mtxp: MintedTxPayload = minted_tx_payload_from_cbor(&cbor_bytes);
        // Remove witness
        let new_witnesses: Witnesses = MaybeIndefArray::Def(Vec::new());
        let mut tx_buf: Vec<u8> = Vec::new();
        match encode(new_witnesses, &mut tx_buf) {
            Ok(_) => (),
            Err(err) => panic!("Unable to encode Tx ({:?})", err),
        };
        mtxp.witness = Decode::decode(&mut Decoder::new(tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_byron(&mtxp);
        let utxos: UTxOs = mk_utxo_for_byron_tx(
            &mtxp.transaction,
            &[(
                String::from(include_str!("../../test_data/byron1.address")),
                19999000000,
            )],
        );
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Byron(ByronProtParams {
                script_version: 0,
                slot_duration: 20000,
                max_block_size: 2000000,
                max_header_size: 2000000,
                max_tx_size: 4096,
                max_proposal_size: 700,
                mpc_thd: 20000000000000,
                heavy_del_thd: 300000000000,
                update_vote_thd: 1000000000000,
                update_proposal_thd: 100000000000000,
                update_implicit: 10000,
                soft_fork_rule: (900000000000000, 600000000000000, 50000000000000),
                summand: 155381,
                multiplier: 44,
                unlock_stake_epoch: 18446744073709551615,
            }),
            prot_magic: 764824073,
            block_slot: 3241381,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => panic!("All inputs must have a witness signature"),
            Err(err) => match err {
                Byron(ByronError::MissingWitness) => (),
                _ => panic!("Unexpected error ({:?})", err),
            },
        }
    }

    #[test]
    // The input to the transaction has an associated witness, but the signature is
    // wrong.
    fn wrong_signature() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/byron1.tx"));
        let mut mtxp: MintedTxPayload = minted_tx_payload_from_cbor(&cbor_bytes);
        // Modify signature in witness
        let new_wit: Twit = match mtxp.witness[0].clone() {
            Twit::PkWitness(CborWrap((pk, _))) => {
                Twit::PkWitness(CborWrap((pk, [0u8; 64].to_vec().into())))
            }
            _ => unreachable!(),
        };

        let new_witnesses: Witnesses = MaybeIndefArray::Def(vec![new_wit]);
        let mut tx_buf: Vec<u8> = Vec::new();

        match encode(new_witnesses, &mut tx_buf) {
            Ok(_) => (),
            Err(err) => panic!("Unable to encode Tx ({:?})", err),
        };
        mtxp.witness = Decode::decode(&mut Decoder::new(tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_byron(&mtxp);
        let utxos: UTxOs = mk_utxo_for_byron_tx(
            &mtxp.transaction,
            &[(
                String::from(include_str!("../../test_data/byron1.address")),
                19999000000,
            )],
        );
        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Byron(ByronProtParams {
                script_version: 0,
                slot_duration: 20000,
                max_block_size: 2000000,
                max_header_size: 2000000,
                max_tx_size: 4096,
                max_proposal_size: 700,
                mpc_thd: 20000000000000,
                heavy_del_thd: 300000000000,
                update_vote_thd: 1000000000000,
                update_proposal_thd: 100000000000000,
                update_implicit: 10000,
                soft_fork_rule: (900000000000000, 600000000000000, 50000000000000),
                summand: 155381,
                multiplier: 44,
                unlock_stake_epoch: 18446744073709551615,
            }),
            prot_magic: 764824073,
            block_slot: 3241381,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => panic!("Witness signature should verify the transaction"),
            Err(err) => match err {
                Byron(ByronError::WrongSignature) => (),
                _ => panic!("Unexpected error ({:?})", err),
            },
        }
    }
}
