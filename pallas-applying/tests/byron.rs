use rand::Rng;
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
    utils::{CborWrap, EmptyMap, KeepRaw, MaybeIndefArray, TagWrap},
};
use pallas_crypto::hash::Hash;
use pallas_primitives::byron::{
    Address, Attributes, MintedTxPayload as ByronTxPayload, Tx as ByronTx, TxId as ByronTxId,
    TxIn as ByronTxIn, TxOut as ByronTxOut, Witnesses as ByronWitnesses,
};
use pallas_traverse::{MultiEraInput, MultiEraOutput, MultiEraTx};

#[cfg(test)]
mod byron_tests {
    use super::*;

    #[test]
    // Note that:
    //      i)   the transaction input contains 100000 lovelace,
    //      ii)  the minimum_fee_constant protocol parameter is 7,
    //      iii) the minimum_fee_factor protocol parameter is 11, and
    //      iv)  the size of the transaction is 82 bytes—it is easy to verify
    //              that 82 == pallas_applying::byron::get_tx_size(tx).
    // The expected fees are therefore 7 + 11 * 82 = 909 lovelace, which is why
    // the output contains 100000 - 909 = 99091 lovelace.
    fn successful_case() {
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Byron(ByronProtParams {
                min_fees_const: 7,
                min_fees_factor: 11,
                max_tx_size: 82,
            }),
            prot_magic: 764824073,
        };
        let mut tx_ins: ByronTxIns = empty_tx_ins();
        let tx_in: ByronTxIn = new_tx_in(rand_tx_id(), 3);
        add_byron_tx_in(&mut tx_ins, &tx_in);
        let mut tx_outs: ByronTxOuts = new_tx_outs();
        let tx_out_addr: Address = new_addr(rand_addr_payload(), 0);
        let tx_out: ByronTxOut = new_tx_out(tx_out_addr, 99091);
        add_tx_out(&mut tx_outs, &tx_out);
        let mut utxos: UTxOs = new_utxos();
        // input_tx_out is the ByronTxOut associated with tx_in.
        let input_tx_out_addr: Address = new_addr(rand_addr_payload(), 0);
        let input_tx_out: ByronTxOut = new_tx_out(input_tx_out_addr, 100000);
        add_to_utxo(&mut utxos, tx_in, input_tx_out);
        let validation_result = mk_byron_tx_and_validate(
            &new_tx(tx_ins, tx_outs, empty_attributes()),
            &empty_witnesses(),
            &utxos,
            &env,
        );
        match validation_result {
            Ok(()) => (),
            Err(err) => assert!(false, "Unexpected error ({:?}).", err),
        }
    }

    #[test]
    // Similar to successful_case, except that no inputs are added to the
    // transaction, which should raise a ValidationError:TxInsEmpty error.
    fn empty_ins() {
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Byron(ByronProtParams {
                min_fees_const: 7,
                min_fees_factor: 11,
                max_tx_size: 82,
            }),
            prot_magic: 764824073,
        };
        let tx_ins: ByronTxIns = empty_tx_ins();
        // Note: tx_in is not added to tx_ins, it is only added to the UTxOs set
        let tx_in: ByronTxIn = new_tx_in(rand_tx_id(), 3);
        let mut tx_outs: ByronTxOuts = new_tx_outs();
        let tx_out_addr: Address = new_addr(rand_addr_payload(), 0);
        let tx_out: ByronTxOut = new_tx_out(tx_out_addr, 99091);
        add_tx_out(&mut tx_outs, &tx_out);
        let mut utxos: UTxOs = new_utxos();
        let input_tx_out_addr: Address = new_addr(rand_addr_payload(), 0);
        let input_tx_out: ByronTxOut = new_tx_out(input_tx_out_addr, 100000);
        add_to_utxo(&mut utxos, tx_in, input_tx_out);
        let validation_result = mk_byron_tx_and_validate(
            &new_tx(tx_ins, tx_outs, empty_attributes()),
            &empty_witnesses(),
            &utxos,
            &env,
        );
        match validation_result {
            Ok(()) => assert!(false, "Inputs set should not be empty."),
            Err(err) => match err {
                ValidationError::TxInsEmpty => (),
                _ => assert!(false, "Unexpected error ({:?}).", err),
            },
        }
    }

    #[test]
    // Similar to empty_ins, except that this time no outputs are added to the
    // transaction, which should raise a ValidationError:TxOutsEmpty error.
    fn empty_outs() {
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Byron(ByronProtParams {
                min_fees_const: 7,
                min_fees_factor: 11,
                max_tx_size: 82,
            }),
            prot_magic: 764824073,
        };
        let mut tx_ins: ByronTxIns = empty_tx_ins();
        let tx_in: ByronTxIn = new_tx_in(rand_tx_id(), 3);
        add_byron_tx_in(&mut tx_ins, &tx_in);
        let tx_outs: ByronTxOuts = new_tx_outs();
        let mut utxos: UTxOs = new_utxos();
        let input_tx_out_addr: Address = new_addr(rand_addr_payload(), 0);
        let input_tx_out: ByronTxOut = new_tx_out(input_tx_out_addr, 100000);
        add_to_utxo(&mut utxos, tx_in, input_tx_out);
        let validation_result = mk_byron_tx_and_validate(
            &new_tx(tx_ins, tx_outs, empty_attributes()),
            &empty_witnesses(),
            &utxos,
            &env,
        );
        match validation_result {
            Ok(()) => assert!(false, "Outputs set should not be empty."),
            Err(err) => match err {
                ValidationError::TxOutsEmpty => (),
                _ => assert!(false, "Unexpected error ({:?}).", err),
            },
        }
    }

    #[test]
    // The UTxO set does not contain an entry for the single input to this transaction. This
    // represents the situation where a transaction tries to spend a non-existent UTxO (e.g., one
    // which has already been spent).
    fn unfound_utxo() {
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Byron(ByronProtParams {
                min_fees_const: 7,
                min_fees_factor: 11,
                max_tx_size: 82,
            }),
            prot_magic: 764824073,
        };
        let mut tx_ins: ByronTxIns = empty_tx_ins();
        let tx_in: ByronTxIn = new_tx_in(rand_tx_id(), 3);
        add_byron_tx_in(&mut tx_ins, &tx_in);
        let mut tx_outs: ByronTxOuts = new_tx_outs();
        let tx_out_addr: Address = new_addr(rand_addr_payload(), 0);
        let tx_out: ByronTxOut = new_tx_out(tx_out_addr, 99091);
        add_tx_out(&mut tx_outs, &tx_out);
        // Note: utxos is empty, hence the only input to this transaction will not be found, for
        // which an error should be raised.
        let utxos: UTxOs = new_utxos();
        let validation_result = mk_byron_tx_and_validate(
            &new_tx(tx_ins, tx_outs, empty_attributes()),
            &empty_witnesses(),
            &utxos,
            &env,
        );
        match validation_result {
            Ok(()) => assert!(false, "All inputs must be within the UTxO set."),
            Err(err) => match err {
                ValidationError::InputMissingInUTxO => (),
                _ => assert!(false, "Unexpected error ({:?}).", err),
            },
        }
    }

    #[test]
    // All outputs must contain a non-zero number of lovelaces.
    fn output_without_lovelace() {
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Byron(ByronProtParams {
                min_fees_const: 7,
                min_fees_factor: 11,
                max_tx_size: 82,
            }),
            prot_magic: 764824073,
        };
        let mut tx_ins: ByronTxIns = empty_tx_ins();
        let tx_in: ByronTxIn = new_tx_in(rand_tx_id(), 3);
        add_byron_tx_in(&mut tx_ins, &tx_in);
        let mut tx_outs: ByronTxOuts = new_tx_outs();
        let tx_out_addr: Address = new_addr(rand_addr_payload(), 0);
        // Note that this output has 0 lovelaces.
        let tx_out: ByronTxOut = new_tx_out(tx_out_addr, 0);
        add_tx_out(&mut tx_outs, &tx_out);
        let mut utxos: UTxOs = new_utxos();
        // input_tx_out is the ByronTxOut associated with tx_in.
        let input_tx_out_addr: Address = new_addr(rand_addr_payload(), 0);
        let input_tx_out: ByronTxOut = new_tx_out(input_tx_out_addr, 100000);
        add_to_utxo(&mut utxos, tx_in, input_tx_out);
        let validation_result = mk_byron_tx_and_validate(
            &new_tx(tx_ins, tx_outs, empty_attributes()),
            &empty_witnesses(),
            &utxos,
            &env,
        );
        match validation_result {
            Ok(()) => assert!(false, "All outputs must contain lovelace."),
            Err(err) => match err {
                ValidationError::OutputWithoutLovelace => (),
                _ => assert!(false, "Unexpected error ({:?}).", err),
            },
        }
    }

    #[test]
    // The following is almost exactly like successful_case, valid in every sense except that
    // fees are exactly 1 lovelace below the minimum allowed by protocol (refer to successful_case
    // for a deeper analysis on fees).
    fn not_enough_fees() {
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Byron(ByronProtParams {
                min_fees_const: 7,
                min_fees_factor: 11,
                max_tx_size: 82,
            }),
            prot_magic: 764824073,
        };
        let mut tx_ins: ByronTxIns = empty_tx_ins();
        let tx_in: ByronTxIn = new_tx_in(rand_tx_id(), 3);
        add_byron_tx_in(&mut tx_ins, &tx_in);
        let mut tx_outs: ByronTxOuts = new_tx_outs();
        let tx_out_addr: Address = new_addr(rand_addr_payload(), 0);
        // Note: in successful_case, tx_out contains 99091. Hence, the successful transaction pays
        // 1 extra lovelace as fees than the one here.
        let tx_out: ByronTxOut = new_tx_out(tx_out_addr, 99092);
        add_tx_out(&mut tx_outs, &tx_out);
        let mut utxos: UTxOs = new_utxos();
        // input_tx_out is the ByronTxOut associated with tx_in.
        let input_tx_out_addr: Address = new_addr(rand_addr_payload(), 0);
        let input_tx_out: ByronTxOut = new_tx_out(input_tx_out_addr, 100000);
        add_to_utxo(&mut utxos, tx_in, input_tx_out);
        let validation_result = mk_byron_tx_and_validate(
            &new_tx(tx_ins, tx_outs, empty_attributes()),
            &empty_witnesses(),
            &utxos,
            &env,
        );
        match validation_result {
            Ok(()) => assert!(false, "Fees should not be below minimum."),
            Err(err) => match err {
                ValidationError::FeesBelowMin => (),
                _ => assert!(false, "Unexpected error ({:?}).", err),
            },
        }
    }

    #[test]
    // The following is exactly the same as successful_case, except that the protocol parameters
    // enforce that transaction sizes do not exceed the 81-byte limit. But, as explained in
    // successful_case, this transaction is 82 bytes big, for which an error should be raised.
    fn tx_size_exceeds_max() {
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Byron(ByronProtParams {
                min_fees_const: 7,
                min_fees_factor: 11,
                max_tx_size: 81,
            }),
            prot_magic: 764824073,
        };
        let mut tx_ins: ByronTxIns = empty_tx_ins();
        let tx_in: ByronTxIn = new_tx_in(rand_tx_id(), 3);
        add_byron_tx_in(&mut tx_ins, &tx_in);
        let mut tx_outs: ByronTxOuts = new_tx_outs();
        let tx_out_addr: Address = new_addr(rand_addr_payload(), 0);
        let tx_out: ByronTxOut = new_tx_out(tx_out_addr, 99091);
        add_tx_out(&mut tx_outs, &tx_out);
        let mut utxos: UTxOs = new_utxos();
        // input_tx_out is the ByronTxOut associated with tx_in.
        let input_tx_out_addr: Address = new_addr(rand_addr_payload(), 0);
        let input_tx_out: ByronTxOut = new_tx_out(input_tx_out_addr, 100000);
        add_to_utxo(&mut utxos, tx_in, input_tx_out);
        let validation_result = mk_byron_tx_and_validate(
            &new_tx(tx_ins, tx_outs, empty_attributes()),
            &empty_witnesses(),
            &utxos,
            &env,
        );
        match validation_result {
            Ok(()) => assert!(false, "Transaction size cannot exceed protocol limit."),
            Err(err) => match err {
                ValidationError::MaxTxSizeExceeded => (),
                _ => assert!(false, "Unexpected error ({:?}).", err),
            },
        }
    }
}

// Types aliases.
type ByronTxIns = MaybeIndefArray<ByronTxIn>;
type ByronTxOuts = MaybeIndefArray<ByronTxOut>;

// Helper functions.
fn empty_tx_ins() -> ByronTxIns {
    MaybeIndefArray::Def(Vec::new())
}

fn rand_tx_id() -> ByronTxId {
    let mut rng = rand::thread_rng();
    let mut bytes = [0u8; 32];
    for elem in bytes.iter_mut() {
        *elem = rng.gen();
    }
    Hash::new(bytes)
}

fn new_tx_in(tx_id: ByronTxId, index: u32) -> ByronTxIn {
    ByronTxIn::Variant0(CborWrap((tx_id, index)))
}

fn add_byron_tx_in(ins: &mut ByronTxIns, new_in: &ByronTxIn) {
    match ins {
        MaybeIndefArray::Def(vec) | MaybeIndefArray::Indef(vec) => vec.push(new_in.clone()),
    }
}

fn new_tx_outs() -> ByronTxOuts {
    MaybeIndefArray::Def(Vec::new())
}

fn rand_addr_payload() -> TagWrap<ByteVec, 24> {
    let mut rng = rand::thread_rng();
    let mut bytes = [0u8; 24];
    for elem in bytes.iter_mut() {
        *elem = rng.gen();
    }
    TagWrap::<ByteVec, 24>::new(ByteVec::from(bytes.to_vec()))
}

fn new_addr(payload: TagWrap<ByteVec, 24>, crc: u32) -> Address {
    Address {
        payload: payload,
        crc: crc,
    }
}

fn new_tx_out(address: Address, amount: u64) -> ByronTxOut {
    ByronTxOut {
        address: address,
        amount: amount,
    }
}

fn add_tx_out(outs: &mut ByronTxOuts, new_out: &ByronTxOut) {
    match outs {
        MaybeIndefArray::Def(vec) | MaybeIndefArray::Indef(vec) => vec.push(new_out.clone()),
    }
}

fn add_to_utxo<'a>(utxos: &mut UTxOs<'a>, tx_in: ByronTxIn, tx_out: ByronTxOut) {
    let multi_era_in: MultiEraInput = MultiEraInput::Byron(Box::new(Cow::Owned(tx_in)));
    let multi_era_out: MultiEraOutput = MultiEraOutput::Byron(Box::new(Cow::Owned(tx_out)));
    utxos.insert(multi_era_in, multi_era_out);
}

fn empty_attributes() -> Attributes {
    EmptyMap
}

// pallas_applying::validate takes a MultiEraTx, not a ByronTx and a
// ByronWitnesses. To be able to build a MultiEraTx from a ByronTx and a
// ByronWitnesses, we need to encode each of them and then decode them into
// KeepRaw<ByronTx> and KeepRaw<ByronWitnesses> values, respectively.
fn mk_byron_tx_and_validate(
    btx: &ByronTx,
    bwit: &ByronWitnesses,
    utxos: &UTxOs,
    env: &Environment,
) -> ValidationResult {
    // Encode btx and decode into a KeepRaw<ByronTx> value.
    let mut btx_buf: Vec<u8> = Vec::new();
    match encode(btx, &mut btx_buf) {
        Ok(_) => (),
        Err(err) => assert!(false, "Unable to encode ByronTx ({:?}).", err),
    };
    let kpbtx: KeepRaw<ByronTx> =
        match Decode::decode(&mut Decoder::new(&btx_buf.as_slice()), &mut ()) {
            Ok(kp) => kp,
            Err(err) => panic!("Unable to decode ByronTx ({:?}).", err),
        };

    // Encode bwit and decode into a KeepRaw<ByronWitnesses> value.
    let mut wit_buf: Vec<u8> = Vec::new();
    match encode(bwit, &mut wit_buf) {
        Ok(_) => (),
        Err(err) => assert!(false, "Unable to encode ByronWitnesses ({:?}).", err),
    };
    let kpbwit: KeepRaw<ByronWitnesses> =
        match Decode::decode(&mut Decoder::new(&wit_buf.as_slice()), &mut ()) {
            Ok(kp) => kp,
            Err(err) => panic!("Unable to decode ByronWitnesses ({:?}).", err),
        };

    let mtxp: ByronTxPayload = ByronTxPayload {
        transaction: kpbtx,
        witness: kpbwit,
    };
    let metx: MultiEraTx = MultiEraTx::from_byron(&mtxp);
    validate(&metx, utxos, env)
}

fn new_tx(ins: ByronTxIns, outs: ByronTxOuts, attrs: Attributes) -> ByronTx {
    ByronTx {
        inputs: ins,
        outputs: outs,
        attributes: attrs,
    }
}

fn empty_witnesses() -> ByronWitnesses {
    MaybeIndefArray::Def(Vec::new())
}

fn new_utxos<'a>() -> UTxOs<'a> {
    UTxOs::new()
}
