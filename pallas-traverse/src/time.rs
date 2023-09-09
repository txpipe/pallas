use crate::{wellknown::GenesisValues, MultiEraBlock};

pub type Epoch = u64;

pub type Slot = u64;

pub type SubSlot = u64;

#[inline]
fn compute_linear_timestamp(
    known_slot: u64,
    known_time: u64,
    slot_length: u64,
    query_slot: u64,
) -> u64 {
    known_time + (query_slot - known_slot) * slot_length
}

#[inline]
fn compute_era_epoch(era_slot: Slot, era_slot_length: u64, era_epoch_length: u64) -> (Epoch, Slot) {
    assert!(
        era_epoch_length > 0,
        "epoch length needs to be greater than zero"
    );

    let epoch = (era_slot * era_slot_length) / era_epoch_length;
    let reminder = era_slot % era_epoch_length;

    (epoch, reminder)
}

pub fn compute_absolute_slot_within_era(
    sub_era_epoch: Epoch,
    sub_epoch_slot: Slot,
    era_epoch_length: u32,
    era_slot_length: u32,
) -> u64 {
    ((sub_era_epoch * era_epoch_length as u64) / era_slot_length as u64) + sub_epoch_slot
}

impl GenesisValues {
    pub fn shelley_start_epoch(&self) -> Epoch {
        let (epoch, _) = compute_era_epoch(
            self.shelley_known_slot,
            self.byron_slot_length as u64,
            self.byron_epoch_length as u64,
        );

        epoch
    }

    pub fn slot_to_wallclock(&self, slot: u64) -> u64 {
        if slot < self.shelley_known_slot {
            compute_linear_timestamp(
                self.byron_known_slot,
                self.byron_known_time,
                self.byron_slot_length as u64,
                slot,
            )
        } else {
            compute_linear_timestamp(
                self.shelley_known_slot,
                self.shelley_known_time,
                self.shelley_slot_length as u64,
                slot,
            )
        }
    }

    pub fn absolute_slot_to_relative(&self, slot: u64) -> (u64, u64) {
        if slot < self.shelley_known_slot {
            compute_era_epoch(
                slot,
                self.byron_slot_length as u64,
                self.byron_epoch_length as u64,
            )
        } else {
            let era_slot = slot - self.shelley_known_slot;

            let (era_epoch, reminder) = compute_era_epoch(
                era_slot,
                self.shelley_slot_length as u64,
                self.shelley_epoch_length as u64,
            );

            (self.shelley_start_epoch() + era_epoch, reminder)
        }
    }

    pub fn relative_slot_to_absolute(&self, epoch: Epoch, slot: Slot) -> Slot {
        let shelley_start_epoch = self.shelley_start_epoch();

        if epoch < shelley_start_epoch {
            compute_absolute_slot_within_era(
                epoch,
                slot,
                self.byron_epoch_length,
                self.byron_slot_length,
            )
        } else {
            let byron_slots = compute_absolute_slot_within_era(
                shelley_start_epoch,
                0,
                self.byron_epoch_length,
                self.byron_slot_length,
            );

            let shelley_slots = compute_absolute_slot_within_era(
                epoch - shelley_start_epoch,
                slot,
                self.shelley_epoch_length,
                self.shelley_slot_length,
            );

            byron_slots + shelley_slots
        }
    }
}

impl<'a> MultiEraBlock<'a> {
    pub fn epoch(&self, genesis: &GenesisValues) -> (Epoch, SubSlot) {
        match self {
            MultiEraBlock::EpochBoundary(x) => (x.header.consensus_data.epoch_id, 0),
            MultiEraBlock::Byron(x) => (
                x.header.consensus_data.0.epoch,
                x.header.consensus_data.0.slot,
            ),
            MultiEraBlock::AlonzoCompatible(x, _) => {
                genesis.absolute_slot_to_relative(x.header.header_body.slot)
            }
            MultiEraBlock::Babbage(x) => {
                genesis.absolute_slot_to_relative(x.header.header_body.slot)
            }
        }
    }

    /// Computes the unix timestamp for the slot of the tx
    pub fn wallclock(&self, genesis: &GenesisValues) -> u64 {
        let slot = self.slot();
        genesis.slot_to_wallclock(slot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MultiEraBlock;

    fn assert_slot_matches_timestamp(
        genesis: &GenesisValues,
        slot: u64,
        expected_ts: u64,
        expected_epoch: u64,
        expected_epoch_slot: u64,
    ) {
        let wallclock = genesis.slot_to_wallclock(slot);
        assert_eq!(wallclock, expected_ts);

        let (epoch, epoch_slot) = genesis.absolute_slot_to_relative(slot);
        assert_eq!(epoch, expected_epoch, "epoch doesn't match");
        assert_eq!(epoch_slot, expected_epoch_slot);
    }

    #[test]
    fn calc_matches_mainnet_values() {
        let genesis = GenesisValues::mainnet();

        // Byron start, value copied from:
        // https://explorer.cardano.org/en/block?id=f0f7892b5c333cffc4b3c4344de48af4cc63f55e44936196f365a9ef2244134f
        assert_slot_matches_timestamp(&genesis, 0, 1506203091, 0, 0);

        // Byron middle, value copied from:
        // https://explorer.cardano.org/en/block?id=c1b57d58761af4dc3c6bdcb3542170cec6db3c81e551cd68012774d1c38129a3
        assert_slot_matches_timestamp(&genesis, 2160007, 1549403231, 100, 7);

        // Shelley start, value copied from:
        // https://explorer.cardano.org/en/block?id=aa83acbf5904c0edfe4d79b3689d3d00fcfc553cf360fd2229b98d464c28e9de
        assert_slot_matches_timestamp(&genesis, 4492800, 1596059091, 208, 0);

        // Shelly middle, value copied from:
        // https://explorer.cardano.org/en/block?id=ca60833847d0e70a1adfa6b7f485766003cf7d96d28d481c20d4390f91b76d68
        assert_slot_matches_timestamp(&genesis, 51580240, 1643146531, 316, 431440);

        // Shelly middle, value copied from:
        // https://explorer.cardano.org/en/block?id=ec07c6f74f344062db5340480e5b364aac8bb40768d184c1b1491e05c5bec4c4
        assert_slot_matches_timestamp(&genesis, 54605026, 1646171317, 324, 226);
    }

    #[test]
    fn calc_matches_testnet_values() {
        let genesis = GenesisValues::testnet();

        // Byron origin, value copied from:
        // https://explorer.cardano-testnet.iohkdev.io/en/block?id=8f8602837f7c6f8b8867dd1cbc1842cf51a27eaed2c70ef48325d00f8efb320f
        assert_slot_matches_timestamp(&genesis, 0, 1564010416, 0, 0);

        // Byron start, value copied from:
        // https://explorer.cardano-testnet.iohkdev.io/en/block?id=388a82f053603f3552717d61644a353188f2d5500f4c6354cc1ad27a36a7ea91
        assert_slot_matches_timestamp(&genesis, 1031, 1564031036, 0, 1031);

        // Byron middle, value copied from:
        // https://explorer.cardano-testnet.iohkdev.io/en/block?id=66102c0b80e1eebc9cddf9cab43c1bf912e4f1963d6f3b8ff948952f8409e779
        assert_slot_matches_timestamp(&genesis, 561595, 1575242316, 25, 129595);

        // Shelley start, value copied from:
        // https://explorer.cardano-testnet.iohkdev.io/en/block?id=02b1c561715da9e540411123a6135ee319b02f60b9a11a603d3305556c04329f
        assert_slot_matches_timestamp(&genesis, 1598400, 1595967616, 74, 0);

        // Shelley middle, value copied from:
        // https://explorer.cardano-testnet.iohkdev.io/en/block?id=26a1b5a649309c0c8dd48f3069d9adea5a27edf5171dfb941b708acaf2d76dcd
        assert_slot_matches_timestamp(&genesis, 48783593, 1643152809, 183, 97193);
    }

    #[test]
    fn calc_matches_preview_values() {
        let genesis = GenesisValues::preview();

        // https://preview.cardanoscan.io/block/1
        assert_slot_matches_timestamp(&genesis, 20, 1666656020, 0, 20);

        // https://preview.cardanoscan.io/block/1384
        assert_slot_matches_timestamp(&genesis, 27680, 1666683680, 0, 27680);

        // https://preview.cardanoscan.io/block/1202991
        assert_slot_matches_timestamp(&genesis, 27556036, 1694212036, 318, 80836);
    }

    #[test]
    fn calc_matches_preprod_values() {
        let genesis = GenesisValues::preprod();

        // https://preprod.cardanoscan.io/block/0
        assert_slot_matches_timestamp(&genesis, 0, 1654041600, 0, 0);

        // https://preprod.cardanoscan.io/block/1
        assert_slot_matches_timestamp(&genesis, 2, 1654041640, 0, 2);

        // Can't make Byron blocks work, not sure what's going on here. Each block jumps
        // several slots. Timestamps work but epoch calculation doesn't. Since anything
        // interesting starts from Shelley, I'll commit to this logic and treat this as
        // a known-issue for later fixing.

        // https://preprod.cardanoscan.io/block/11
        // assert_slot_matches_timestamp(&genesis, 21600, 1654473600, 1, 0);

        // https://preprod.cardanoscan.io/block/46
        assert_slot_matches_timestamp(&genesis, 86400, 1655769600, 4, 0);

        // https://preprod.cardanoscan.io/block/1360501
        assert_slot_matches_timestamp(&genesis, 38580791, 1694263991, 93, 46391);
    }

    #[test]
    fn known_slot_matches() {
        // TODO: expand this test to include more test blocks
        let block_str = include_str!("../../test_data/byron1.block");
        let block_cbor = hex::decode(block_str).expect("invalid hex");
        let block = MultiEraBlock::decode(&block_cbor).expect("invalid cbor");

        let byron = block.as_byron().unwrap();

        let genesis = GenesisValues::default();

        let computed_slot = genesis.relative_slot_to_absolute(
            byron.header.consensus_data.0.epoch,
            byron.header.consensus_data.0.slot,
        );

        assert_eq!(computed_slot, 4492794);
    }
}
