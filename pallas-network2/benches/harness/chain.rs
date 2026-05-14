use pallas_network2::protocol::{Point, blockfetch, chainsync};

/// A mock chain data provider for benchmark responder nodes.
pub struct MockChain {
    slot: u64,
}

impl MockChain {
    pub fn new() -> Self {
        Self { slot: 0 }
    }

    pub fn tip(&self) -> chainsync::Tip {
        chainsync::Tip(Point::new(self.slot, vec![0xAA; 32]), self.slot)
    }

    pub fn next_header(&mut self) -> (chainsync::HeaderContent, chainsync::Tip) {
        self.slot += 1;

        let header = chainsync::HeaderContent {
            variant: 1,
            byron_prefix: None,
            cbor: vec![0xBE; 32],
        };

        (header, self.tip())
    }

    pub fn blocks(&self, count: usize) -> Vec<blockfetch::Body> {
        (0..count).map(|i| vec![0xDE; 64 + i]).collect()
    }
}

impl Default for MockChain {
    fn default() -> Self {
        Self::new()
    }
}
