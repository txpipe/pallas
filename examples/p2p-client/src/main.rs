use std::time::Duration;

use pallas_network2::{
    Manager,
    behavior::{
        AnyMessage,
        initiator::{InitiatorBehavior, InitiatorCommand, InitiatorEvent},
    },
    interface::TcpInterface,
    protocol as proto,
};
use tokio::{select, time::Interval};

struct ClientNode {
    network: Manager<TcpInterface<AnyMessage>, InitiatorBehavior, AnyMessage>,
    housekeeping_interval: Interval,
}

impl ClientNode {
    fn new() -> Self {
        let interface = TcpInterface::new();
        let behavior = InitiatorBehavior::default();
        let network = Manager::new(interface, behavior);

        Self {
            network,
            housekeeping_interval: tokio::time::interval(Duration::from_secs(3)),
        }
    }

    fn handle_event(&mut self, event: InitiatorEvent) {
        match event {
            InitiatorEvent::PeerInitialized(pid, (version, _data)) => {
                tracing::info!(%pid, version, "peer initialized");
            }

            InitiatorEvent::IntersectionFound(pid, point, tip) => {
                tracing::info!(
                    %pid,
                    point = ?point,
                    tip_slot = tip.1,
                    "intersection found"
                );
                self.network
                    .execute(InitiatorCommand::ContinueSync(pid));
            }

            InitiatorEvent::BlockHeaderReceived(pid, header, tip) => {
                tracing::info!(
                    %pid,
                    variant = header.variant,
                    tip_slot = tip.1,
                    "header received"
                );
                self.network
                    .execute(InitiatorCommand::ContinueSync(pid));
            }

            InitiatorEvent::RollbackReceived(pid, point, tip) => {
                tracing::info!(
                    %pid,
                    point = ?point,
                    tip_slot = tip.1,
                    "rollback received"
                );
                self.network
                    .execute(InitiatorCommand::ContinueSync(pid));
            }

            other => {
                tracing::debug!(?other, "unhandled event");
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

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let mut node = ClientNode::new();

    // Add the responder peer and start syncing from Origin
    node.network
        .execute(InitiatorCommand::IncludePeer(
            "127.0.0.1:3000".parse().unwrap(),
        ));
    node.network
        .execute(InitiatorCommand::StartSync(vec![proto::Point::Origin]));

    tracing::info!("connecting to 127.0.0.1:3000");

    node.run_forever().await;
}
