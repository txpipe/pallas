//! A terminal dashboard for a Leios initiator node observing the chain.
//!
//! Connects to the Cardano **Leios** ("Musashi Dojo") testnet, negotiates the
//! v15 (Leios) handshake, follows Praos chain-sync, and fetches Endorser Blocks
//! over leios-notify / leios-fetch — the same flow as the `leios-testnet`
//! example, but rendered as a live TUI instead of log lines.
//!
//! The screen renders ranking blocks (RBs) and endorser blocks (EBs) as two
//! vertically-aligned swim lanes sharing one column axis: the newest RBs define
//! the columns (newest at the tip, right), and each EB is drawn in the column of
//! the RB it belongs to — so the shared column *is* the connection. Each EB box
//! shows its transaction download and vote accumulation as mini progress bars,
//! with full figures for the selected EB in a detail strip; a log panel sits
//! below. The EB→RB association is a slot heuristic (no on-wire link exists).
//!
//! Run with:
//!
//! ```sh
//! cargo run -p leios-tui
//! ```
//!
//! Keys: `q` quit · `←`/`→` select EB · `f` toggle follow-tip · `c` clear log.

mod dashboard;
mod logbuf;
mod ui;

use std::time::Duration;

use crossterm::event::EventStream;
use futures::StreamExt;
use pallas_network2::{
    Manager,
    behavior::{
        AnyMessage,
        initiator::{
            Config as HandshakeConfig, HandshakeBehavior, InitiatorBehavior, InitiatorCommand,
            InitiatorEvent,
        },
    },
    interface::TcpInterface,
    protocol::{Point, handshake::n2n::VersionTable},
};
use ratatui::DefaultTerminal;
use tokio::{select, time::Interval};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use dashboard::{Action, Dashboard};

/// Public bootstrap relay for the Leios "Musashi Dojo" testnet.
const LEIOS_RELAY: &str = "leios-node.play.dev.cardano.org:3001";

/// Network magic for the Leios testnet.
const LEIOS_TESTNET_MAGIC: u64 = 164;

/// Chain-sync intersection point, so we follow near the tip instead of replaying
/// from origin. The testnet resets periodically; replace with a current point if
/// the intersection is not found.
const INTERSECT_SLOT: u64 = 2889961;
const INTERSECT_HASH: &str = "f0221534bd8fa9ec6c7b8c36348718b6a382c40cc39824681a2003af9c820eeb";

struct LeiosNode {
    network: Manager<TcpInterface<AnyMessage>, InitiatorBehavior, AnyMessage>,
    housekeeping_interval: Interval,
    render_interval: Interval,
    input: EventStream,
    dashboard: Dashboard,
}

impl LeiosNode {
    fn new(dashboard: Dashboard) -> Self {
        let interface = TcpInterface::new();

        // Propose v11..=v15 with the testnet magic so the peer can negotiate v15
        // and enable the Leios mini-protocols.
        let behavior = InitiatorBehavior {
            handshake: HandshakeBehavior::new(HandshakeConfig {
                supported_version: VersionTable::v11_and_above_with_query(
                    LEIOS_TESTNET_MAGIC,
                    false,
                ),
            }),
            ..Default::default()
        };

        Self {
            network: Manager::new(interface, behavior),
            housekeeping_interval: tokio::time::interval(Duration::from_secs(3)),
            render_interval: tokio::time::interval(Duration::from_millis(200)),
            input: EventStream::new(),
            dashboard,
        }
    }

    /// Folds an event into the dashboard and issues any resulting commands.
    fn handle_event(&mut self, event: InitiatorEvent) {
        for action in self.dashboard.apply_event(&event) {
            match action {
                Action::ContinueSync(pid) => {
                    self.network.execute(InitiatorCommand::ContinueSync(pid))
                }
                Action::FetchEb(pid, eb) => {
                    self.network.execute(InitiatorCommand::FetchEb(pid, eb))
                }
                Action::FetchEbTxs(pid, eb, bitmaps) => self
                    .network
                    .execute(InitiatorCommand::FetchEbTxs(pid, eb, bitmaps)),
            }
        }
    }

    async fn run(&mut self, terminal: &mut DefaultTerminal) -> std::io::Result<()> {
        terminal.draw(|f| ui::draw(f, &self.dashboard))?;

        loop {
            select! {
                _ = self.housekeeping_interval.tick() => {
                    self.network.execute(InitiatorCommand::Housekeeping);
                }
                _ = self.render_interval.tick() => {
                    terminal.draw(|f| ui::draw(f, &self.dashboard))?;
                }
                evt = self.network.poll_next() => {
                    if let Some(evt) = evt {
                        self.handle_event(evt);
                    }
                }
                ev = self.input.next() => {
                    if let Some(Ok(ev)) = ev
                        && self.dashboard.handle_input(ev)
                    {
                        return Ok(());
                    }
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let log = logbuf::new_log();
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(logbuf::LogLayer::new(log.clone()))
        .init();

    let dashboard = Dashboard::new(LEIOS_RELAY.to_string(), LEIOS_TESTNET_MAGIC, log);
    let mut node = LeiosNode::new(dashboard);

    let peer = LEIOS_RELAY
        .parse()
        .expect("LEIOS_RELAY should be a valid host:port");

    tracing::info!(
        relay = LEIOS_RELAY,
        magic = LEIOS_TESTNET_MAGIC,
        "connecting to Leios testnet"
    );

    node.network.execute(InitiatorCommand::IncludePeer(peer));
    let intersect = Point::Specific(
        INTERSECT_SLOT,
        hex::decode(INTERSECT_HASH).expect("INTERSECT_HASH should be valid hex"),
    );
    node.network
        .execute(InitiatorCommand::StartSync(vec![intersect]));

    let mut terminal = ratatui::init();
    let result = node.run(&mut terminal).await;
    ratatui::restore();
    result
}
