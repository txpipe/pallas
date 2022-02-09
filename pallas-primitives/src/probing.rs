//! Heuristics for detecting cbor content without decoding

use minicbor::decode::{Token, Tokenizer};

pub enum BlockInference {
    Byron,
    Shelley,
    Inconclusive,
}

// Executes a very lightweight inspection of the initial tokens of the CBOR
// payload and infers with a certain degree of confidence the type of Cardano
// structure within.
pub fn probe_block_cbor(cbor: &[u8]) -> BlockInference {
    let mut tokenizer = Tokenizer::new(cbor);

    if !matches!(tokenizer.next(), Some(Ok(Token::Array(2)))) {
        return BlockInference::Inconclusive;
    }

    if !matches!(tokenizer.next(), Some(Ok(Token::U8(_)))) {
        return BlockInference::Inconclusive;
    }

    //println!("{:?}", tokenizer.next());

    match tokenizer.next() {
        Some(Ok(Token::Array(3))) => BlockInference::Byron,
        Some(Ok(Token::Array(5))) => BlockInference::Shelley,
        _ => BlockInference::Inconclusive,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byron_block_detected() {
        let block_str = include_str!("byron/test_data/test1.block");
        let bytes = hex::decode(block_str).unwrap();

        let inference = probe_block_cbor(bytes.as_slice());

        assert!(matches!(inference, BlockInference::Byron));
    }

    #[test]
    fn shelley_block_detected() {
        let block_str = include_str!("alonzo/test_data/test1.block");
        let bytes = hex::decode(block_str).unwrap();

        let inference = probe_block_cbor(bytes.as_slice());

        assert!(matches!(inference, BlockInference::Shelley));
    }
}
