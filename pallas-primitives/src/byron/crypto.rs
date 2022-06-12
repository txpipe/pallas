use crate::ToHash;

use super::{BlockHead, EbbHead, Tx};
use pallas_codec::utils::KeepRaw;
use pallas_crypto::hash::{Hash, Hasher};

impl ToHash<32> for EbbHead {
    fn to_hash(&self) -> Hash<32> {
        // hash expects to have a prefix for the type of block
        Hasher::<256>::hash_cbor(&(0, self))
    }
}

impl ToHash<32> for KeepRaw<'_, EbbHead> {
    fn to_hash(&self) -> Hash<32> {
        // hash expects to have a prefix for the type of block
        Hasher::<256>::hash_cbor(&(0, self))
    }
}

impl ToHash<32> for BlockHead {
    fn to_hash(&self) -> Hash<32> {
        // hash expects to have a prefix for the type of block
        Hasher::<256>::hash_cbor(&(1, self))
    }
}

impl ToHash<32> for KeepRaw<'_, BlockHead> {
    fn to_hash(&self) -> Hash<32> {
        // hash expects to have a prefix for the type of block
        Hasher::<256>::hash_cbor(&(1, self))
    }
}

impl ToHash<32> for Tx {
    fn to_hash(&self) -> Hash<32> {
        Hasher::<256>::hash_cbor(self)
    }
}

impl ToHash<32> for KeepRaw<'_, Tx> {
    fn to_hash(&self) -> Hash<32> {
        Hasher::<256>::hash(self.raw_cbor())
    }
}

#[cfg(test)]
mod tests {
    use pallas_codec::minicbor;

    use crate::{byron::MintedMainBlock, ToHash};

    type BlockWrapper<'b> = (u16, MintedMainBlock<'b>);

    const KNOWN_HASH: &'static str =
        "5c196e7394ace0449ba5a51c919369699b13896e97432894b4f0354dce8670b6";

    #[test]
    fn transaction_hash_works() {
        // TODO: expand this test to include more test blocks
        let block_idx = 1;
        let block_str = include_str!("../../../test_data/byron1.block");

        let block_bytes = hex::decode(block_str).expect(&format!("bad block file {}", block_idx));
        let (_, block_model): BlockWrapper = minicbor::decode(&block_bytes[..])
            .expect(&format!("error decoding cbor for file {}", block_idx));

        let computed_hash = block_model.header.to_hash();

        assert_eq!(hex::encode(computed_hash), KNOWN_HASH)
    }
}
