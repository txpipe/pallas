//! Internal supporting utilities

use pallas_primitives::{alonzo, babbage, byron, conway};

macro_rules! clone_tx_fn {
    ($fn_name:ident, $era:tt, $block_type:ident, $tx_type:ident) => {
        fn $fn_name<'b>(block: &'b $era::$block_type, index: usize) -> Option<$era::$tx_type<'b>> {
            let transaction_body = block.transaction_bodies.get(index).cloned()?;

            let transaction_witness_set = block.transaction_witness_sets.get(index)?.clone();

            let success = !block
                .invalid_transactions
                .as_ref()
                .map(|x| x.contains(&(index as u32)))
                .unwrap_or(false);

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
                .cloned()
                .into();

            let x = $era::$tx_type {
                transaction_body,
                transaction_witness_set,
                success,
                auxiliary_data,
            };

            Some(x)
        }
    };
}

clone_tx_fn!(conway_clone_tx_at, conway, MintedBlock, MintedTx);
clone_tx_fn!(
    conway_clone_tx_with_raw_aux_at,
    conway,
    MintedBlockWithRawAuxiliary,
    MintedTxWithRawAuxiliary
);
clone_tx_fn!(babbage_clone_tx_at, babbage, MintedBlock, MintedTx);
clone_tx_fn!(
    babbage_clone_tx_with_raw_aux_at,
    babbage,
    MintedBlockWithRawAuxiliary,
    MintedTxWithRawAuxiliary
);
clone_tx_fn!(alonzo_clone_tx_at, alonzo, MintedBlock, MintedTx);
clone_tx_fn!(
    alonzo_clone_tx_with_raw_aux_at,
    alonzo,
    MintedBlockWithRawAuxiliary,
    MintedTxWithRawAuxiliary
);

pub fn clone_alonzo_txs<'b>(block: &'b alonzo::MintedBlock) -> Vec<alonzo::MintedTx<'b>> {
    (0..block.transaction_bodies.len())
        .step_by(1)
        .filter_map(|idx| alonzo_clone_tx_at(block, idx))
        .collect()
}

pub fn clone_alonzo_txs_with_raw_aux<'b>(
    block: &'b alonzo::MintedBlockWithRawAuxiliary,
) -> Vec<alonzo::MintedTxWithRawAuxiliary<'b>> {
    (0..block.transaction_bodies.len())
        .step_by(1)
        .filter_map(|idx| alonzo_clone_tx_with_raw_aux_at(block, idx))
        .collect()
}

pub fn clone_babbage_txs<'b>(block: &'b babbage::MintedBlock) -> Vec<babbage::MintedTx<'b>> {
    (0..block.transaction_bodies.len())
        .step_by(1)
        .filter_map(|idx| babbage_clone_tx_at(block, idx))
        .collect()
}

pub fn clone_babbage_txs_with_raw_aux<'b>(
    block: &'b babbage::MintedBlockWithRawAuxiliary,
) -> Vec<babbage::MintedTxWithRawAuxiliary<'b>> {
    (0..block.transaction_bodies.len())
        .step_by(1)
        .filter_map(|idx| babbage_clone_tx_with_raw_aux_at(block, idx))
        .collect()
}

pub fn clone_conway_txs<'b>(block: &'b conway::MintedBlock) -> Vec<conway::MintedTx<'b>> {
    (0..block.transaction_bodies.len())
        .step_by(1)
        .filter_map(|idx| conway_clone_tx_at(block, idx))
        .collect()
}

pub fn clone_conway_txs_with_raw_aux<'b>(
    block: &'b conway::MintedBlockWithRawAuxiliary,
) -> Vec<conway::MintedTxWithRawAuxiliary<'b>> {
    (0..block.transaction_bodies.len())
        .step_by(1)
        .filter_map(|idx| conway_clone_tx_with_raw_aux_at(block, idx))
        .collect()
}

pub fn clone_byron_txs<'b>(block: &'b byron::MintedBlock) -> Vec<byron::MintedTxPayload<'b>> {
    block.body.tx_payload.iter().cloned().collect()
}
