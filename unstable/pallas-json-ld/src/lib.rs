use pallas_crypto::hash::Hash;
use pallas_traverse::{MultiEraInput, MultiEraOutput, MultiEraTx};
use serde_json::{json, Value};

pub fn tx_in_to_json(tx_in: &MultiEraInput) -> Value {
    json!({ "@id": format!("{}#{}", tx_in.hash(), tx_in.index()) })
}

pub fn tx_out_asset_to_json(tx_in: &MultiEraInput) -> Value {
    json!({ "@id": format!("{}#{}", tx_in.hash(), tx_in.index()) })
}

pub fn tx_out_to_json(tx_out: &MultiEraOutput, tx_hash: Hash<32>, idx: usize) -> Value {
    json!({
        "@id": format!("{}#{}", tx_hash, idx),
        "address": tx_out.address().ok().map(|x| x.to_string()),
        "amount": tx_out.ada_amount(),
    })
}

pub fn tx_to_json(tx: &MultiEraTx) -> Value {
    json!({
        "@context": "https://txpipe.io/specs/cardano-ld",
        "@type": "tx",
        "hash": tx.hash(),
        "fee": tx.fee(),
        "inputs": tx.inputs().iter().map(tx_in_to_json).collect::<Vec<_>>(),
        "outputs": tx.outputs().iter().enumerate().map(|(idx ,out)| tx_out_to_json(out, tx.hash(), idx)).collect::<Vec<_>>()
    })
}

#[cfg(test)]
mod tests {
    use pallas_traverse::MultiEraBlock;

    use super::*;

    #[test]
    fn test_iteration() {
        let blocks = vec![
            include_str!("../../../test_data/byron2.block"),
            include_str!("../../../test_data/shelley1.block"),
            include_str!("../../../test_data/mary1.block"),
            include_str!("../../../test_data/allegra1.block"),
            include_str!("../../../test_data/alonzo1.block"),
        ];

        for block_str in blocks.into_iter() {
            let cbor = hex::decode(block_str).expect("invalid hex");
            let block = MultiEraBlock::decode(&cbor).expect("invalid cbor");
            for tx in block.txs() {
                let json = tx_to_json(&tx);
                println!("{}", json);
            }
        }
    }
}
