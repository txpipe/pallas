use rand::Rng;
use std::{borrow::Cow, vec::Vec};

use pallas_applying::{
    types::{ByronProtParams, MultiEraProtParams},
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
    //      iv)  the size of the transaction is 82 bytes—it is easy to verify that
    //              82 == pallas_applying::get_byron_tx_size(tx)
    // The expected fees are therefore 7 + 11 * 82 = 909 lovelace, which is why the output contains
    // 100000 - 909 = 99091 lovelace.
    fn successful_case() {
        let protocol_params: ByronProtParams = ByronProtParams;
        let mut tx_ins: ByronTxIns = empty_byron_tx_ins();
        let tx_in: ByronTxIn = new_tx_in(random_tx_id(), 3);
        add_byron_tx_in(&mut tx_ins, &tx_in);
        let mut tx_outs: ByronTxOuts = new_byron_tx_outs();
        let tx_out: ByronTxOut = new_byron_tx_out(new_address(random_address_payload(), 0), 99091);
        add_byron_tx_out(&mut tx_outs, &tx_out);
        let mut utxos: UTxOs = new_utxos();
        // input_tx_out is the ByronTxOut associated with tx_in.
        let input_tx_out: ByronTxOut =
            new_byron_tx_out(new_address(random_address_payload(), 0), 100000);
        add_to_utxo(&mut utxos, tx_in, input_tx_out);
        let validation_result = mk_byron_tx_and_validate(
            &new_byron_tx(tx_ins, tx_outs, empty_byron_attributes()),
            &empty_byron_witnesses(),
            &utxos,
            &protocol_params,
        );
        match validation_result {
            Ok(()) => (),
            Err(err) => assert!(false, "Unexpected error (sucessful_case - {:?}).", err),
        }
    }
}

// Types aliases.
type ByronTxIns = MaybeIndefArray<ByronTxIn>;
type ByronTxOuts = MaybeIndefArray<ByronTxOut>;

// Helper functions.
fn empty_byron_tx_ins() -> ByronTxIns {
    MaybeIndefArray::Def(Vec::new())
}

fn random_tx_id() -> ByronTxId {
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

fn new_byron_tx_outs() -> ByronTxOuts {
    MaybeIndefArray::Def(Vec::new())
}

fn random_address_payload() -> TagWrap<ByteVec, 24> {
    let mut rng = rand::thread_rng();
    let mut bytes = [0u8; 24];
    for elem in bytes.iter_mut() {
        *elem = rng.gen();
    }
    TagWrap::<ByteVec, 24>::new(ByteVec::from(bytes.to_vec()))
}

fn new_address(payload: TagWrap<ByteVec, 24>, crc: u32) -> Address {
    Address {
        payload: payload,
        crc: crc,
    }
}

fn new_byron_tx_out(address: Address, amount: u64) -> ByronTxOut {
    ByronTxOut {
        address: address,
        amount: amount,
    }
}

fn add_byron_tx_out(outs: &mut ByronTxOuts, new_out: &ByronTxOut) {
    match outs {
        MaybeIndefArray::Def(vec) | MaybeIndefArray::Indef(vec) => vec.push(new_out.clone()),
    }
}

fn add_to_utxo<'a>(utxos: &mut UTxOs<'a>, tx_in: ByronTxIn, tx_out: ByronTxOut) {
    let multi_era_in: MultiEraInput = MultiEraInput::Byron(Box::new(Cow::Owned(tx_in)));
    let multi_era_out: MultiEraOutput = MultiEraOutput::Byron(Box::new(Cow::Owned(tx_out)));
    utxos.insert(multi_era_in, multi_era_out);
}

fn empty_byron_attributes() -> Attributes {
    EmptyMap
}

/// pallas_applying::validate takes a MultiEraTx, not a ByronTx and a ByronWitnesses. To be able to
/// build a MultiEraTx from a ByronTx and a ByronWitnesses, we need to encode each of them and then
/// decode them into KeepRaw<ByronTx> and KeepRaw<ByronWitnesses> values, respectively.
fn mk_byron_tx_and_validate(
    btx: &ByronTx,
    bwit: &ByronWitnesses,
    utxos: &UTxOs,
    prot_pps: &ByronProtParams,
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
    validate(
        &metx,
        utxos,
        &MultiEraProtParams::Byron(Box::new(Cow::Borrowed(&prot_pps))),
    )
}

fn new_byron_tx(ins: ByronTxIns, outs: ByronTxOuts, attrs: Attributes) -> ByronTx {
    ByronTx {
        inputs: ins,
        outputs: outs,
        attributes: attrs,
    }
}

fn empty_byron_witnesses() -> ByronWitnesses {
    MaybeIndefArray::Def(Vec::new())
}

fn new_utxos<'a>() -> UTxOs<'a> {
    UTxOs::new()
}
