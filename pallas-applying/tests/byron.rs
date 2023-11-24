use std::{borrow::Cow, vec::Vec};

use pallas_applying::{
    types::{ByronError::*, ByronProtParams, Environment, MultiEraProtParams, ValidationError::*},
    validate, UTxOs,
};
use pallas_codec::{
    minicbor::{
        bytes::ByteVec,
        decode::{Decode, Decoder},
        encode,
    },
    utils::{CborWrap, MaybeIndefArray, TagWrap},
};
use pallas_primitives::byron::{Address, MintedTxPayload, Twit, Tx, TxIn, TxOut, Witnesses};
use pallas_traverse::{MultiEraInput, MultiEraOutput, MultiEraTx};

#[cfg(test)]
mod byron_tests {
    use super::*;

    fn cbor_to_bytes(input: &str) -> Vec<u8> {
        hex::decode(input).unwrap()
    }

    fn tx_from_cbor<'a>(tx_cbor: &'a Vec<u8>) -> MintedTxPayload<'a> {
        pallas_codec::minicbor::decode::<MintedTxPayload>(&tx_cbor[..]).unwrap()
    }

    // Careful: this function assumes tx has exactly one input.
    fn mk_utxo_for_single_input_tx<'a>(tx: &Tx, address_payload: String, amount: u64) -> UTxOs<'a> {
        let mut tx_ins: Vec<TxIn> = tx.inputs.clone().to_vec();
        assert_eq!(tx_ins.len(), 1, "Unexpected number of inputs.");
        let tx_in: TxIn = tx_ins.pop().unwrap();
        let input_tx_out_addr: Address = match hex::decode(address_payload) {
            Ok(addr_bytes) => Address {
                payload: TagWrap(ByteVec::from(addr_bytes)),
                crc: 3430631884,
            },
            _ => panic!("Unable to decode input address."),
        };
        let tx_out: TxOut = TxOut {
            address: input_tx_out_addr,
            amount,
        };
        let mut utxos: UTxOs = UTxOs::new();
        add_to_utxo(&mut utxos, tx_in, tx_out);
        utxos
    }

    #[test]
    // Transaction hash: a9e4413a5fb61a7a43c7df006ffcaaf3f2ffc9541f54757023968c5a8f8294fd
    fn successful_mainnet_tx_with_genesis_utxos() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/byron2.tx"));
        let mtxp: MintedTxPayload = tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_byron(&mtxp);
        let utxos: UTxOs = mk_utxo_for_single_input_tx(
            &mtxp.transaction,
            String::from(include_str!("../../test_data/byron2.address")),
            19999000000,
        );
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Byron(ByronProtParams {
                min_fees_const: 155381,
                min_fees_factor: 44,
                max_tx_size: 4096,
            }),
            prot_magic: 764824073,
            block_slot: 6341,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => (),
            Err(err) => panic!("Unexpected error ({:?}).", err),
        }
    }

    #[test]
    // Transaction hash: a06e5a0150e09f8983be2deafab9e04afc60d92e7110999eb672c903343f1e26
    fn successful_mainnet_tx() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/byron1.tx"));
        let mtxp: MintedTxPayload = tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_byron(&mtxp);
        let utxos: UTxOs = mk_utxo_for_single_input_tx(
            &mtxp.transaction,
            String::from(include_str!("../../test_data/byron1.address")),
            19999000000,
        );
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Byron(ByronProtParams {
                min_fees_const: 155381,
                min_fees_factor: 44,
                max_tx_size: 4096,
            }),
            prot_magic: 764824073,
            block_slot: 3241381,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => (),
            Err(err) => panic!("Unexpected error ({:?}).", err),
        }
    }

    #[test]
    // Identical to successful_mainnet_tx, except that all inputs are removed.
    fn empty_ins() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/byron1.tx"));
        let mut mtxp: MintedTxPayload = tx_from_cbor(&cbor_bytes);
        let utxos: UTxOs = mk_utxo_for_single_input_tx(
            &mtxp.transaction,
            String::from(include_str!("../../test_data/byron1.address")),
            19999000000,
        );
        // Clear the set of inputs in the transaction.
        let mut tx: Tx = (*mtxp.transaction).clone();
        tx.inputs = MaybeIndefArray::Def(Vec::new());
        let mut tx_buf: Vec<u8> = Vec::new();
        match encode(tx, &mut tx_buf) {
            Ok(_) => (),
            Err(err) => panic!("Unable to encode Tx ({:?}).", err),
        };
        mtxp.transaction = Decode::decode(&mut Decoder::new(&tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_byron(&mtxp);
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Byron(ByronProtParams {
                min_fees_const: 155381,
                min_fees_factor: 44,
                max_tx_size: 4096,
            }),
            prot_magic: 764824073,
            block_slot: 3241381,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => assert!(false, "Inputs set should not be empty."),
            Err(err) => match err {
                Byron(TxInsEmpty) => (),
                _ => panic!("Unexpected error ({:?}).", err),
            },
        }
    }

    #[test]
    // Identical to successful_mainnet_tx, except that all outputs are removed.
    fn empty_outs() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/byron1.tx"));
        let mut mtxp: MintedTxPayload = tx_from_cbor(&cbor_bytes);
        // Clear the set of outputs in the transaction.
        let mut tx: Tx = (*mtxp.transaction).clone();
        tx.outputs = MaybeIndefArray::Def(Vec::new());
        let mut tx_buf: Vec<u8> = Vec::new();
        match encode(tx, &mut tx_buf) {
            Ok(_) => (),
            Err(err) => panic!("Unable to encode Tx ({:?}).", err),
        };
        mtxp.transaction = Decode::decode(&mut Decoder::new(&tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_byron(&mtxp);
        let utxos: UTxOs = mk_utxo_for_single_input_tx(
            &mtxp.transaction,
            String::from(include_str!("../../test_data/byron1.address")),
            19999000000,
        );
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Byron(ByronProtParams {
                min_fees_const: 155381,
                min_fees_factor: 44,
                max_tx_size: 4096,
            }),
            prot_magic: 764824073,
            block_slot: 3241381,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => assert!(false, "Outputs set should not be empty."),
            Err(err) => match err {
                Byron(TxOutsEmpty) => (),
                _ => panic!("Unexpected error ({:?}).", err),
            },
        }
    }

    #[test]
    // The transaction is valid, but the UTxO set is empty.
    fn unfound_utxo() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/byron1.tx"));
        let mtxp: MintedTxPayload = tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_byron(&mtxp);
        let utxos: UTxOs = UTxOs::new();
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Byron(ByronProtParams {
                min_fees_const: 155381,
                min_fees_factor: 44,
                max_tx_size: 4096,
            }),
            prot_magic: 764824073,
            block_slot: 3241381,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => assert!(false, "All inputs must be within the UTxO set."),
            Err(err) => match err {
                Byron(InputNotInUTxO) => (),
                _ => panic!("Unexpected error ({:?}).", err),
            },
        }
    }

    #[test]
    // All lovelace in one of the outputs was removed.
    fn output_without_lovelace() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/byron1.tx"));
        let mut mtxp: MintedTxPayload = tx_from_cbor(&cbor_bytes);
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
            Err(err) => panic!("Unable to encode Tx ({:?}).", err),
        };
        mtxp.transaction = Decode::decode(&mut Decoder::new(&tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_byron(&mtxp);
        let utxos: UTxOs = mk_utxo_for_single_input_tx(
            &mtxp.transaction,
            String::from(include_str!("../../test_data/byron1.address")),
            19999000000,
        );
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Byron(ByronProtParams {
                min_fees_const: 155381,
                min_fees_factor: 44,
                max_tx_size: 4096,
            }),
            prot_magic: 764824073,
            block_slot: 3241381,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => assert!(false, "All outputs must contain lovelace."),
            Err(err) => match err {
                Byron(OutputWithoutLovelace) => (),
                _ => panic!("Unexpected error ({:?}).", err),
            },
        }
    }

    #[test]
    // Expected fees are increased by increasing the protocol parameters.
    fn not_enough_fees() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/byron1.tx"));
        let mtxp: MintedTxPayload = tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_byron(&mtxp);
        let utxos: UTxOs = mk_utxo_for_single_input_tx(
            &mtxp.transaction,
            String::from(include_str!("../../test_data/byron1.address")),
            19999000000,
        );
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Byron(ByronProtParams {
                min_fees_const: 1000,
                min_fees_factor: 1000,
                max_tx_size: 4096,
            }),
            prot_magic: 764824073,
            block_slot: 3241381,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => assert!(false, "Fees should not be below minimum."),
            Err(err) => match err {
                Byron(FeesBelowMin) => (),
                _ => panic!("Unexpected error ({:?}).", err),
            },
        }
    }

    #[test]
    // Tx size limit set by protocol parameters is established at 0.
    fn tx_size_exceeds_max() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/byron1.tx"));
        let mtxp: MintedTxPayload = tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_byron(&mtxp);
        let utxos: UTxOs = mk_utxo_for_single_input_tx(
            &mtxp.transaction,
            String::from(include_str!("../../test_data/byron1.address")),
            19999000000,
        );
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Byron(ByronProtParams {
                min_fees_const: 155381,
                min_fees_factor: 44,
                max_tx_size: 0,
            }),
            prot_magic: 764824073,
            block_slot: 3241381,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => assert!(false, "Transaction size cannot exceed protocol limit."),
            Err(err) => match err {
                Byron(MaxTxSizeExceeded) => (),
                _ => panic!("Unexpected error ({:?}).", err),
            },
        }
    }

    #[test]
    // The input to the transaction does not have a corresponding witness.
    fn missing_witness() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/byron1.tx"));
        let mut mtxp: MintedTxPayload = tx_from_cbor(&cbor_bytes);
        // Remove witness
        let new_witnesses: Witnesses = MaybeIndefArray::Def(Vec::new());
        let mut tx_buf: Vec<u8> = Vec::new();
        match encode(new_witnesses, &mut tx_buf) {
            Ok(_) => (),
            Err(err) => panic!("Unable to encode Tx ({:?}).", err),
        };
        mtxp.witness = Decode::decode(&mut Decoder::new(&tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_byron(&mtxp);
        let utxos: UTxOs = mk_utxo_for_single_input_tx(
            &mtxp.transaction,
            String::from(include_str!("../../test_data/byron1.address")),
            19999000000,
        );
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Byron(ByronProtParams {
                min_fees_const: 155381,
                min_fees_factor: 44,
                max_tx_size: 4096,
            }),
            prot_magic: 764824073,
            block_slot: 3241381,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => assert!(false, "All inputs must have a witness signature."),
            Err(err) => match err {
                Byron(MissingWitness) => (),
                _ => panic!("Unexpected error ({:?}).", err),
            },
        }
    }

    #[test]
    // The input to the transaction has an associated witness, but the signature is
    // wrong.
    fn wrong_signature() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/byron1.tx"));
        let mut mtxp: MintedTxPayload = tx_from_cbor(&cbor_bytes);
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
            Err(err) => panic!("Unable to encode Tx ({:?}).", err),
        };
        mtxp.witness = Decode::decode(&mut Decoder::new(&tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_byron(&mtxp);
        let utxos: UTxOs = mk_utxo_for_single_input_tx(
            &mtxp.transaction,
            String::from(include_str!("../../test_data/byron1.address")),
            19999000000,
        );
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Byron(ByronProtParams {
                min_fees_const: 155381,
                min_fees_factor: 44,
                max_tx_size: 4096,
            }),
            prot_magic: 764824073,
            block_slot: 3241381,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => assert!(false, "Witness signature should verify the transaction."),
            Err(err) => match err {
                Byron(WrongSignature) => (),
                _ => panic!("Unexpected error ({:?}).", err),
            },
        }
    }
}

// Helper functions.
fn add_to_utxo<'a>(utxos: &mut UTxOs<'a>, tx_in: TxIn, tx_out: TxOut) {
    let multi_era_in: MultiEraInput = MultiEraInput::Byron(Box::new(Cow::Owned(tx_in)));
    let multi_era_out: MultiEraOutput = MultiEraOutput::Byron(Box::new(Cow::Owned(tx_out)));
    utxos.insert(multi_era_in, multi_era_out);
}
