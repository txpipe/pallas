use crate::Error;

use super::Address;
use base58::ToBase58;
use pallas_codec::minicbor::to_vec;

impl Address {
    pub fn to_addr_string(&self) -> Result<String, Error> {
        let cbor = to_vec(self)?;
        Ok(cbor.to_base58())
    }
}

#[cfg(test)]
mod tests {
    use crate::byron::Block;
    use crate::Fragment;

    const KNOWN_ADDRESSES: &[&str] = &[
        "DdzFFzCqrht8QHTQXbWy2qoyPaqTN8BjyfKygGmpy9dtot1tvkBfCaVTnR22XCaaDVn3M1U6aiMShoCLzw6VWSwzQKhhJrM3YjYp3wyy",
        "DdzFFzCqrhsjUjMRukzoFx8sToHzCt4iidB17STXk9adAoVMNus5SvAjS1cXfPKbuNbPUZ5xQG25sMK85n9GdMkqo2ytqBnKWC68s8P3",
        "Ae2tdPwUPEZFBnsqpm2RkDQfwJseUrBKrTECCDom4bAqNsxTNwbMPCZtbyJ",
        "DdzFFzCqrhsvNcX48aHrYdcQhKhAwEdLebH7xPskKtQnZucNQmXXmEMJEU2NDqipuDTZKFec5UMCtm4vYoUgjixxULJexAzHGa3Bmctk",
        "Ae2tdPwUPEZGEC75fV3vktzbwxhkD71JHxSYVgiNCgKB7Yo1rWamWVJDFsV",
        "DdzFFzCqrht1K1sR9fQWEdcxhTxgqKQqwPHucez1McJy14uqbxu1UKnnq12EdHr9NxYs8RqKnSvswegh8wLvfC4fw6arB3nyqC5Wy4Ky",
        "DdzFFzCqrhtC8HauYJMa59DzoAeTLnpDcdst1hFWmjkTdg5Xu55ougiBpAmwuo2Coe2DfAj26m52aF4e2yk8v5GQc4umZxsXUT2CuTB2",
        "DdzFFzCqrht4Zsigv43q9LRHNsgu6TWdASuGe6tCYuv2B9H2wTggkvyuwHMb5WALqWDDiNQEHYq7BvnFJ65UDzKi6ThdZusVYmYLpJg9",
    ];

    #[test]
    fn known_address_matches() {
        // TODO: expand this test to include more test blocks
        let block_idx = 1;
        let block_str = include_str!("test_data/test2.block");

        let block_bytes = hex::decode(block_str).expect(&format!("bad block file {}", block_idx));
        let block = Block::decode_fragment(&block_bytes[..])
            .expect(&format!("error decoding cbor for file {}", block_idx));

        let block = match block {
            Block::MainBlock(x) => x,
            Block::EbBlock(_) => panic!(),
        };

        // don't want to pass if we don't have tx in the block
        assert!(block.body.tx_payload.len() > 0);

        for tx in block.body.tx_payload.iter() {
            for output in tx.deref().transaction.outputs.iter() {
                let addr_str = output.address.to_addr_string().unwrap();

                assert!(
                    KNOWN_ADDRESSES.contains(&addr_str.as_str()),
                    "address {} not in known list",
                    addr_str
                );
            }
        }
    }
}
