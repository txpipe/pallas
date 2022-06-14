//! Internal supporting utilities

use pallas_primitives::{alonzo, byron};

pub fn clone_alonzo_tx_at<'b>(
    block: &'b alonzo::MintedBlock,
    index: usize,
) -> Option<alonzo::MintedTx<'b>> {
    let transaction_body = block.transaction_bodies.get(index).cloned()?;

    let transaction_witness_set = block.transaction_witness_sets.get(index).cloned()?;

    let success = !block
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

pub fn clone_alonzo_txs<'b>(block: &'b alonzo::MintedBlock) -> Vec<alonzo::MintedTx<'b>> {
    (0..block.transaction_bodies.len())
        .step_by(1)
        .map(|idx| clone_alonzo_tx_at(block, idx))
        .flatten()
        .collect()
}

pub fn clone_byron_txs<'b>(block: &'b byron::MintedBlock) -> Vec<byron::MintedTxPayload<'b>> {
    block.body.tx_payload.iter().cloned().collect()
}
