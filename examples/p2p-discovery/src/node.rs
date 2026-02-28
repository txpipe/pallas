use std::{collections::HashSet, time::Duration};

use pallas_network2::{
    behavior::{
        AnyMessage, InitiatorBehavior, InitiatorCommand, InitiatorEvent, PromotionBehavior,
    },
    protocol::{blockfetch::Body, chainsync::HeaderContent},
    Interface, Manager,
};

pub use pallas_network2::behavior::PromotionConfig;
pub use pallas_network2::protocol::Point;
pub use pallas_network2::PeerId;

use pallas_crypto::hash::Hash;
use pallas_traverse::MultiEraBlock;
use tokio::{select, time::Interval};

pub struct MyConfig {
    pub chain_intersection: Vec<Point>,
    pub initial_peers: Vec<String>,
    pub promotion: PromotionConfig,
}

pub struct MyNode<I: Interface<AnyMessage>> {
    network: Manager<I, InitiatorBehavior, AnyMessage>,
    pending_blocks: HashSet<Point>,
    housekeeping_interval: Interval,
    initial_peers: Vec<String>,
    chain_intersection: Vec<Point>,
    downloaded_blocks: HashSet<Hash<32>>,
}

impl<I: Interface<AnyMessage>> MyNode<I> {
    pub fn new(config: MyConfig, interface: I) -> Self {
        let behavior = InitiatorBehavior {
            promotion: PromotionBehavior::new(config.promotion),
            ..Default::default()
        };

        let network = Manager::new(interface, behavior);

        Self {
            network,
            initial_peers: config.initial_peers,
            chain_intersection: config.chain_intersection,
            housekeeping_interval: tokio::time::interval(Duration::from_secs(3)),
            pending_blocks: HashSet::new(),
            downloaded_blocks: HashSet::new(),
        }
    }

    fn on_header_received(&mut self, pid: PeerId, header: HeaderContent) {
        let tag = header.variant;
        let subtag = header.byron_prefix.map(|(x, _)| x);
        let cbor = &header.cbor;

        let header = pallas_traverse::MultiEraHeader::decode(tag, subtag, cbor).unwrap();

        let hash = header.hash();

        if self.downloaded_blocks.contains(&hash) {
            tracing::debug!(%pid, %hash, "block already downloaded");
        } else {
            let point = Point::Specific(header.slot(), header.hash().to_vec());
            self.pending_blocks.insert(point);
        }

        // we naively continue sync after we see a header. This is not a good idea in
        // production code, there's no backpressure here so you might end up with lots
        // of new headers coming in without any way to process them in a timely manner.
        self.network.execute(InitiatorCommand::ContinueSync(pid));
    }

    fn on_block_body_received(&mut self, pid: PeerId, body: Body) {
        let block = MultiEraBlock::decode(&body).unwrap();

        tracing::info!(%pid, slot = block.slot(), "block body received");

        let hash = block.hash();

        self.downloaded_blocks.insert(hash);
    }

    fn fetch_pending_blocks(&mut self) {
        if self.pending_blocks.len() < 20 {
            return;
        }

        let all = self.pending_blocks.drain().collect::<Vec<_>>();

        let start = all
            .iter()
            .min_by_key(|p| p.slot_or_default())
            .cloned()
            .unwrap();

        let end = all
            .iter()
            .max_by_key(|p| p.slot_or_default())
            .cloned()
            .unwrap();

        tracing::info!(
            start = start.slot_or_default(),
            end = end.slot_or_default(),
            "requesting block range"
        );

        let cmd = InitiatorCommand::RequestBlocks((start, end));

        self.network.execute(cmd);
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
            InitiatorEvent::IntersectionFound(pid, _, _) => {
                tracing::info!(%pid, "intersection found");
                self.network.execute(InitiatorCommand::ContinueSync(pid));
            }
            InitiatorEvent::BlockHeaderReceived(pid, x, _) => {
                tracing::info!(tag = x.variant, %pid, "header received");
                self.on_header_received(pid, x);
            }
            InitiatorEvent::RollbackReceived(pid, p, _) => {
                let slot = p.slot_or_default();
                tracing::info!(%pid, %slot, "rollback received");
                self.network.execute(InitiatorCommand::ContinueSync(pid));
            }
            InitiatorEvent::BlockBodyReceived(pid, body) => {
                self.on_block_body_received(pid, body);
            }
            InitiatorEvent::TxRequested(pid, _) => {
                tracing::info!(%pid, "tx requested");
            }
        }

        self.enqueue_next_cmds();
    }

    async fn tick(&mut self) {
        select! {
            _ = self.housekeeping_interval.tick() => {
                tracing::debug!("housekeeping tick");
                self.network.execute(InitiatorCommand::Housekeeping);
            }
            evt = self.network.poll_next() => {
                if let Some(evt) = evt {
                    self.handle_event(evt);
                }
            }
        }
    }

    fn initialize(&mut self) -> anyhow::Result<()> {
        for peer in self.initial_peers.iter() {
            let peer = peer.parse::<PeerId>().map_err(|e| anyhow::anyhow!(e))?;
            self.network.execute(InitiatorCommand::IncludePeer(peer));
        }

        self.network
            .execute(InitiatorCommand::StartSync(self.chain_intersection.clone()));

        Ok(())
    }

    pub async fn run_forever(&mut self) -> anyhow::Result<()> {
        self.initialize()?;

        loop {
            self.tick().await;
        }
    }
}
