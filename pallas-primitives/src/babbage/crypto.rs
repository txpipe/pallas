use crate::ToHash;

use super::{Header, PlutusV2Script, TransactionBody, DatumOption};
use pallas_codec::utils::KeepRaw;
use pallas_crypto::hash::{Hash, Hasher};

impl ToHash<32> for Header {
    fn to_hash(&self) -> pallas_crypto::hash::Hash<32> {
        Hasher::<256>::hash_cbor(self)
    }
}

impl ToHash<28> for PlutusV2Script {
    fn to_hash(&self) -> Hash<28> {
        Hasher::<224>::hash_tagged_cbor(self, 1)
    }
}

impl ToHash<32> for TransactionBody {
    fn to_hash(&self) -> Hash<32> {
        Hasher::<256>::hash_cbor(self)
    }
}

impl ToHash<32> for KeepRaw<'_, TransactionBody> {
    fn to_hash(&self) -> pallas_crypto::hash::Hash<32> {
        Hasher::<256>::hash(self.raw_cbor())
    }
}

impl ToHash<32> for DatumOption {
    fn to_hash(&self) -> Hash<32> {
        match self {
            DatumOption::Hash(hash) => *hash,
            DatumOption::Data(data) => data.to_hash()
        }
    }
}

#[cfg(test)]
mod tests {
    use pallas_codec::minicbor;

    use crate::babbage::MintedBlock;
    use crate::ToHash;

    type BlockWrapper<'b> = (u16, MintedBlock<'b>);

    #[test]
    fn transaction_hash_works() {
        // TODO: expand this test to include more test blocks
        let block_idx = 1;
        let block_str = include_str!("../../../test_data/babbage1.block");

        let block_bytes = hex::decode(block_str).expect(&format!("bad block file {}", block_idx));
        let (_, block_model): BlockWrapper = minicbor::decode(&block_bytes[..])
            .expect(&format!("error decoding cbor for file {}", block_idx));

        let valid_hashes = vec!["3fad302595665b004971a6b76909854a39a0a7ecdbff3692f37b77ae37dbe882"];

        for (tx_idx, tx) in block_model.transaction_bodies.iter().enumerate() {
            let computed_hash = tx.to_hash();
            let known_hash = valid_hashes[tx_idx];
            assert_eq!(hex::encode(computed_hash), known_hash)
        }
    }
}
