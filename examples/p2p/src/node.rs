use std::{collections::HashSet, time::Duration};

use futures::{select_biased, FutureExt};
use pallas_network2::{
    behavior::{AnyMessage, InitiatorBehavior, InitiatorCommand, InitiatorEvent},
    protocol::{blockfetch::Body, chainsync::HeaderContent, Point},
    Interface, Manager, PeerId,
};
use tokio::select;

pub struct MyConfig {
    pub chain_intersection: Vec<Point>,
    pub initial_peers: Vec<PeerId>,
}

pub struct MyNode<I: Interface<AnyMessage>> {
    network: Manager<I, InitiatorBehavior, AnyMessage>,
    pending_blocks: HashSet<Point>,
    pub chain: Vec<Body>,
}

impl<I: Interface<AnyMessage>> MyNode<I> {
    pub fn new(config: MyConfig, interface: I) -> Self {
        let mut network = Manager::new(interface, InitiatorBehavior::default());

        for peer in config.initial_peers {
            network.enqueue(InitiatorCommand::IncludePeer(peer));
        }

        for point in config.chain_intersection {
            network.enqueue(InitiatorCommand::StartSync(vec![point]));
        }

        Self {
            network,
            pending_blocks: HashSet::new(),
            chain: Vec::new(),
        }
    }

    fn on_header_received(&mut self, header: HeaderContent) {
        let tag = header.variant;
        let subtag = header.byron_prefix.map(|(x, _)| x);
        let cbor = &header.cbor;

        let header = pallas_traverse::MultiEraHeader::decode(tag, subtag, cbor).unwrap();

        let point = Point::Specific(header.slot(), header.hash().to_vec());

        // naively assume that we haven't seend this block yet
        self.pending_blocks.insert(point);
    }

    fn on_block_body_received(&mut self, body: Body) {
        self.chain.push(body);
    }

    fn fetch_pending_blocks(&mut self) {
        if self.pending_blocks.len() < 20 {
            return;
        }

        let cmd = InitiatorCommand::StopSync;
        self.network.enqueue(cmd);

        let all = self.pending_blocks.drain();

        for point in all {
            let cmd = InitiatorCommand::RequestBlocks((point.clone(), point));
            self.network.enqueue(cmd);
        }
    }

    fn enqueue_next_cmds(&mut self) {
        // fetch whatever blocks we have pending
        self.fetch_pending_blocks();

        // todo other stuff goes here
    }

    fn handle_event(&mut self, event: InitiatorEvent) {
        match event {
            InitiatorEvent::PeerInitialized(pid, _) => {
                tracing::info!(%pid, "peer initialized");
            }
            InitiatorEvent::BlockHeaderReceived(pid, x, _) => {
                tracing::info!(tag = x.variant, %pid, "header received");
                self.on_header_received(x);
            }
            InitiatorEvent::RollbackReceived(pid, p, _) => {
                let slot = p.slot_or_default();
                tracing::info!(%pid, %slot, "rollback received");
            }
            InitiatorEvent::BlockBodyReceived(pid, body) => {
                tracing::info!(%pid, "block body received");
                self.on_block_body_received(body);
            }
            InitiatorEvent::TxRequested(pid, _) => {
                tracing::info!(%pid, "tx requested");
            }
        }

        self.enqueue_next_cmds();
    }

    async fn tick(&mut self) {
        select_biased! {
            _ = tokio::time::sleep(Duration::from_secs(3)).fuse() => {
                self.network.enqueue(InitiatorCommand::Housekeeping);
            }
            evt = self.network.poll_next().fuse() => {
                if let Some(evt) = evt {
                    self.handle_event(evt);
                }
            }
        }
    }

    pub async fn run_forever(&mut self) {
        loop {
            self.tick().await;
            tokio::task::yield_now().await;
        }
    }

    pub async fn download_chain(&mut self, block_count: usize) -> Vec<Body> {
        loop {
            self.tick().await;
            tokio::task::yield_now().await;

            if self.chain.len() > block_count {
                break;
            }
        }

        self.chain.drain(..block_count).collect()
    }
}
