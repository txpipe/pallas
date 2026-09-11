//! Transaction builders that write CBOR directly, so a test reading a field back reads bytes.

// A builder whose only readers are the tests of one era is unused in a build
// that has no such era compiled in.
#![allow(dead_code)]

use pallas_codec::minicbor::{self, data::Tag};

pub fn dijkstra_block_tx(
    body: &[u8],
    witness_set: &[u8],
    auxiliary_data: Option<&[u8]>,
    valid: bool,
) -> Vec<u8> {
    let mut e = minicbor::Encoder::new(Vec::new());
    e.array(4).unwrap();
    e.writer_mut().extend_from_slice(body);
    e.writer_mut().extend_from_slice(witness_set);
    write_aux(&mut e, auxiliary_data);
    e.bool(valid).unwrap();
    e.into_writer()
}

pub fn dijkstra_mempool_tx(
    body: &[u8],
    witness_set: &[u8],
    auxiliary_data: Option<&[u8]>,
) -> Vec<u8> {
    let mut e = minicbor::Encoder::new(Vec::new());
    e.array(3).unwrap();
    e.writer_mut().extend_from_slice(body);
    e.writer_mut().extend_from_slice(witness_set);
    write_aux(&mut e, auxiliary_data);
    e.into_writer()
}

pub fn conway_tx(
    body: &[u8],
    witness_set: &[u8],
    auxiliary_data: Option<&[u8]>,
    valid: bool,
) -> Vec<u8> {
    let mut e = minicbor::Encoder::new(Vec::new());
    e.array(4).unwrap();
    e.writer_mut().extend_from_slice(body);
    e.writer_mut().extend_from_slice(witness_set);
    e.bool(valid).unwrap();
    write_aux(&mut e, auxiliary_data);
    e.into_writer()
}

fn write_aux(e: &mut minicbor::Encoder<Vec<u8>>, auxiliary_data: Option<&[u8]>) {
    match auxiliary_data {
        Some(aux) => e.writer_mut().extend_from_slice(aux),
        None => {
            e.null().unwrap();
        }
    }
}

/// Builds a transaction body with one input, no outputs and a fee.
pub fn minimal_body() -> Vec<u8> {
    let mut e = minicbor::Encoder::new(Vec::new());
    e.map(3).unwrap();
    write_mandatory_keys(&mut e);
    e.into_writer()
}

pub fn body_with_proposal(proposal: &[u8]) -> Vec<u8> {
    let mut e = minicbor::Encoder::new(Vec::new());
    e.map(4).unwrap();
    write_mandatory_keys(&mut e);

    e.u8(20).unwrap();
    e.tag(Tag::new(258)).unwrap();
    e.array(1).unwrap();
    e.writer_mut().extend_from_slice(proposal);

    e.into_writer()
}

fn write_mandatory_keys(e: &mut minicbor::Encoder<Vec<u8>>) {
    e.u8(0).unwrap();
    e.tag(Tag::new(258)).unwrap();
    e.array(1).unwrap();
    e.array(2).unwrap();
    e.bytes(&[0x11; 32]).unwrap();
    e.u8(0).unwrap();

    e.u8(1).unwrap();
    e.array(0).unwrap();

    e.u8(2).unwrap();
    e.u32(1_000).unwrap();
}

pub fn empty_witness_set() -> Vec<u8> {
    let mut e = minicbor::Encoder::new(Vec::new());
    e.map(0).unwrap();
    e.into_writer()
}

pub fn witness_set_with_native_script(script: &[u8]) -> Vec<u8> {
    let mut e = minicbor::Encoder::new(Vec::new());
    e.map(1).unwrap();
    e.u8(1).unwrap();
    e.tag(Tag::new(258)).unwrap();
    e.array(1).unwrap();
    e.writer_mut().extend_from_slice(script);
    e.into_writer()
}

/// Builds a witness set with one redeemer at key 5. The datum and
/// execution units are arbitrary.
pub fn witness_set_with_redeemer(tag: u8, index: u32) -> Vec<u8> {
    let mut e = minicbor::Encoder::new(Vec::new());
    e.map(1).unwrap();

    e.u8(5).unwrap();
    e.map(1).unwrap();

    e.array(2).unwrap();
    e.u8(tag).unwrap();
    e.u32(index).unwrap();

    // redeemers_value = [plutus_data, ex_units]
    e.array(2).unwrap();
    e.tag(Tag::new(121)).unwrap();
    e.array(0).unwrap();
    e.array(2).unwrap();
    e.u32(1_000).unwrap();
    e.u32(2_000).unwrap();

    e.into_writer()
}

/// Builds a `script_pubkey` clause, which every era since Shelley has.
pub fn native_script_pubkey(keyhash: u8) -> Vec<u8> {
    let mut e = minicbor::Encoder::new(Vec::new());
    e.array(2).unwrap();
    e.u8(0).unwrap();
    e.bytes(&[keyhash; 28]).unwrap();
    e.into_writer()
}

/// Builds an `invalid_before` clause, written canonically.
pub fn native_script_invalid_before(slot: u8) -> Vec<u8> {
    let mut e = minicbor::Encoder::new(Vec::new());
    e.array(2).unwrap();
    e.u8(4).unwrap();
    e.u8(slot).unwrap();
    e.into_writer()
}

/// Builds the same clause with the slot in the one byte unsigned form. No
/// encoder in this workspace emits that form, so a hash over these bytes
/// differs from a hash over a re-encoding.
pub fn native_script_invalid_before_long_form(slot: u8) -> Vec<u8> {
    assert!(slot < 24, "a larger slot is already written in this form");
    let mut e = minicbor::Encoder::new(Vec::new());
    e.array(2).unwrap();
    e.u8(4).unwrap();
    e.writer_mut().extend_from_slice(&[0x18, slot]);
    e.into_writer()
}

/// Builds a `script_require_guard` clause, which no era before Dijkstra has.
pub fn native_script_require_guard(keyhash: u8) -> Vec<u8> {
    let mut e = minicbor::Encoder::new(Vec::new());
    e.array(2).unwrap();
    e.u8(6).unwrap();
    e.array(2).unwrap();
    e.u8(0).unwrap();
    e.bytes(&[keyhash; 28]).unwrap();
    e.into_writer()
}

/// Builds auxiliary data in the post Alonzo map form, tag 259, with one
/// native script and, when given, one PlutusV4 script.
pub fn post_alonzo_aux_data(native_script: &[u8], plutus_v4: Option<&[u8]>) -> Vec<u8> {
    match plutus_v4 {
        Some(script) => post_alonzo_aux_data_with_plutus(native_script, &[(5, script)]),
        None => post_alonzo_aux_data_with_plutus(native_script, &[]),
    }
}

/// Post Alonzo auxiliary data with one native script and one Plutus script
/// under each version key given (2 for V1 through 5 for V4).
pub fn post_alonzo_aux_data_with_plutus(native_script: &[u8], plutus: &[(u8, &[u8])]) -> Vec<u8> {
    let mut e = minicbor::Encoder::new(Vec::new());
    e.tag(Tag::new(259)).unwrap();
    e.map(1 + plutus.len() as u64).unwrap();

    e.u8(1).unwrap();
    e.array(1).unwrap();
    e.writer_mut().extend_from_slice(native_script);

    for (key, script) in plutus {
        e.u8(*key).unwrap();
        e.array(1).unwrap();
        e.bytes(script).unwrap();
    }

    e.into_writer()
}

/// Builds a post Alonzo output whose reference script is wrapped in a
/// `#6.24` tagged byte string.
pub fn post_alonzo_output_with_script_ref(address: &[u8], coin: u64, script: &[u8]) -> Vec<u8> {
    let mut e = minicbor::Encoder::new(Vec::new());
    e.map(3).unwrap();

    e.u8(0).unwrap();
    e.bytes(address).unwrap();

    e.u8(1).unwrap();
    e.u64(coin).unwrap();

    e.u8(3).unwrap();
    e.tag(Tag::new(24)).unwrap();
    e.bytes(script).unwrap();

    e.into_writer()
}

pub fn script_ref_plutus_v4(bytes: &[u8]) -> Vec<u8> {
    let mut e = minicbor::Encoder::new(Vec::new());
    e.array(2).unwrap();
    e.u8(4).unwrap();
    e.bytes(bytes).unwrap();
    e.into_writer()
}

/// Builds a `plutus_v2_script` reference, which every era since Babbage has.
pub fn script_ref_plutus_v2(bytes: &[u8]) -> Vec<u8> {
    let mut e = minicbor::Encoder::new(Vec::new());
    e.array(2).unwrap();
    e.u8(2).unwrap();
    e.bytes(bytes).unwrap();
    e.into_writer()
}

/// Builds a transaction body with one input, one raw CBOR output and a fee.
pub fn body_with_output(output: &[u8]) -> Vec<u8> {
    let mut e = minicbor::Encoder::new(Vec::new());
    e.map(3).unwrap();

    e.u8(0).unwrap();
    e.tag(Tag::new(258)).unwrap();
    e.array(1).unwrap();
    e.array(2).unwrap();
    e.bytes(&[0x11; 32]).unwrap();
    e.u8(0).unwrap();

    e.u8(1).unwrap();
    e.array(1).unwrap();
    e.writer_mut().extend_from_slice(output);

    e.u8(2).unwrap();
    e.u32(1_000).unwrap();

    e.into_writer()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shared_transaction_fixtures_are_what_these_builders_write() {
        let proposal = hex::decode(include_str!(
            "../../test_data/proposal-param-change-key48.hex"
        ))
        .unwrap();
        let built = dijkstra_block_tx(
            &body_with_proposal(&proposal),
            &empty_witness_set(),
            None,
            true,
        );
        assert_eq!(
            hex::encode(&built),
            include_str!("../../test_data/dijkstra-proposal.tx").trim(),
            "test_data/dijkstra-proposal.tx has drifted from the builder"
        );

        let guard = native_script_require_guard(0x7a);
        let v4 = [0xd8, 0x79, 0x80];
        let output =
            post_alonzo_output_with_script_ref(&[0x60; 29], 2_000_000, &script_ref_plutus_v4(&v4));
        let built = dijkstra_block_tx(
            &body_with_output(&output),
            &witness_set_with_native_script(&guard),
            Some(&post_alonzo_aux_data(&guard, Some(&v4))),
            true,
        );
        assert_eq!(
            hex::encode(&built),
            include_str!("../../test_data/dijkstra-scripts.tx").trim(),
            "test_data/dijkstra-scripts.tx has drifted from the builder"
        );
    }
}
