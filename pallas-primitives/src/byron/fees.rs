use crate::Error;

use super::TxPayload;
use pallas_codec::minicbor::to_vec;

pub struct PolicyParams {
    constant: u64,
    size_coeficient: u64,
}

impl Default for PolicyParams {
    fn default() -> Self {
        Self {
            constant: 155_381_000_000_000u64,
            size_coeficient: 43_946_000_000u64,
        }
    }
}

fn compute_linear_fee_policy(tx_size: u64, params: &PolicyParams) -> u64 {
    println!("tx size: {}", tx_size);
    let nanos = params.constant + (tx_size * params.size_coeficient);

    let loves = nanos / 1_000_000_000;

    let rem = match nanos % 1_000_000_000 {
        0 => 0u64,
        _ => 1u64,
    };

    loves + rem
}

impl TxPayload {
    pub fn compute_fee(&self, params: &PolicyParams) -> Result<u64, Error> {
        let tx_size = to_vec(&self)?.len();
        let fee = compute_linear_fee_policy(tx_size as u64, params);

        Ok(fee)
    }

    pub fn compute_fee_with_defaults(&self) -> Result<u64, Error> {
        self.compute_fee(&PolicyParams::default())
    }
}

#[cfg(test)]
mod tests {
    use pallas_codec::minicbor;

    use crate::{byron::MainBlock, ToHash};

    type BlockWrapper = (u16, MainBlock);

    #[test]
    fn known_fee_matches() {
        // TODO: expand this test to include more test blocks
        let block_idx = 1;
        let block_str = include_str!("../../../test_data/byron4.block");

        let block_bytes = hex::decode(block_str).expect(&format!("bad block file {}", block_idx));
        let (_, block): BlockWrapper = minicbor::decode(&block_bytes[..])
            .expect(&format!("error decoding cbor for file {}", block_idx));

        // don't want to pass if we don't have tx in the block
        assert!(block.body.tx_payload.len() > 0);

        for tx in block.body.tx_payload.iter().take(1) {
            println!("{}", tx.transaction.to_hash());
            let fee = tx.compute_fee_with_defaults().unwrap();
            assert_eq!(fee, 171070);
        }
    }
}
