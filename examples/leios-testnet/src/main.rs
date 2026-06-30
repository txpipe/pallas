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
//! - **leios-notify** (server-push): the relay announces / offers new EBs and
//!   their transactions, and diffuses full votes inline. Surfaced as
//!   [`InitiatorEvent::EbNotification`].
//! - **leios-fetch** (client-pull): we request the EB body or a subset of its
//!   transactions. Surfaced as [`InitiatorEvent::EbFetched`].
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

use std::collections::HashMap;
use std::time::Duration;

use pallas_codec::minicbor::{Decoder, data::Type};
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
        EbId, Point,
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

/// Chain-sync intersection point to start following from, so we sync near the
/// tip instead of replaying the chain from origin.
///
/// The Musashi testnet resets periodically; if sync stalls or the intersection
/// is not found, replace this with a current point from the chain.
const INTERSECT_SLOT: u64 = 2812236;
const INTERSECT_HASH: &str = "9d8a43aa5ddfa5e2e379ad14b38c3edf98cb6898ed480726fec9da9b68aa3d0e";

/// How many of an EB's transactions to fetch per request. We only request txs a
/// peer has offered (so it holds the closure), and bound each request to one
/// 64-tx bitmap window — requesting a whole large EB at once can exceed the
/// relay's per-response limits. A real client pages across windows as needed.
const MAX_TXS_PER_FETCH: usize = 64;

struct LeiosNode {
    network: Manager<TcpInterface<AnyMessage>, InitiatorBehavior, AnyMessage>,
    housekeeping_interval: Interval,
    /// Transaction count per EB, learned by decoding the EB body. Used to size a
    /// correct tx bitmap when the peer later offers that EB's transactions.
    eb_tx_counts: HashMap<EbId, usize>,
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
            eb_tx_counts: HashMap::new(),
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
                tracing::info!(%pid, ?point, tip = %fmt_eb(&tip.0), "intersection found");
                self.network.execute(InitiatorCommand::ContinueSync(pid));
            }
            InitiatorEvent::BlockHeaderReceived(pid, header, tip) => {
                tracing::info!(%pid, variant = header.variant, tip_block = tip.1, "header received");
                self.network.execute(InitiatorCommand::ContinueSync(pid));
            }
            InitiatorEvent::RollbackReceived(pid, point, tip) => {
                tracing::warn!(%pid, ?point, tip_block = tip.1, "rollback received");
                self.network.execute(InitiatorCommand::ContinueSync(pid));
            }

            // --- Leios: server-pushed announcements / offers (leios-notify) ---
            InitiatorEvent::EbNotification(pid, notification) => {
                self.handle_notification(pid, notification);
            }

            // --- Leios: pulled bodies / txs (leios-fetch) ---
            InitiatorEvent::EbFetched(pid, eb, response) => match response {
                leiosfetch::Response::Block(body) => {
                    // The EB body is a `{ tx_hash => size }` map; its entry count
                    // is the EB's transaction count. Remember it so we can size a
                    // correct tx bitmap once the peer offers the transactions.
                    let n = eb_tx_count(body.raw_bytes());
                    tracing::info!(%pid, eb = %fmt_eb(&eb), bytes = body.raw_bytes().len(), txs = n, "EB body fetched");
                    self.eb_tx_counts.insert(eb, n);
                }
                leiosfetch::Response::BlockTxs { txs } => {
                    let total: usize = txs.iter().map(|tx| tx.raw_bytes().len()).sum();
                    tracing::info!(%pid, eb = %fmt_eb(&eb), count = txs.len(), bytes = total, "EB transactions fetched");
                    for (i, tx) in txs.iter().enumerate() {
                        tracing::debug!(%pid, index = i, bytes = tx.raw_bytes().len(), "  tx");
                    }
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
                // Pull the body to learn the EB's tx count; we fetch the txs
                // later, when the peer offers them (`BlockTxsOffer`).
                self.network.execute(InitiatorCommand::FetchEb(pid, eb_id));
            }
            leiosnotify::Notification::BlockAnnouncement(raw) => {
                tracing::info!(%pid, bytes = raw.raw_bytes().len(), "EB announced");
            }
            leiosnotify::Notification::BlockTxsOffer(eb_id) => {
                // The peer signals it holds this EB's transaction closure, so it
                // can serve a BlockTxsRequest. Requesting txs a peer has NOT
                // offered makes the prototype relay reset the connection, so we
                // only fetch in response to this offer, sized from the body's tx
                // count (learned when we fetched the body).
                match self.eb_tx_counts.get(&eb_id).copied() {
                    Some(n) if n > 0 => {
                        let want = n.min(MAX_TXS_PER_FETCH);
                        tracing::info!(%pid, eb = %fmt_eb(&eb_id), want, total = n, "txs offered → fetching");
                        self.network.execute(InitiatorCommand::FetchEbTxs(
                            pid,
                            eb_id,
                            leiosfetch::Bitmaps::all(want),
                        ));
                    }
                    _ => {
                        tracing::info!(%pid, eb = %fmt_eb(&eb_id), "txs offered (body not yet fetched)");
                    }
                }
            }
            leiosnotify::Notification::Votes(votes) => {
                tracing::info!(%pid, count = votes.len(), "votes received");
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

/// Counts the transactions in an EB body, which is a `{ tx_hash => size }` CBOR
/// map — the number of entries is the transaction count.
fn eb_tx_count(body: &[u8]) -> usize {
    let mut d = Decoder::new(body);
    match d.map() {
        Ok(Some(n)) => n as usize,
        // Indefinite-length map: count key/value pairs until the break marker.
        Ok(None) => {
            let mut n = 0;
            while !matches!(d.datatype(), Ok(Type::Break)) {
                if d.skip().is_err() || d.skip().is_err() {
                    break;
                }
                n += 1;
            }
            n
        }
        Err(_) => 0,
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
    // Start chain-sync near the tip (not origin) so we follow live blocks without
    // replaying the whole chain. The Leios overlay diffuses EBs over the same
    // connection independently of where we are in chain-sync.
    let intersect = Point::Specific(
        INTERSECT_SLOT,
        hex::decode(INTERSECT_HASH).expect("INTERSECT_HASH should be valid hex"),
    );
    node.network
        .execute(InitiatorCommand::StartSync(vec![intersect]));

    node.run_forever().await;
}
