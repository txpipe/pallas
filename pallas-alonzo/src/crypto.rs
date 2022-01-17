use crate::{AuxiliaryData, Header, PlutusData, TransactionBody};
use cryptoxide::blake2b::Blake2b;
use minicbor::Encode;

pub type Hash32 = [u8; 32];

pub type Error = Box<dyn std::error::Error>;

struct Hasher<const N: usize> {
    inner: Blake2b,
}

impl Hasher<256> {
    #[inline]
    fn new() -> Self {
        Self {
            inner: Blake2b::new(32),
        }
    }

    #[inline]
    fn result(mut self) -> Hash32 {
        use cryptoxide::digest::Digest as _;

        let mut hash = [0; 32];
        self.inner.result(&mut hash);
        hash
    }
}

impl<'a, const N: usize> minicbor::encode::write::Write for &'a mut Hasher<N> {
    type Error = std::convert::Infallible;

    fn write_all(&mut self, buf: &[u8]) -> Result<(), Self::Error> {
        use cryptoxide::digest::Digest as _;
        self.inner.input(buf);
        Ok(())
    }
}

// TODO: think if we should turn this into a blanket implementation of a new
// trait
fn hash_cbor_encodable(data: &impl Encode) -> Result<Hash32, Error> {
    let mut hasher = Hasher::<256>::new();
    let () = minicbor::encode(data, &mut hasher)?;

    Ok(hasher.result())
}

pub fn hash_block_header(data: &Header) -> Result<Hash32, Error> {
    hash_cbor_encodable(data)
}

pub fn hash_auxiliary_data(data: &AuxiliaryData) -> Result<Hash32, Error> {
    hash_cbor_encodable(data)
}

pub fn hash_transaction(data: &TransactionBody) -> Result<Hash32, Error> {
    hash_cbor_encodable(data)
}

pub fn hash_plutus_data(data: &PlutusData) -> Result<Hash32, Error> {
    hash_cbor_encodable(data)
}

#[cfg(test)]
mod tests {
    use crate::{BlockWrapper, Fragment};

    use super::hash_transaction;

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
            let computed_hash = hash_transaction(tx).expect(&format!(
                "error hashing tx {} from block {}",
                tx_idx, block_idx
            ));
            let known_hash = valid_hashes[tx_idx];
            assert_eq!(hex::encode(computed_hash), known_hash)
        }
    }
}
