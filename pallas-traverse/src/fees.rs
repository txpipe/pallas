use pallas_codec::minicbor::to_vec;
use pallas_primitives::byron;

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

pub fn compute_linear_fee_policy(tx_size: u64, params: &PolicyParams) -> u64 {
    let nanos = params.constant + (tx_size * params.size_coeficient);

    let loves = nanos / 1_000_000_000;

    let rem = match nanos % 1_000_000_000 {
        0 => 0u64,
        _ => 1u64,
    };

    loves + rem
}

pub fn compute_byron_fee(tx: &byron::MintedTxPayload, params: Option<&PolicyParams>) -> u64 {
    let tx_size = to_vec(tx).unwrap().len();

    match params {
        Some(params) => compute_linear_fee_policy(tx_size as u64, params),
        None => compute_linear_fee_policy(tx_size as u64, &PolicyParams::default()),
    }
}

#[cfg(test)]
mod tests {
    use super::compute_byron_fee;

    #[test]
    fn known_fee_matches() {
        // TODO: expand this test to include more test blocks
        let block_str = include_str!("../../test_data/byron4.block");

        let block_bytes = hex::decode(block_str).expect("bad block file");
        let block = crate::MultiEraBlock::decode_byron(&block_bytes).unwrap();
        let txs = block.txs();

        // don't want to pass if we don't have tx in the block
        assert!(!txs.is_empty());

        for tx in txs.iter().take(1) {
            let byron = tx.as_byron().unwrap();
            let fee = compute_byron_fee(byron, None);
            assert_eq!(fee, 171070);
        }
    }
}
