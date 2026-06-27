//! Connect a node to the Cardano **Leios** ("Musashi Dojo") testnet and follow
//! its endorsement layer.
//!
//! Leios (CIP-0164) is an overlay on top of Ouroboros Praos: alongside the
//! normal Praos chain, producers diffuse **Endorser Blocks (EBs)** — compact
//! lists of transaction references — which are then voted on and certified back
//! into a ranking block. Pallas speaks this overlay through two node-to-node
//! mini-protocols that ride the *same* connection as Praos once a Leios-capable
//! handshake version is negotiated:
//!
//! - **leios-notify** (server-push): the relay announces / offers new EBs,
//!   their transactions and votes. Surfaced as [`InitiatorEvent::EbNotification`].
//! - **leios-fetch** (client-pull): we request the EB body, a subset of its
//!   transactions, or specific votes. Surfaced as [`InitiatorEvent::EbFetched`].
//!
//! The only thing that "turns Leios on" is the handshake: we must propose an
//! N2N version `>= LEIOS_MIN_VERSION` (v15, the Dijkstra era) with the testnet's
//! network magic. The default [`InitiatorBehavior`] only proposes a mainnet v13
//! handshake, so this example swaps in a v15-capable version table.
//!
//! Run with:
//!
//! ```sh
//! RUST_LOG=info cargo run -p leios-testnet
//! ```

use std::time::Duration;

use pallas_network2::{
    Manager, PeerId,
    behavior::{
        AnyMessage,
        initiator::{
            Config as HandshakeConfig, HandshakeBehavior, InitiatorBehavior, InitiatorCommand,
            InitiatorEvent,
        },
    },
    interface::TcpInterface,
    protocol::{
        Point,
        handshake::n2n::{LEIOS_MIN_VERSION, VersionTable},
        leiosfetch, leiosnotify,
    },
};
use tokio::{select, time::Interval};

/// Public bootstrap relay for the Leios "Musashi Dojo" testnet.
///
/// This is a throwaway, continuously-resetting devnet — if the connection is
/// refused, check <https://leios.cardano-scaling.org/docs/testnet/getting-started/>
/// for the current relay address and network magic.
const LEIOS_RELAY: &str = "leios-node.play.dev.cardano.org:3001";

/// Network magic for the Leios testnet (a nod to CIP-0164).
const LEIOS_TESTNET_MAGIC: u64 = 164;

struct LeiosNode {
    network: Manager<TcpInterface<AnyMessage>, InitiatorBehavior, AnyMessage>,
    housekeeping_interval: Interval,
}

impl LeiosNode {
    fn new() -> Self {
        let interface = TcpInterface::new();

        // The default behavior only proposes a mainnet v13 handshake, which does
        // not carry the Leios overlay. Swap in a version table that proposes
        // v11..=v15 with the testnet magic so the peer can negotiate v15 and
        // enable leios-notify / leios-fetch. The rest of the behavior (chain-sync,
        // block-fetch, keepalive, ...) is left at its defaults.
        let behavior = InitiatorBehavior {
            handshake: HandshakeBehavior::new(HandshakeConfig {
                supported_version: VersionTable::v11_and_above_with_query(
                    LEIOS_TESTNET_MAGIC,
                    false,
                ),
            }),
            ..Default::default()
        };

        let network = Manager::new(interface, behavior);

        Self {
            network,
            housekeeping_interval: tokio::time::interval(Duration::from_secs(3)),
        }
    }

    fn handle_event(&mut self, event: InitiatorEvent) {
        match event {
            InitiatorEvent::PeerInitialized(pid, (version, _data)) => {
                let leios = version >= LEIOS_MIN_VERSION;
                tracing::info!(%pid, version, leios, "peer initialized");
                if !leios {
                    tracing::warn!(
                        %pid,
                        version,
                        min = LEIOS_MIN_VERSION,
                        "peer negotiated a pre-Leios version; no EBs will be diffused"
                    );
                }
            }

            // --- Praos chain-sync (runs underneath Leios) ---
            InitiatorEvent::IntersectionFound(pid, point, tip) => {
                tracing::info!(%pid, ?point, tip_slot = tip.1, "intersection found");
                self.network.execute(InitiatorCommand::ContinueSync(pid));
            }
            InitiatorEvent::BlockHeaderReceived(pid, header, tip) => {
                tracing::debug!(%pid, variant = header.variant, tip_slot = tip.1, "header received");
                self.network.execute(InitiatorCommand::ContinueSync(pid));
            }
            InitiatorEvent::RollbackReceived(pid, point, tip) => {
                tracing::debug!(%pid, ?point, tip_slot = tip.1, "rollback received");
                self.network.execute(InitiatorCommand::ContinueSync(pid));
            }

            // --- Leios: server-pushed announcements / offers (leios-notify) ---
            InitiatorEvent::EbNotification(pid, notification) => {
                self.handle_notification(pid, notification);
            }

            // --- Leios: pulled bodies / txs / votes (leios-fetch) ---
            InitiatorEvent::EbFetched(pid, response) => match response {
                leiosfetch::Response::Block(body) => {
                    tracing::info!(%pid, bytes = body.0.len(), "EB body fetched");
                }
                leiosfetch::Response::BlockTxs { txs, .. } => {
                    tracing::info!(%pid, count = txs.len(), "EB transactions fetched");
                }
                leiosfetch::Response::Votes(votes) => {
                    tracing::info!(%pid, count = votes.len(), "votes fetched");
                }
            },

            other => {
                tracing::debug!(?other, "unhandled event");
            }
        }
    }

    fn handle_notification(&mut self, pid: PeerId, notification: leiosnotify::Notification) {
        match notification {
            leiosnotify::Notification::BlockOffer(eb_id, size) => {
                tracing::info!(%pid, eb = %fmt_eb(&eb_id), size, "EB offered → fetching body");
                // Showcase the notify → fetch round-trip: pull the offered body.
                self.network.execute(InitiatorCommand::FetchEb(pid, eb_id));
            }
            leiosnotify::Notification::BlockAnnouncement(raw) => {
                tracing::info!(%pid, bytes = raw.0.len(), "EB announced");
            }
            leiosnotify::Notification::BlockTxsOffer(eb_id) => {
                tracing::info!(%pid, eb = %fmt_eb(&eb_id), "EB transactions offered");
            }
            leiosnotify::Notification::VotesOffer(votes) => {
                tracing::info!(%pid, count = votes.len(), "votes offered");
            }
        }
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

    async fn run_forever(&mut self) {
        loop {
            self.tick().await;
        }
    }
}

/// Formats an EB reference (`[slot, hash]`) for logging.
fn fmt_eb(eb: &Point) -> String {
    match eb {
        Point::Origin => "origin".to_string(),
        Point::Specific(slot, hash) => format!("{slot}@{}", hex::encode(hash)),
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let mut node = LeiosNode::new();

    let peer = LEIOS_RELAY
        .parse()
        .expect("LEIOS_RELAY should be a valid host:port");

    tracing::info!(
        relay = LEIOS_RELAY,
        magic = LEIOS_TESTNET_MAGIC,
        "connecting to Leios testnet"
    );

    node.network.execute(InitiatorCommand::IncludePeer(peer));
    // Start Praos chain-sync from origin; the Leios overlay diffuses EBs over the
    // same connection independently of where we are in chain-sync.
    node.network
        .execute(InitiatorCommand::StartSync(vec![Point::Origin]));

    node.run_forever().await;
}
