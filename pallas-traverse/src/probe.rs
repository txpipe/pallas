//! Lightweight inspection of block data without full CBOR decoding

use pallas_codec::minicbor::decode::{Token, Tokenizer};

use crate::Era;

#[derive(Debug)]
pub enum Outcome {
    Matched(Era),
    EpochBoundary,
    Inconclusive,
}

// Executes a very lightweight inspection of the initial tokens of the CBOR
// block payload to extract the tag of the block wrapper which defines the era
// of the contained bytes.
pub fn block_era(cbor: &[u8]) -> Outcome {
    let mut tokenizer = Tokenizer::new(cbor);

    if !matches!(tokenizer.next(), Some(Ok(Token::Array(2)))) {
        return Outcome::Inconclusive;
    }

    match tokenizer.next() {
        Some(Ok(Token::U8(variant))) => match variant {
            0 => Outcome::EpochBoundary,
            1 => Outcome::Matched(Era::Byron),
            2 => Outcome::Matched(Era::Shelley),
            3 => Outcome::Matched(Era::Allegra),
            4 => Outcome::Matched(Era::Mary),
            5 => Outcome::Matched(Era::Alonzo),
            6 => Outcome::Matched(Era::Babbage),
            _ => Outcome::Inconclusive,
        },
        _ => Outcome::Inconclusive,
    }
}

fn skip_until_match(tokenizer: &mut Tokenizer, expected: Token) -> bool {
    while let Some(Ok(token)) = tokenizer.next() {
        if token == expected {
            return true;
        }
    }

    false
}

fn skip_items(tokenizer: &mut Tokenizer, quantity: usize) -> bool {
    let mut skipped = 0;

    while skipped < quantity {
        let token = tokenizer.next();

        if !matches!(token, Some(Ok(_))) {
            return false;
        }

        let a = match token {
            Token::Array(x) => skip_until_match(tokenizer, expected)
            Token::Map(_) => todo!(),
            Token::Tag(_) => todo!(),
            Token::Simple(_) => todo!(),
            Token::Break => todo!(),
            Token::Null => todo!(),
            Token::Undefined => todo!(),
            Token::BeginBytes => todo!(),
            Token::BeginString => todo!(),
            Token::BeginArray => todo!(),
            Token::BeginMap => todo!(),
            _ => true,
        }

        skipped += 1;

        if skipped == quantity {
            return true;
        }
    }

    true
}

pub fn tx_era(cbor: &[u8]) -> Outcome {
    let mut tokenizer = Tokenizer::new(cbor);

    if !matches!(tokenizer.next(), Some(Ok(Token::Array(4)))) {
        return Outcome::Inconclusive;
    }

    Outcome::Matched(Era::Alonzo)
}

#[cfg(test)]
mod tests {
    use crate::MultiEraBlock;

    use super::*;

    #[test]
    fn genesis_block_detected() {
        let block_str = include_str!("../../test_data/genesis.block");
        let bytes = hex::decode(block_str).unwrap();

        let inference = block_era(bytes.as_slice());

        assert!(matches!(inference, Outcome::EpochBoundary));
    }

    #[test]
    fn byron_block_detected() {
        let block_str = include_str!("../../test_data/byron1.block");
        let bytes = hex::decode(block_str).unwrap();

        let inference = block_era(bytes.as_slice());

        assert!(matches!(inference, Outcome::Matched(Era::Byron)));
    }

    #[test]
    fn shelley_block_detected() {
        let block_str = include_str!("../../test_data/shelley1.block");
        let bytes = hex::decode(block_str).unwrap();

        let inference = block_era(bytes.as_slice());

        assert!(matches!(inference, Outcome::Matched(Era::Shelley)));
    }

    #[test]
    fn allegra_block_detected() {
        let block_str = include_str!("../../test_data/allegra1.block");
        let bytes = hex::decode(block_str).unwrap();

        let inference = block_era(bytes.as_slice());

        assert!(matches!(inference, Outcome::Matched(Era::Allegra)));
    }

    #[test]
    fn mary_block_detected() {
        let block_str = include_str!("../../test_data/mary1.block");
        let bytes = hex::decode(block_str).unwrap();

        let inference = block_era(bytes.as_slice());

        assert!(matches!(inference, Outcome::Matched(Era::Mary)));
    }

    #[test]
    fn alonzo_block_detected() {
        let block_str = include_str!("../../test_data/alonzo1.block");
        let bytes = hex::decode(block_str).unwrap();

        let inference = block_era(&bytes);

        assert!(matches!(inference, Outcome::Matched(Era::Alonzo)));
    }

    #[test]
    fn alonzo_tx_detected() {
        let block_str = include_str!("../../test_data/alonzo1.block");
        let bytes = hex::decode(block_str).unwrap();

        let block = MultiEraBlock::decode(&bytes).unwrap();

        for tx in block.txs() {
            let cbor = tx.encode();

            let inference = tx_era(&cbor);

            assert!(matches!(inference, Outcome::Matched(Era::Alonzo)));
        }
    }
}
