use super::{AuxiliaryData, Header, NativeScript, PlutusData, TransactionBody};
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

#[deprecated(note = "use PlutusData::to_hash instead")]
pub fn hash_plutus_data(data: &PlutusData) -> Hash<32> {
    Hasher::<256>::hash_cbor(data)
}

impl NativeScript {
    pub fn to_hash(&self) -> Hash<28> {
        Hasher::<224>::hash_tagged_cbor(self, 0)
    }
}

impl PlutusData {
    pub fn to_hash(&self) -> Hash<32> {
        Hasher::<256>::hash_cbor(self)
    }
}

impl TransactionBody {
    pub fn to_hash(&self) -> Hash<32> {
        Hasher::<256>::hash_cbor(self)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use pallas_codec::utils::MaybeIndefArray;
    use pallas_crypto::hash::Hash;

    use crate::alonzo::{BlockWrapper, NativeScript};
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

    #[test]
    fn native_script_hashes_cardano_cli() {
        // construct an arbitrary script to use as example
        let ns = NativeScript::ScriptAll(MaybeIndefArray::Def(vec![
            NativeScript::ScriptPubkey(
                Hash::<28>::from_str("4d04380dcb9fbad5aff8e2f4e19394ef4e5e11b37932838f01984a12")
                    .unwrap(),
            ),
            NativeScript::InvalidBefore(112500819),
        ]));

        // hash that we assume correct since it was generated through the cardano-cli
        let cardano_cli_output = "d6a8ced01ecdfbb26c90850010a06fbc20a7c23632fc92f531667f36";

        assert_eq!(
            ns.to_hash(),
            Hash::<28>::from_str(cardano_cli_output).unwrap()
        )
    }
}
