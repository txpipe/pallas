use std::collections::HashSet;

use pallas_network2::emulation::happy::HappyEmulator;
use pallas_network2::initiator::{AnyMessage, InitiatorBehavior, InitiatorCommand, InitiatorEvent};
use pallas_network2::{Manager, PeerId};

struct MockNode {
    network: Manager<HappyEmulator, InitiatorBehavior, AnyMessage>,
    initialized_peers: HashSet<PeerId>,
}

impl MockNode {
    async fn tick(&mut self) {
        let event = self.network.poll_next().await;

        let Some(event) = event else {
            return;
        };

        let next_cmd = match event {
            InitiatorEvent::PeerInitialized(peer_id, version) => {
                tracing::info!(%peer_id, ?version, "peer initialized");
                self.initialized_peers.insert(peer_id);
                None
            }
            InitiatorEvent::IntersectionFound(pid, _, _) => {
                tracing::info!(%pid, "intersection found");
                None
            }
            InitiatorEvent::BlockHeaderReceived(pid, _, _) => {
                tracing::debug!(%pid, "block header received");
                None
            }
            InitiatorEvent::RollbackReceived(pid, _, _) => {
                tracing::debug!(%pid, "rollback received");
                None
            }
            InitiatorEvent::BlockBodyReceived(pid, _) => {
                tracing::debug!(%pid, "block body received");
                None
            }
            InitiatorEvent::TxRequested(pid, _) => {
                tracing::info!(%pid, "tx requested");
                Some(InitiatorCommand::SendTx(
                    pid,
                    pallas_network2::protocol::txsubmission::EraTxId(0, vec![]),
                    pallas_network2::protocol::txsubmission::EraTxBody(0, vec![]),
                ))
            }
        };

        if let Some(cmd) = next_cmd {
            self.network.execute(cmd);
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_include_peer() {
    let mut node = MockNode {
        initialized_peers: HashSet::new(),
        network: Manager::new(HappyEmulator::default(), InitiatorBehavior::default()),
    };

    node.network.execute(InitiatorCommand::IncludePeer(PeerId {
        host: "99.99.99.99".to_string(),
        port: 1234,
    }));

    for i in 0..100 {
        node.tick().await;

        if i % 5 == 0 {
            node.network.execute(InitiatorCommand::Housekeeping);
        }
    }

    assert_eq!(node.initialized_peers.len(), 1);
}
