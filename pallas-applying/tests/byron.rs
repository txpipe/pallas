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

#[cfg(test)]
mod byron_tests {
    use super::*;

    const MAINNET_TX_HEX_CBOR: &str =
        "82839f8200d8185824825820da832fb5ef57df5b91817e9a7448d26e92552afb34f8ee5adb491b24bbe990d50e\
        ff9f8282d818584283581cdac5d9464c2140aeb0e3b6d69f0657e61f51e0c259fe19681ed268e8a101581e581c2\
        b5a44277e3543c08eae5d9d9d1146f43ba009fea6e285334f2549be001ae69c4d201b0000000172a84e408282d8\
        18584283581c2b8e5e0cb6495ec275872d1340b0581613b04a49a3c6f2f760ecaf95a101581e581cca3e553c9c6\
        3c5b66689e943ce7dad7d560ae84d7c2eaf21611c024c001ad27c159a1b00000003355d95efffa0818200d81858\
        85825840888cdf85991d85f2023423ba4c80d41570ebf1fc878c9f5731df1d20c64aecf3e8aa2bbafc9beba8ef3\
        3acb4d7e199b445229085718fba83b7f86ab6a3bcf782584063e34cf5fa6d8c0288630437fa5e151d93907e826e\
        66ba273145e3ee712930b6f446ff81cb91d7f0cb4ceccd0466ba9ab14448d7eab9fc480a122324bd80170e";

    fn cbor_to_bytes(input: &str) -> Vec<u8> {
        hex::decode(input).unwrap()
    }

    fn mainnet_tx_from_bytes_cbor<'a>(tx_cbor: &'a Vec<u8>) -> MintedTxPayload<'a> {
        pallas_codec::minicbor::decode::<MintedTxPayload>(&tx_cbor[..]).unwrap()
    }

    fn build_utxo<'a>(tx: &Tx) -> UTxOs<'a> {
        let mut tx_ins: Vec<TxIn> = tx.inputs.clone().to_vec();
        assert_eq!(tx_ins.len(), 1, "Unexpected number of inputs.");
        let tx_in: TxIn = tx_ins.pop().unwrap();
        let address_payload =
            "83581cff66e7549ee0706abe5ce63ba325f792f2c1145d918baf563db2b457a101581e581cca3e553c9c63\
            c5927480e7434620200eb3a162ef0b6cf6f671ba925100";
        let input_tx_out_addr: Address = match hex::decode(address_payload) {
            Ok(addr_bytes) => Address {
                payload: TagWrap(ByteVec::from(addr_bytes)),
                crc: 3430631884,
            },
            _ => panic!("Unable to decode input address."),
        };
        let tx_out: TxOut = TxOut {
            address: input_tx_out_addr,
            amount: 19999000000,
        };
        let mut utxos: UTxOs = new_utxos();
        add_to_utxo(&mut utxos, tx_in, tx_out);
        utxos
    }

    #[test]
    fn successful_mainnet_tx() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(MAINNET_TX_HEX_CBOR);
        let mtxp: MintedTxPayload = mainnet_tx_from_bytes_cbor(&cbor_bytes);
        let utxos: UTxOs = build_utxo(&mtxp.transaction);
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
            Err(err) => assert!(false, "Unexpected error ({:?}).", err),
        }
    }

    #[test]
    // Identical to successful_mainnet_tx, except that all inputs are removed.
    fn empty_ins() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(MAINNET_TX_HEX_CBOR);
        let mut mtxp: MintedTxPayload = mainnet_tx_from_bytes_cbor(&cbor_bytes);
        let utxos: UTxOs = build_utxo(&mtxp.transaction);
        // Clear the set of inputs in the transaction.
        let mut tx: Tx = (*mtxp.transaction).clone();
        tx.inputs = MaybeIndefArray::Def(Vec::new());
        let mut tx_buf: Vec<u8> = Vec::new();
        match encode(tx, &mut tx_buf) {
            Ok(_) => (),
            Err(err) => assert!(false, "Unable to encode Tx ({:?}).", err),
        };
        mtxp.transaction = Decode::decode(&mut Decoder::new(&tx_buf.as_slice()), &mut ()).unwrap();
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Byron(ByronProtParams {
                min_fees_const: 155381,
                min_fees_factor: 44,
                max_tx_size: 4096,
            }),
            prot_magic: 764824073,
        };
        match mk_byron_tx_and_validate(&mtxp.transaction, &mtxp.witness, &utxos, &env) {
            Ok(()) => assert!(false, "Inputs set should not be empty."),
            Err(err) => match err {
                ValidationError::TxInsEmpty => (),
                _ => assert!(false, "Unexpected error ({:?}).", err),
            },
        }
    }

    #[test]
    // Identical to successful_mainnet_tx, except that all outputs are removed.
    fn empty_outs() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(MAINNET_TX_HEX_CBOR);
        let mut mtxp: MintedTxPayload = mainnet_tx_from_bytes_cbor(&cbor_bytes);
        let utxos: UTxOs = build_utxo(&mtxp.transaction);
        // Clear the set of outputs in the transaction.
        let mut tx: Tx = (*mtxp.transaction).clone();
        tx.outputs = MaybeIndefArray::Def(Vec::new());
        let mut tx_buf: Vec<u8> = Vec::new();
        match encode(tx, &mut tx_buf) {
            Ok(_) => (),
            Err(err) => assert!(false, "Unable to encode Tx ({:?}).", err),
        };
        mtxp.transaction = Decode::decode(&mut Decoder::new(&tx_buf.as_slice()), &mut ()).unwrap();
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Byron(ByronProtParams {
                min_fees_const: 155381,
                min_fees_factor: 44,
                max_tx_size: 4096,
            }),
            prot_magic: 764824073,
        };
        match mk_byron_tx_and_validate(&mtxp.transaction, &mtxp.witness, &utxos, &env) {
            Ok(()) => assert!(false, "Outputs set should not be empty."),
            Err(err) => match err {
                ValidationError::TxOutsEmpty => (),
                _ => assert!(false, "Unexpected error ({:?}).", err),
            },
        }
    }

    #[test]
    // The transaction is valid, but the UTxO set is empty.
    fn unfound_utxo() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(MAINNET_TX_HEX_CBOR);
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
            Ok(()) => assert!(false, "All inputs must be within the UTxO set."),
            Err(err) => match err {
                ValidationError::InputMissingInUTxO => (),
                _ => assert!(false, "Unexpected error ({:?}).", err),
            },
        }
    }

    #[test]
    // All lovelace in one of the outputs was removed.
    fn output_without_lovelace() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(MAINNET_TX_HEX_CBOR);
        let mut mtxp: MintedTxPayload = mainnet_tx_from_bytes_cbor(&cbor_bytes);
        let utxos: UTxOs = build_utxo(&mtxp.transaction);
        // Remove lovelace from output.
        let mut tx: Tx = (*mtxp.transaction).clone();
        let altered_tx_out: TxOut = TxOut {
            address: tx.outputs[0].address.clone(),
            amount: 0,
        };
        let mut new_tx_outs: Vec<TxOut> = Vec::new();
        new_tx_outs.push(tx.outputs[1].clone());
        new_tx_outs.push(altered_tx_out);
        tx.outputs = MaybeIndefArray::Indef(new_tx_outs);
        let mut tx_buf: Vec<u8> = Vec::new();
        match encode(tx, &mut tx_buf) {
            Ok(_) => (),
            Err(err) => assert!(false, "Unable to encode Tx ({:?}).", err),
        };
        mtxp.transaction = Decode::decode(&mut Decoder::new(&tx_buf.as_slice()), &mut ()).unwrap();
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Byron(ByronProtParams {
                min_fees_const: 155381,
                min_fees_factor: 44,
                max_tx_size: 4096,
            }),
            prot_magic: 764824073,
        };
        match mk_byron_tx_and_validate(&mtxp.transaction, &mtxp.witness, &utxos, &env) {
            Ok(()) => assert!(false, "All outputs must contain lovelace."),
            Err(err) => match err {
                ValidationError::OutputWithoutLovelace => (),
                _ => assert!(false, "Unexpected error ({:?}).", err),
            },
        }
    }

    #[test]
    // Expected fees are increased by increasing the protocol parameters.
    fn not_enough_fees() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(MAINNET_TX_HEX_CBOR);
        let mtxp: MintedTxPayload = mainnet_tx_from_bytes_cbor(&cbor_bytes);
        let utxos: UTxOs = build_utxo(&mtxp.transaction);
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Byron(ByronProtParams {
                min_fees_const: 1000,
                min_fees_factor: 1000,
                max_tx_size: 4096,
            }),
            prot_magic: 764824073,
        };
        match mk_byron_tx_and_validate(&mtxp.transaction, &mtxp.witness, &utxos, &env) {
            Ok(()) => assert!(false, "Fees should not be below minimum."),
            Err(err) => match err {
                ValidationError::FeesBelowMin => (),
                _ => assert!(false, "Unexpected error ({:?}).", err),
            },
        }
    }

    #[test]
    // Tx size limit set by protocol parameters is established at 0.
    fn tx_size_exceeds_max() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(MAINNET_TX_HEX_CBOR);
        let mtxp: MintedTxPayload = mainnet_tx_from_bytes_cbor(&cbor_bytes);
        let utxos: UTxOs = build_utxo(&mtxp.transaction);
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Byron(ByronProtParams {
                min_fees_const: 155381,
                min_fees_factor: 44,
                max_tx_size: 0,
            }),
            prot_magic: 764824073,
        };
        match mk_byron_tx_and_validate(&mtxp.transaction, &mtxp.witness, &utxos, &env) {
            Ok(()) => assert!(false, "Transaction size cannot exceed protocol limit."),
            Err(err) => match err {
                ValidationError::MaxTxSizeExceeded => (),
                _ => assert!(false, "Unexpected error ({:?}).", err),
            },
        }
    }

    #[test]
    // The input to the transaction does not have a corresponding witness.
    fn missing_witness() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(MAINNET_TX_HEX_CBOR);
        let mut mtxp: MintedTxPayload = mainnet_tx_from_bytes_cbor(&cbor_bytes);
        let utxos: UTxOs = build_utxo(&mtxp.transaction);
        // Remove witness
        let new_witnesses: Witnesses = MaybeIndefArray::Def(Vec::new());
        let mut tx_buf: Vec<u8> = Vec::new();
        match encode(new_witnesses, &mut tx_buf) {
            Ok(_) => (),
            Err(err) => assert!(false, "Unable to encode Tx ({:?}).", err),
        };
        mtxp.witness = Decode::decode(&mut Decoder::new(&tx_buf.as_slice()), &mut ()).unwrap();
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Byron(ByronProtParams {
                min_fees_const: 155381,
                min_fees_factor: 44,
                max_tx_size: 4096,
            }),
            prot_magic: 764824073,
        };
        match mk_byron_tx_and_validate(&mtxp.transaction, &mtxp.witness, &utxos, &env) {
            Ok(()) => assert!(false, "All inputs must have a witness signature."),
            Err(err) => match err {
                ValidationError::MissingWitness => (),
                _ => assert!(false, "Unexpected error ({:?}).", err),
            },
        }
    }

    #[test]
    // The input to the transaction has an associated witness, but the signature is wrong.
    fn wrong_signature() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(MAINNET_TX_HEX_CBOR);
        let mut mtxp: MintedTxPayload = mainnet_tx_from_bytes_cbor(&cbor_bytes);
        let utxos: UTxOs = build_utxo(&mtxp.transaction);
        // Modify signature in witness
        let new_wit: Twit = match mtxp.witness[0].clone() {
            Twit::PkWitness(CborWrap((pk, _))) => {
                Twit::PkWitness(CborWrap((pk, [0u8; 64].to_vec().into())))
            }
            _ => unreachable!(),
        };
        let mut new_witnesses_vec = Vec::new();
        new_witnesses_vec.push(new_wit);
        let new_witnesses: Witnesses = MaybeIndefArray::Def(new_witnesses_vec);
        let mut tx_buf: Vec<u8> = Vec::new();
        match encode(new_witnesses, &mut tx_buf) {
            Ok(_) => (),
            Err(err) => assert!(false, "Unable to encode Tx ({:?}).", err),
        };
        mtxp.witness = Decode::decode(&mut Decoder::new(&tx_buf.as_slice()), &mut ()).unwrap();
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Byron(ByronProtParams {
                min_fees_const: 155381,
                min_fees_factor: 44,
                max_tx_size: 4096,
            }),
            prot_magic: 764824073,
        };
        match mk_byron_tx_and_validate(&mtxp.transaction, &mtxp.witness, &utxos, &env) {
            Ok(()) => assert!(false, "Witness signature should verify the transaction."),
            Err(err) => match err {
                ValidationError::WrongSignature => (),
                _ => assert!(false, "Unexpected error ({:?}).", err),
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

// pallas_applying::validate takes a MultiEraTx, not a Tx and a Witnesses. To be able to build a
// MultiEraTx from a Tx and a Witnesses, we need to encode each of them and then decode them into
// KeepRaw<Tx> and KeepRaw<Witnesses> values, respectively, to be able to make the MultiEraTx value.
fn mk_byron_tx_and_validate(
    tx: &Tx,
    wits: &Witnesses,
    utxos: &UTxOs,
    env: &Environment,
) -> ValidationResult {
    let mut tx_buf: Vec<u8> = Vec::new();
    match encode(tx, &mut tx_buf) {
        Ok(_) => (),
        Err(err) => assert!(false, "Unable to encode Tx ({:?}).", err),
    };
    let kptx: KeepRaw<Tx> = match Decode::decode(&mut Decoder::new(&tx_buf.as_slice()), &mut ()) {
        Ok(kp) => kp,
        Err(err) => panic!("Unable to decode Tx ({:?}).", err),
    };

    let mut wit_buf: Vec<u8> = Vec::new();
    match encode(wits, &mut wit_buf) {
        Ok(_) => (),
        Err(err) => assert!(false, "Unable to encode Witnesses ({:?}).", err),
    };
    let kpwit: KeepRaw<Witnesses> =
        match Decode::decode(&mut Decoder::new(&wit_buf.as_slice()), &mut ()) {
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
