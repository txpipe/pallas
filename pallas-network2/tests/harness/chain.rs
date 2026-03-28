use pallas_network2::protocol::{blockfetch, chainsync, Point};

/// A mock chain data provider for test responder nodes.
///
/// Produces synthetic chain data (tips, headers, blocks) on demand.
/// Each call to `next_header` advances the slot, simulating chain growth.
pub struct MockChain {
    slot: u64,
}

impl MockChain {
    pub fn new() -> Self {
        Self { slot: 0 }
    }

    /// Returns the current chain tip.
    pub fn tip(&self) -> chainsync::Tip {
        chainsync::Tip(Point::new(self.slot, vec![0xAA; 32]), self.slot)
    }

    /// Advances the chain by one slot and returns the next header with the new tip.
    pub fn next_header(&mut self) -> (chainsync::HeaderContent, chainsync::Tip) {
        self.slot += 1;

        let header = chainsync::HeaderContent {
            variant: 1,
            byron_prefix: None,
            cbor: vec![0xBE; 32],
        };

        (header, self.tip())
    }

    /// Returns a list of synthetic block bodies.
    pub fn blocks(&self, count: usize) -> Vec<blockfetch::Body> {
        (0..count).map(|i| vec![0xDE; 64 + i]).collect()
    }
}

impl Default for MockChain {
    fn default() -> Self {
        Self::new()
    }
}
