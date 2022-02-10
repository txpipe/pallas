use super::{Block, BlockHead, EbbHead};
use pallas_crypto::hash::{Hash, Hasher};

pub fn hash_boundary_block_header(header: &EbbHead) -> Hash<32> {
    // hash expects to have a prefix for the type of block
    Hasher::<256>::hash_cbor(&(0, header))
}

pub fn hash_main_block_header(header: &BlockHead) -> Hash<32> {
    // hash expects to have a prefix for the type of block
    Hasher::<256>::hash_cbor(&(1, header))
}

pub fn hash_block_header(block: &Block) -> Hash<32> {
    match block {
        Block::EbBlock(x) => hash_boundary_block_header(&x.header),
        Block::MainBlock(x) => hash_main_block_header(&x.header),
    }
}

//pub fn hash_transaction(data: &TransactionBody) -> Hash<32> {
//    Hasher::<256>::hash_cbor(data)
//}

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

        let computed_hash = super::hash_block_header(&block_model);

        assert_eq!(hex::encode(computed_hash), KNOWN_HASH)
    }
}
