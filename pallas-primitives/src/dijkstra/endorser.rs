//! The Leios endorser block, `endorser_block` in
//! `cardano-blueprint/src/network/node-to-node/leios-fetch/messages.cddl`.

use pallas_codec::utils::KeyValuePairs;
use pallas_crypto::hash::Hash;

/// The blake2b-256 and byte length of each transaction the block names.
pub type EndorserBlock = KeyValuePairs<Hash<32>, u32>;

#[cfg(test)]
mod tests {
    use super::*;
    use pallas_codec::minicbor;

    /// Name, body, and the count, first key and first size recorded for it in
    /// `test_data/dijkstra-fixtures.md`.
    const FIXTURES: &[(&str, &str, usize, &str, u32)] = &[
        (
            "dijkstra-eb1",
            include_str!("../../../test_data/dijkstra-eb1.ebbody"),
            1,
            "a69f9fc581e5914a101a3e619f5ce64b6bce76721fd5eed0cbad0e0f6d411cc5",
            229,
        ),
        (
            "dijkstra-eb2",
            include_str!("../../../test_data/dijkstra-eb2.ebbody"),
            30,
            "455a00b521f35f2c0a6ff0a59296c3316de6219af206c13d2e95870f66541fec",
            200,
        ),
    ];

    fn body(hex_body: &str) -> Vec<u8> {
        hex::decode(hex_body.trim()).expect("fixture body is hex")
    }

    #[cfg(not(feature = "relaxed"))]
    fn one_entry_with_key_of(len: u8) -> Vec<u8> {
        let mut body = vec![0xa1, 0x58, len];
        body.extend_from_slice(&vec![0x11; len as usize]);
        body.push(0x01);
        body
    }

    #[cfg(not(feature = "relaxed"))]
    #[test]
    fn a_key_that_is_not_32_bytes_is_refused_as_a_hash() {
        let block = minicbor::decode::<EndorserBlock>(&one_entry_with_key_of(32))
            .expect("a 32 byte key must be accepted");
        assert_eq!(block[0], (Hash::new([0x11; 32]), 1));

        for len in [31, 33] {
            let err = minicbor::decode::<EndorserBlock>(&one_entry_with_key_of(len))
                .expect_err("a key that is not 32 bytes must be refused");
            assert!(
                err.to_string().contains("Invalid hash size"),
                "wrong refusal for {len}: {err}"
            );
        }
    }

    #[test]
    fn each_body_decodes_to_its_recorded_count_and_first_entry() {
        for (name, hex_body, count, first_key, first_size) in FIXTURES {
            let block: EndorserBlock =
                minicbor::decode(&body(hex_body)).unwrap_or_else(|e| panic!("{name}: {e}"));

            assert_eq!(block.len(), *count, "{name} transaction count");

            let (key, size) = &block[0];
            assert_eq!(key.to_string(), *first_key, "{name} first key");
            assert_eq!(size, first_size, "{name} first size");
        }
    }

    #[test]
    fn each_body_re_encodes_byte_for_byte() {
        for (name, hex_body, ..) in FIXTURES {
            let raw = body(hex_body);
            let block: EndorserBlock = minicbor::decode(&raw).unwrap();

            assert_eq!(minicbor::to_vec(&block).unwrap(), raw, "{name}");
        }
    }

    #[test]
    fn an_indefinite_body_decodes_to_the_entries_of_its_definite_form() {
        let definite = body(FIXTURES[0].1);
        assert_eq!(definite[0], 0xa1, "fixture precondition");
        let expected = minicbor::decode::<EndorserBlock>(&definite).unwrap();

        let mut indefinite = vec![0xbf];
        indefinite.extend_from_slice(&definite[1..]);
        indefinite.push(0xff);

        let block = minicbor::decode::<EndorserBlock>(&indefinite)
            .expect("an indefinite length body must decode");
        assert_eq!(*block, *expected);
        assert_eq!(minicbor::to_vec(&block).unwrap(), indefinite);
    }
}
