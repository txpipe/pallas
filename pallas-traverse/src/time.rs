// TODO: is it safe to hardcode these values?
const WELLKNOWN_SLOT_LENGTH: u64 = 20; // 20 secs
const WELLKNOWN_EPOCH_LENGTH: u64 = 5 * 24 * 60 * 60; // 5 days

pub fn byron_epoch_slot_to_absolute(epoch: u64, sub_epoch_slot: u64) -> u64 {
    ((epoch * WELLKNOWN_EPOCH_LENGTH) / WELLKNOWN_SLOT_LENGTH) + sub_epoch_slot
}

#[cfg(test)]
mod tests {
    use pallas_codec::minicbor;

    use crate::{byron::Block, time::byron_epoch_slot_to_absolute};

    type BlockWrapper = (u16, Block);

    #[test]
    fn knwon_slot_matches() {
        // TODO: expand this test to include more test blocks
        let block_idx = 1;
        let block_str = include_str!("../../test_data/byron1.block");

        let block_bytes = hex::decode(block_str).expect(&format!("bad block file {}", block_idx));
        let (_, block): BlockWrapper = minicbor::decode(&block_bytes[..])
            .expect(&format!("error decoding cbor for file {}", block_idx));

        let computed_slot = byron_epoch_slot_to_absolute(
            block.header.consensus_data.0.epoch,
            block.header.consensus_data.0.slot,
        );

        assert_eq!(computed_slot, 4492794);
    }
}
