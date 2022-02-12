use super::{Block, BlockHead, EbbHead, Tx};
use pallas_crypto::hash::{Hash, Hasher};

impl EbbHead {
    pub fn to_hash(&self) -> Hash<32> {
        // hash expects to have a prefix for the type of block
        Hasher::<256>::hash_cbor(&(0, self))
    }
}

impl BlockHead {
    pub fn to_hash(&self) -> Hash<32> {
        // hash expects to have a prefix for the type of block
        Hasher::<256>::hash_cbor(&(1, self))
    }
}

impl Block {
    pub fn to_hash(&self) -> Hash<32> {
        match self {
            Block::EbBlock(x) => x.header.to_hash(),
            Block::MainBlock(x) => x.header.to_hash(),
        }
    }
}

impl Tx {
    pub fn to_hash(&self) -> Hash<32> {
        Hasher::<256>::hash_cbor(self)
    }
}

#[cfg(test)]
mod tests {
    use crate::byron::Block;
    use crate::Fragment;

    const KNOWN_HASH: &'static str =
        "5c196e7394ace0449ba5a51c919369699b13896e97432894b4f0354dce8670b6";

    #[test]
    fn transaction_hash_works() {
        // TODO: expand this test to include more test blocks
        let block_idx = 1;
        let block_str = include_str!("test_data/test1.block");

        let block_bytes = hex::decode(block_str).expect(&format!("bad block file {}", block_idx));
        let block_model = Block::decode_fragment(&block_bytes[..])
            .expect(&format!("error decoding cbor for file {}", block_idx));

        let computed_hash = block_model.to_hash();

        assert_eq!(hex::encode(computed_hash), KNOWN_HASH)
    }
}
