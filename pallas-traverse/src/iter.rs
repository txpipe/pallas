//! Iterate over block data

use pallas_primitives::{alonzo, byron};

use crate::{MultiEraBlock, MultiEraTx};

fn clone_alonzo_tx_at<'b>(
    block: &'b alonzo::MintedBlock,
    index: usize,
) -> Option<alonzo::MintedTx<'b>> {
    let transaction_body = block.transaction_bodies.get(index).cloned()?;
    let transaction_witness_set = block.transaction_witness_sets.get(index).cloned()?;
    let success = block
        .invalid_transactions
        .as_ref()?
        .contains(&(index as u32));

    let auxiliary_data = block
        .auxiliary_data_set
        .iter()
        .find_map(|(idx, val)| {
            if idx.eq(&(index as u32)) {
                Some(val)
            } else {
                None
            }
        })
        .cloned();

    Some(alonzo::MintedTx {
        transaction_body,
        transaction_witness_set,
        success,
        auxiliary_data,
    })
}

fn clone_byron_tx_at<'b>(
    block: &'b byron::MintedBlock,
    index: usize,
) -> Option<byron::MintedTxPayload<'b>> {
    block.body.tx_payload.get(index).cloned()
}

pub struct TxIter<'b> {
    block: &'b MultiEraBlock<'b>,
    index: usize,
}

impl<'b> Iterator for TxIter<'b> {
    type Item = MultiEraTx<'b>;

    fn next(&mut self) -> Option<Self::Item> {
        let tx = match self.block {
            MultiEraBlock::EpochBoundary(_) => None,
            MultiEraBlock::AlonzoCompatible(x) => {
                clone_alonzo_tx_at(x, self.index).map(MultiEraTx::from_alonzo_compatible)
            }
            MultiEraBlock::Byron(x) => clone_byron_tx_at(x, self.index).map(MultiEraTx::from_byron),
        }?;

        self.index += 1;
        Some(tx)
    }
}

impl<'b> MultiEraBlock<'b> {
    pub fn tx_iter(&'b self) -> TxIter<'b> {
        TxIter {
            index: 0,
            block: self,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iteration() {
        let blocks = vec![
            (include_str!("../../test_data/byron2.block"), 2usize),
            (include_str!("../../test_data/shelley1.block"), 0),
            (include_str!("../../test_data/mary1.block"), 0),
            (include_str!("../../test_data/allegra1.block"), 0),
            (include_str!("../../test_data/alonzo1.block"), 5),
        ];

        for (block_str, tx_count) in blocks.into_iter() {
            let cbor = hex::decode(block_str).expect("invalid hex");
            let block = MultiEraBlock::decode(&cbor).expect("invalid cbor");
            assert_eq!(block.tx_iter().count(), tx_count);
        }
    }
}
