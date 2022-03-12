//! Heuristics for detecting cbor content without decoding

use pallas_codec::minicbor::decode::{Token, Tokenizer};

use crate::Era;

#[derive(Debug)]
pub enum Outcome {
    Matched(Era),
    GenesisBlock,
    Inconclusive,
}

// Executes a very lightweight inspection of the initial tokens of the CBOR
// payload and infers with a certain degree of confidence the type of Cardano
// structure within.
pub fn probe_block_cbor_era(cbor: &[u8]) -> Outcome {
    let mut tokenizer = Tokenizer::new(cbor);

    if !matches!(tokenizer.next(), Some(Ok(Token::Array(2)))) {
        return Outcome::Inconclusive;
    }

    match tokenizer.next() {
        Some(Ok(Token::U8(variant))) => match variant {
            0 => Outcome::GenesisBlock,
            1 => Outcome::Matched(Era::Byron),
            2 => Outcome::Matched(Era::Shelley),
            3 => Outcome::Matched(Era::Allegra),
            4 => Outcome::Matched(Era::Mary),
            5 => Outcome::Matched(Era::Alonzo),
            _ => Outcome::Inconclusive,
        },
        _ => Outcome::Inconclusive,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn genesis_block_detected() {
        let block_str = include_str!("byron/test_data/genesis.block");
        let bytes = hex::decode(block_str).unwrap();

        let inference = probe_block_cbor_era(bytes.as_slice());

        assert!(matches!(inference, Outcome::GenesisBlock));
    }

    #[test]
    fn byron_block_detected() {
        let block_str = include_str!("byron/test_data/test1.block");
        let bytes = hex::decode(block_str).unwrap();

        let inference = probe_block_cbor_era(bytes.as_slice());

        assert!(matches!(inference, Outcome::Matched(Era::Byron)));
    }

    #[test]
    fn shelley_block_detected() {
        let block_str = include_str!("test_data/shelley1.block");
        let bytes = hex::decode(block_str).unwrap();

        let inference = probe_block_cbor_era(bytes.as_slice());

        assert!(matches!(inference, Outcome::Matched(Era::Shelley)));
    }

    #[test]
    fn allegra_block_detected() {
        let block_str = include_str!("test_data/allegra1.block");
        let bytes = hex::decode(block_str).unwrap();

        let inference = probe_block_cbor_era(bytes.as_slice());

        assert!(matches!(inference, Outcome::Matched(Era::Allegra)));
    }

    #[test]
    fn mary_block_detected() {
        let block_str = include_str!("test_data/mary1.block");
        let bytes = hex::decode(block_str).unwrap();

        let inference = probe_block_cbor_era(bytes.as_slice());

        assert!(matches!(inference, Outcome::Matched(Era::Mary)));
    }

    #[test]
    fn alonzo_block_detected() {
        let block_str = include_str!("alonzo/test_data/test1.block");
        let bytes = hex::decode(block_str).unwrap();

        let inference = probe_block_cbor_era(bytes.as_slice());

        assert!(matches!(inference, Outcome::Matched(Era::Alonzo)));
    }
}
