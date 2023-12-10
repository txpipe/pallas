use std::{borrow::Cow, vec::Vec};

use pallas_applying::{
    types::{ByronProtParams, Environment, MultiEraProtParams, ValidationError},
    validate, UTxOs, ValidationResult,
};
use pallas_codec::{
    minicbor::{
        bytes::ByteVec,
        decode::{Decode, Decoder},
        encode,
    },
    utils::{CborWrap, KeepRaw, MaybeIndefArray, TagWrap},
};
use pallas_primitives::byron::{Address, MintedTxPayload, Twit, Tx, TxIn, TxOut, Witnesses};
use pallas_traverse::{MultiEraInput, MultiEraOutput, MultiEraTx};

// Helper functions.
fn add_to_utxo(utxos: &mut UTxOs, tx_in: TxIn, tx_out: TxOut) {
    let multi_era_in: MultiEraInput = MultiEraInput::Byron(Box::new(Cow::Owned(tx_in)));
    let multi_era_out: MultiEraOutput = MultiEraOutput::Byron(Box::new(Cow::Owned(tx_out)));
    utxos.insert(multi_era_in, multi_era_out);
}

// pallas_applying::validate takes a MultiEraTx, not a Tx and a Witnesses. To be
// able to build a MultiEraTx from a Tx and a Witnesses, we need to encode each
// of them and then decode them into KeepRaw<Tx> and KeepRaw<Witnesses> values,
// respectively, to be able to make the MultiEraTx value.
fn mk_byron_tx_and_validate(
    tx: &Tx,
    wits: &Witnesses,
    utxos: &UTxOs,
    env: &Environment,
) -> ValidationResult {
    let mut tx_buf: Vec<u8> = Vec::new();

    match encode(tx, &mut tx_buf) {
        Ok(_) => (),
        Err(err) => panic!("Unable to encode Tx ({:?}).", err),
    };

    let kptx: KeepRaw<Tx> = match Decode::decode(&mut Decoder::new(tx_buf.as_slice()), &mut ()) {
        Ok(kp) => kp,
        Err(err) => panic!("Unable to decode Tx ({:?}).", err),
    };

    let mut wit_buf: Vec<u8> = Vec::new();

    match encode(wits, &mut wit_buf) {
        Ok(_) => (),
        Err(err) => panic!("Unable to encode Witnesses ({:?}).", err),
    };

    let kpwit: KeepRaw<Witnesses> =
        match Decode::decode(&mut Decoder::new(wit_buf.as_slice()), &mut ()) {
            Ok(kp) => kp,
            Err(err) => panic!("Unable to decode Witnesses ({:?}).", err),
        };

    let mtxp: MintedTxPayload = MintedTxPayload {
        transaction: kptx,
        witness: kpwit,
    };

    let metx: MultiEraTx = MultiEraTx::from_byron(&mtxp);

    validate(&metx, utxos, env)
}

fn new_utxos<'a>() -> UTxOs<'a> {
    UTxOs::new()
}

#[cfg(test)]
mod byron_tests {
    use super::*;

    fn cbor_to_bytes(input: &str) -> Vec<u8> {
        hex::decode(input).unwrap()
    }

    fn mainnet_tx_from_bytes_cbor(tx_cbor: &[u8]) -> MintedTxPayload<'_> {
        pallas_codec::minicbor::decode::<MintedTxPayload>(tx_cbor).unwrap()
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
        let mut utxos: UTxOs = new_utxos();
        add_to_utxo(&mut utxos, tx_in, tx_out);
        utxos
    }

    #[test]
    fn successful_mainnet_tx_with_genesis_utxos() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/byron2.tx"));
        let mtxp: MintedTxPayload = mainnet_tx_from_bytes_cbor(&cbor_bytes);
        let utxos: UTxOs = mk_utxo_for_single_input_tx(
            &mtxp.transaction,
            String::from(include_str!("../../test_data/byron2.address")),
            // The number of lovelace in this input is irrelevant, since no fees have to be paid
            // for this transaction.
            1,
        );
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Byron(ByronProtParams {
                min_fees_const: 155381,
                min_fees_factor: 44,
                max_tx_size: 4096,
            }),
            prot_magic: 764824073,
        };
        match mk_byron_tx_and_validate(&mtxp.transaction, &mtxp.witness, &utxos, &env) {
            Ok(()) => (),
            Err(err) => panic!("Unexpected error ({:?}).", err),
        }
    }

    #[test]
    fn successful_mainnet_tx() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/byron1.tx"));
        let mtxp: MintedTxPayload = mainnet_tx_from_bytes_cbor(&cbor_bytes);
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
        };
        match mk_byron_tx_and_validate(&mtxp.transaction, &mtxp.witness, &utxos, &env) {
            Ok(()) => (),
            Err(err) => panic!("Unexpected error ({:?}).", err),
        }
    }

    #[test]
    // Identical to successful_mainnet_tx, except that all inputs are removed.
    fn empty_ins() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/byron1.tx"));
        let mut mtxp: MintedTxPayload = mainnet_tx_from_bytes_cbor(&cbor_bytes);
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
        mtxp.transaction = Decode::decode(&mut Decoder::new(tx_buf.as_slice()), &mut ()).unwrap();
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Byron(ByronProtParams {
                min_fees_const: 155381,
                min_fees_factor: 44,
                max_tx_size: 4096,
            }),
            prot_magic: 764824073,
        };
        match mk_byron_tx_and_validate(&mtxp.transaction, &mtxp.witness, &utxos, &env) {
            Ok(()) => panic!("Inputs set should not be empty."),
            Err(err) => match err {
                ValidationError::TxInsEmpty => (),
                _ => panic!("Unexpected error ({:?}).", err),
            },
        }
    }

    #[test]
    // Identical to successful_mainnet_tx, except that all outputs are removed.
    fn empty_outs() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/byron1.tx"));
        let mut mtxp: MintedTxPayload = mainnet_tx_from_bytes_cbor(&cbor_bytes);
        let utxos: UTxOs = mk_utxo_for_single_input_tx(
            &mtxp.transaction,
            String::from(include_str!("../../test_data/byron1.address")),
            19999000000,
        );
        // Clear the set of outputs in the transaction.
        let mut tx: Tx = (*mtxp.transaction).clone();
        tx.outputs = MaybeIndefArray::Def(Vec::new());
        let mut tx_buf: Vec<u8> = Vec::new();
        match encode(tx, &mut tx_buf) {
            Ok(_) => (),
            Err(err) => panic!("Unable to encode Tx ({:?}).", err),
        };
        mtxp.transaction = Decode::decode(&mut Decoder::new(tx_buf.as_slice()), &mut ()).unwrap();
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Byron(ByronProtParams {
                min_fees_const: 155381,
                min_fees_factor: 44,
                max_tx_size: 4096,
            }),
            prot_magic: 764824073,
        };
        match mk_byron_tx_and_validate(&mtxp.transaction, &mtxp.witness, &utxos, &env) {
            Ok(()) => panic!("Outputs set should not be empty."),
            Err(err) => match err {
                ValidationError::TxOutsEmpty => (),
                _ => panic!("Unexpected error ({:?}).", err),
            },
        }
    }

    #[test]
    // The transaction is valid, but the UTxO set is empty.
    fn unfound_utxo() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/byron1.tx"));
        let mtxp: MintedTxPayload = mainnet_tx_from_bytes_cbor(&cbor_bytes);
        let utxos: UTxOs = UTxOs::new();
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Byron(ByronProtParams {
                min_fees_const: 155381,
                min_fees_factor: 44,
                max_tx_size: 4096,
            }),
            prot_magic: 764824073,
        };
        match mk_byron_tx_and_validate(&mtxp.transaction, &mtxp.witness, &utxos, &env) {
            Ok(()) => panic!("All inputs must be within the UTxO set."),
            Err(err) => match err {
                ValidationError::InputMissingInUTxO => (),
                _ => panic!("Unexpected error ({:?}).", err),
            },
        }
    }

    #[test]
    // All lovelace in one of the outputs was removed.
    fn output_without_lovelace() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/byron1.tx"));
        let mut mtxp: MintedTxPayload = mainnet_tx_from_bytes_cbor(&cbor_bytes);
        let utxos: UTxOs = mk_utxo_for_single_input_tx(
            &mtxp.transaction,
            String::from(include_str!("../../test_data/byron1.address")),
            19999000000,
        );
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
        mtxp.transaction = Decode::decode(&mut Decoder::new(tx_buf.as_slice()), &mut ()).unwrap();
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Byron(ByronProtParams {
                min_fees_const: 155381,
                min_fees_factor: 44,
                max_tx_size: 4096,
            }),
            prot_magic: 764824073,
        };
        match mk_byron_tx_and_validate(&mtxp.transaction, &mtxp.witness, &utxos, &env) {
            Ok(()) => panic!("All outputs must contain lovelace."),
            Err(err) => match err {
                ValidationError::OutputWithoutLovelace => (),
                _ => panic!("Unexpected error ({:?}).", err),
            },
        }
    }

    #[test]
    // Expected fees are increased by increasing the protocol parameters.
    fn not_enough_fees() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/byron1.tx"));
        let mtxp: MintedTxPayload = mainnet_tx_from_bytes_cbor(&cbor_bytes);
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
        };
        match mk_byron_tx_and_validate(&mtxp.transaction, &mtxp.witness, &utxos, &env) {
            Ok(()) => panic!("Fees should not be below minimum."),
            Err(err) => match err {
                ValidationError::FeesBelowMin => (),
                _ => panic!("Unexpected error ({:?}).", err),
            },
        }
    }

    #[test]
    // Tx size limit set by protocol parameters is established at 0.
    fn tx_size_exceeds_max() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/byron1.tx"));
        let mtxp: MintedTxPayload = mainnet_tx_from_bytes_cbor(&cbor_bytes);
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
        };
        match mk_byron_tx_and_validate(&mtxp.transaction, &mtxp.witness, &utxos, &env) {
            Ok(()) => panic!("Transaction size cannot exceed protocol limit."),
            Err(err) => match err {
                ValidationError::MaxTxSizeExceeded => (),
                _ => panic!("Unexpected error ({:?}).", err),
            },
        }
    }

    #[test]
    // The input to the transaction does not have a corresponding witness.
    fn missing_witness() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/byron1.tx"));
        let mut mtxp: MintedTxPayload = mainnet_tx_from_bytes_cbor(&cbor_bytes);
        let utxos: UTxOs = mk_utxo_for_single_input_tx(
            &mtxp.transaction,
            String::from(include_str!("../../test_data/byron1.address")),
            19999000000,
        );
        // Remove witness
        let new_witnesses: Witnesses = MaybeIndefArray::Def(Vec::new());
        let mut tx_buf: Vec<u8> = Vec::new();
        match encode(new_witnesses, &mut tx_buf) {
            Ok(_) => (),
            Err(err) => panic!("Unable to encode Tx ({:?}).", err),
        };
        mtxp.witness = Decode::decode(&mut Decoder::new(tx_buf.as_slice()), &mut ()).unwrap();
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Byron(ByronProtParams {
                min_fees_const: 155381,
                min_fees_factor: 44,
                max_tx_size: 4096,
            }),
            prot_magic: 764824073,
        };
        match mk_byron_tx_and_validate(&mtxp.transaction, &mtxp.witness, &utxos, &env) {
            Ok(()) => panic!("All inputs must have a witness signature."),
            Err(err) => match err {
                ValidationError::MissingWitness => (),
                _ => panic!("Unexpected error ({:?}).", err),
            },
        }
    }

    #[test]
    // The input to the transaction has an associated witness, but the signature is
    // wrong.
    fn wrong_signature() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/byron1.tx"));
        let mut mtxp: MintedTxPayload = mainnet_tx_from_bytes_cbor(&cbor_bytes);
        let utxos: UTxOs = mk_utxo_for_single_input_tx(
            &mtxp.transaction,
            String::from(include_str!("../../test_data/byron1.address")),
            19999000000,
        );
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

        mtxp.witness = Decode::decode(&mut Decoder::new(tx_buf.as_slice()), &mut ()).unwrap();

        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Byron(ByronProtParams {
                min_fees_const: 155381,
                min_fees_factor: 44,
                max_tx_size: 4096,
            }),
            prot_magic: 764824073,
        };

        match mk_byron_tx_and_validate(&mtxp.transaction, &mtxp.witness, &utxos, &env) {
            Ok(()) => panic!("Witness signature should verify the transaction."),
            Err(err) => match err {
                ValidationError::WrongSignature => (),
                _ => panic!("Unexpected error ({:?}).", err),
            },
        }
    }
}
