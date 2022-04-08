use super::{AuxiliaryData, Header, PlutusData, TransactionBody};
use pallas_crypto::hash::{Hash, Hasher};

pub fn hash_block_header(data: &Header) -> Hash<32> {
    Hasher::<256>::hash_cbor(data)
}

pub fn hash_auxiliary_data(data: &AuxiliaryData) -> Hash<32> {
    Hasher::<256>::hash_cbor(data)
}

#[deprecated(note = "use TransactionBody::to_hash instead")]
pub fn hash_transaction(data: &TransactionBody) -> Hash<32> {
    Hasher::<256>::hash_cbor(data)
}

pub fn hash_plutus_data(data: &PlutusData) -> Hash<32> {
    Hasher::<256>::hash_cbor(data)
}

impl TransactionBody {
    pub fn to_hash(&self) -> Hash<32> {
        Hasher::<256>::hash_cbor(self)
    }
}

#[cfg(test)]
mod tests {
    use crate::alonzo::BlockWrapper;
    use crate::Fragment;

    #[test]
    fn transaction_hash_works() {
        // TODO: expand this test to include more test blocks
        let block_idx = 1;
        let block_str = include_str!("test_data/test1.block");

        let block_bytes = hex::decode(block_str).expect(&format!("bad block file {}", block_idx));
        let block_model = BlockWrapper::decode_fragment(&block_bytes[..])
            .expect(&format!("error decoding cbor for file {}", block_idx));

        let valid_hashes = vec![
            "8ae0cd531635579a9b52b954a840782d12235251fb1451e5c699e864c677514a",
            "bb5bb4e1c09c02aa199c60e9f330102912e3ef977bb73ecfd8f790945c6091d4",
            "8cdd88042ddb6c800714fb1469fb1a1a93152aae3c87a81f2a3016f2ee5c664a",
            "10add6bdaa7ade06466bdd768456e756709090846b58bf473f240c484db517fa",
            "8838f5ab27894a6543255aeaec086f7b3405a6db6e7457a541409cdbbf0cd474",
        ];

        for (tx_idx, tx) in block_model.1.transaction_bodies.iter().enumerate() {
            let computed_hash = tx.to_hash();
            let known_hash = valid_hashes[tx_idx];
            assert_eq!(hex::encode(computed_hash), known_hash)
        }
    }
}
