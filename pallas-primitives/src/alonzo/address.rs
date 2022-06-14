use crate::Error;

use super::TransactionOutput;
use bech32::{self, ToBase32};

pub fn encode_bech32_address(data: &[u8], hrp: &str) -> Result<String, Error> {
    bech32::encode(hrp, data.to_base32(), bech32::Variant::Bech32).map_err(|e| e.into())
}

impl TransactionOutput {
    pub fn to_bech32_address(&self, hrp: &str) -> Result<String, Error> {
        encode_bech32_address(self.address.as_slice(), hrp)
    }
}

#[cfg(test)]
mod tests {
    use pallas_codec::minicbor;

    use crate::alonzo::Block;

    type BlockWrapper = (u16, Block);

    const KNOWN_ADDRESSES: &[&str] =&[
        "addr_test1vzzql63nddp8qdgka578hx6pats290js9kmn4uay5we9fwsgza0z3",
        "addr_test1qzlqdmc0npkdzvgkdlzx8xzv0jucenqxr08cpf9p3s7u5k7rgeu6pd0ng8lhsnme5w4gdjfwfngl4tqxhpfasgampuksrrmxfy",
        "addr_test1qpmtp5t0t5y6cqkaz7rfsyrx7mld77kpvksgkwm0p7en7qum7a589n30e80tclzrrnj8qr4qvzj6al0vpgtnmrkkksnqd8upj0",
        "addr_test1qz2fxv2umyhttkxyxp8x0dlpdt3k6cwng5pxj3jhsydzer3jcu5d8ps7zex2k2xt3uqxgjqnnj83ws8lhrn648jjxtwq2ytjqp",
        "addr_test1wqe44q5knddmv9rldtw2k0vn8d2am4pujtj8mm9u0a0t5esed99ya",
        "addr_test1qqwvmfyhqhx8ljjtq30t7nzus6u8jy8qhsk58gsmsh6ceeka9qwl7c3dwddxnk5tzswrjy06kenj4c3qkuhzw7s5k2ns0cs0cp",
        "addr_test1wpl98az2a6w7pu9us27l5k78wz94y02wm8fttq2qy2jmtfgmqelfe",
        "addr_test1qzy6yecn04s5dj49mgyaxh4q6wf2arxpdmrahvl9uwrvkravh7fy2k7pu7dslmp9pkkzgd8yy0fkdexpqpwglx75lc2qswpw53",
        "addr_test1qqt6mydgwuems9aclwvthunty0hs9a4n2kfu202q9xrdtr3j4cmsjwn8cd7kcnyzgraqu09l5wgv5l8zexw5dy43fwzsl0wg9q",
        "addr_test1qpjkgkn77jq5y00caf2zra7lnzdjhtl5878uvhvnwd4h4yv8c5heyhcqwa8ddqd4xprwrq9qflsyc4567ymkv0jzeyhqz9hz45",
        "addr_test1qqr9c5s9tyac09j434tf86h0gh9f9acd4nrllnqqthr5fcumajy082egq3c2yq264skep28zs4se9znqhy4xt98tk57q4yzg7h",
    ];

    #[test]
    fn known_address_matches() {
        // TODO: expand this test to include more test blocks
        let block_idx = 1;
        let block_str = include_str!("../../../test_data/alonzo2.block");

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
