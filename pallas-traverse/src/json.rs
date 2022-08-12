use pallas_primitives::{alonzo, babbage};
use serde_json::Value;

use crate::MultiEraBlock;

impl<'b> MultiEraBlock<'b> {
    pub fn to_json(self) -> serde_json::Result<Value> {
        match self {
            MultiEraBlock::AlonzoCompatible(x, _) => {
                let standalone: alonzo::Block = (*x).into();
                serde_json::to_value(standalone)
            }
            MultiEraBlock::Babbage(x) => {
                let standalone: babbage::Block = (*x).into();
                serde_json::to_value(standalone)
            }
            _ => Ok(Value::Null),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iteration() {
        let blocks = vec![
            include_str!("../../test_data/shelley1.block"),
            include_str!("../../test_data/mary1.block"),
            include_str!("../../test_data/allegra1.block"),
            include_str!("../../test_data/alonzo1.block"),
        ];

        for block_str in blocks.into_iter() {
            let cbor = hex::decode(block_str).expect("invalid hex");
            let block = MultiEraBlock::decode(&cbor).expect("invalid cbor");

            dbg!(block.to_json().unwrap());
        }
    }
}
