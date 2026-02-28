use std::collections::HashSet;
use std::net::Ipv4Addr;
use std::time::Duration;

use pallas_network2::behavior::AnyMessage;
use pallas_network2::behavior::responder::{ResponderBehavior, ResponderCommand, ResponderEvent};
use pallas_network2::emulation::initiator_mock::MockInitiatorInterface;
use pallas_network2::protocol::{self as proto, chainsync, peersharing};
use pallas_network2::{Manager, PeerId};

struct TestResponderNode {
    network: Manager<MockInitiatorInterface, ResponderBehavior, AnyMessage>,
    initialized_peers: HashSet<PeerId>,
    disconnected_peers: HashSet<PeerId>,
    mock_slot: u64,
}

impl TestResponderNode {
    fn new(interface: MockInitiatorInterface) -> Self {
        let behavior = ResponderBehavior::default();
        let network = Manager::new(interface, behavior);

        Self {
            network,
            initialized_peers: HashSet::new(),
            disconnected_peers: HashSet::new(),
            mock_slot: 1,
        }
    }

    fn mock_tip(&self) -> chainsync::Tip {
        chainsync::Tip(
            proto::Point::Specific(self.mock_slot + 100, vec![0xFF; 32]),
            self.mock_slot + 100,
        )
    }

    fn handle_event(&mut self, event: ResponderEvent) {
        match event {
            ResponderEvent::PeerInitialized(pid, (version, _data)) => {
                tracing::info!(%pid, version, "peer initialized");
                self.initialized_peers.insert(pid);
            }

            ResponderEvent::PeerDisconnected(pid) => {
                tracing::info!(%pid, "peer disconnected");
                self.disconnected_peers.insert(pid);
            }

            ResponderEvent::IntersectionRequested(pid, _points) => {
                let point = proto::Point::Origin;
                let tip = self.mock_tip();
                self.network
                    .execute(ResponderCommand::ProvideIntersection(pid, point, tip));
            }

            ResponderEvent::NextHeaderRequested(pid) => {
                self.mock_slot += 1;

                let header = chainsync::HeaderContent {
                    variant: 6,
                    byron_prefix: None,
                    cbor: vec![0x00; 32],
                };
                let tip = self.mock_tip();
                self.network
                    .execute(ResponderCommand::ProvideHeader(pid, header, tip));
            }

            ResponderEvent::BlockRangeRequested(pid, (_start, _end)) => {
                let blocks: Vec<Vec<u8>> = (0..3).map(|i| vec![0xBE; 64 + i]).collect();
                self.network
                    .execute(ResponderCommand::ProvideBlocks(pid, blocks));
            }

            ResponderEvent::PeersRequested(pid, amount) => {
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

            ResponderEvent::TxReceived(_pid, _tx) => {}
        }
    }

    async fn tick(&mut self) {
        let event = self.network.poll_next().await;

        if let Some(event) = event {
            self.handle_event(event);
        }
    }
}

#[tokio::test]
async fn all_peers_complete_protocol_flow() {
    let num_peers = 3;
    let headers_per_peer = 5;
    let interface = MockInitiatorInterface::new(num_peers, headers_per_peer);
    let mut node = TestResponderNode::new(interface);

    // Run the tick loop with a generous timeout. The mock uses jittered sleeps
    // (50-1500ms per step), so the full protocol flow for 3 peers should
    // complete well within 30 seconds.
    let result = tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            node.tick().await;

            if node.disconnected_peers.len() == num_peers as usize {
                break;
            }
        }
    })
    .await;

    assert!(
        result.is_ok(),
        "test timed out waiting for protocol flow to complete"
    );

    assert_eq!(
        node.initialized_peers.len(),
        num_peers as usize,
        "expected all peers to initialize"
    );

    assert_eq!(
        node.disconnected_peers.len(),
        num_peers as usize,
        "expected all peers to disconnect after completing protocol flow"
    );
}
