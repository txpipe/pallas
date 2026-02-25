use std::{net::Ipv4Addr, time::Duration};

use pallas_network2::{
    behavior::{
        responder::{
            ResponderBehavior, ResponderCommand, ResponderEvent,
            connection::{ConnectionResponder, ConnectionResponderConfig},
        },
        AnyMessage,
    },
    interface::TcpListenerInterface,
    protocol::{self as proto, chainsync, peersharing},
    Manager,
};
use tokio::{select, time::Interval};

struct MockResponderNode {
    network: Manager<TcpListenerInterface<AnyMessage>, ResponderBehavior, AnyMessage>,
    housekeeping_interval: Interval,
    mock_slot: u64,
}

impl MockResponderNode {
    fn new(listener: tokio::net::TcpListener) -> Self {
        let interface = TcpListenerInterface::new(listener);
        let behavior = ResponderBehavior {
            connection: ConnectionResponder::new(ConnectionResponderConfig {
                max_connections_per_ip: 3,
                ..Default::default()
            }),
            ..Default::default()
        };
        let network = Manager::new(interface, behavior);

        Self {
            network,
            housekeeping_interval: tokio::time::interval(Duration::from_secs(3)),
            mock_slot: 1,
        }
    }

    fn mock_tip(&self) -> chainsync::Tip {
        chainsync::Tip(
            proto::Point::Specific(self.mock_slot + 100, vec![0xFF; 32]),
            self.mock_slot + 100,
        )
    }

    async fn handle_event(&mut self, event: ResponderEvent) {
        match event {
            ResponderEvent::PeerInitialized(pid, (version, _data)) => {
                tracing::info!(%pid, version, "peer initialized");
            }

            ResponderEvent::IntersectionRequested(pid, points) => {
                tracing::info!(%pid, num_points = points.len(), "intersection requested");

                let point = proto::Point::Origin;
                let tip = self.mock_tip();
                self.network
                    .execute(ResponderCommand::ProvideIntersection(pid, point, tip));
            }

            ResponderEvent::NextHeaderRequested(pid) => {
                tokio::time::sleep(Duration::from_millis(500)).await;

                let slot = self.mock_slot;
                self.mock_slot += 1;

                tracing::info!(%pid, slot, "header requested");

                let header = chainsync::HeaderContent {
                    variant: 6, // post-shelley era
                    byron_prefix: None,
                    cbor: vec![0x00; 32], // mock header bytes
                };
                let tip = self.mock_tip();
                self.network
                    .execute(ResponderCommand::ProvideHeader(pid, header, tip));
            }

            ResponderEvent::BlockRangeRequested(pid, (start, end)) => {
                tracing::info!(
                    %pid,
                    start = start.slot_or_default(),
                    end = end.slot_or_default(),
                    "block range requested"
                );

                let blocks: Vec<Vec<u8>> = (0..3).map(|i| vec![0xBE; 64 + i]).collect();
                self.network
                    .execute(ResponderCommand::ProvideBlocks(pid, blocks));
            }

            ResponderEvent::PeersRequested(pid, amount) => {
                tracing::info!(%pid, amount, "peers requested");

                let peers: Vec<peersharing::PeerAddress> = (0..amount.min(5))
                    .map(|i| {
                        peersharing::PeerAddress::V4(
                            Ipv4Addr::new(192, 168, 1, 100 + i),
                            3001 + i as u16,
                        )
                    })
                    .collect();
                self.network
                    .execute(ResponderCommand::ProvidePeers(pid, peers));
            }

            ResponderEvent::TxReceived(pid, _tx) => {
                tracing::info!(%pid, "tx received");
            }
        }
    }

    async fn tick(&mut self) {
        select! {
            _ = self.housekeeping_interval.tick() => {
                tracing::debug!("housekeeping tick");
                self.network.execute(ResponderCommand::Housekeeping);
            }
            evt = self.network.poll_next() => {
                if let Some(evt) = evt {
                    self.handle_event(evt).await;
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

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("failed to bind TCP listener");

    tracing::info!(
        addr = %listener.local_addr().unwrap(),
        "listening for inbound connections"
    );

    let mut node = MockResponderNode::new(listener);
    node.run_forever().await;
}
