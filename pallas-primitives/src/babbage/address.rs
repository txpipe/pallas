use crate::Error;

use super::TransactionOutput;
use bech32::{self, ToBase32};

pub fn encode_bech32_address(data: &[u8], hrp: &str) -> Result<String, Error> {
    bech32::encode(hrp, data.to_base32(), bech32::Variant::Bech32).map_err(|e| e.into())
}

impl TransactionOutput {
    pub fn to_bech32_address(&self, hrp: &str) -> Result<String, Error> {
        let address = match self {
            TransactionOutput::Legacy(x) => &x.address,
            TransactionOutput::PostAlonzo(x) => &x.address,
        };

        encode_bech32_address(address.as_slice(), hrp)
    }
}

#[cfg(test)]
mod tests {
    use pallas_codec::minicbor;

    use crate::babbage::Block;

    type BlockWrapper = (u16, Block);

    const KNOWN_ADDRESSES: &[&str] =
        &["addr_test1vpfwv0ezc5g8a4mkku8hhy3y3vp92t7s3ul8g778g5yegsgalc6gc"];

    #[test]
    fn known_address_matches() {
        // TODO: expand this test to include more test blocks
        let block_idx = 1;
        let block_str = include_str!("../../../test_data/babbage1.block");

        let block_bytes = hex::decode(block_str).expect(&format!("bad block file {}", block_idx));
        let (_, block): BlockWrapper = minicbor::decode(&block_bytes[..])
            .expect(&format!("error decoding cbor for file {}", block_idx));

        // don't want to pass if we don't have tx in the block
        assert!(block.transaction_bodies.len() > 0);

        for tx in block.transaction_bodies.iter() {
            for output in tx.outputs.iter() {
                let addr_str = output.to_bech32_address("addr_test").unwrap();

                assert!(
                    KNOWN_ADDRESSES.contains(&addr_str.as_str()),
                    "address {} not in known list",
                    addr_str
                );
            }
        }
    }
}
