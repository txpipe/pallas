use std::any::Any;

use pallas_network::miniprotocols::{txsubmission, Point};
use pallas_network2::{
    behavior::{AnyMessage, InitiatorBehavior, InitiatorCommand, InitiatorEvent},
    Interface, Manager,
};

use crate::emulator::MyEmulator;

pub struct MyNode<I: Interface<AnyMessage>> {
    network: Manager<I, InitiatorBehavior, AnyMessage>,
}

impl<I: Interface<AnyMessage>> MyNode<I> {
    pub async fn tick(&mut self) {
        let event = self.network.poll_next().await;

        let Some(event) = event else {
            return;
        };

        let next_cmd = match event {
            InitiatorEvent::PeerInitialized(peer_id, _) => {
                tracing::info!(%peer_id, "peer initialized");
                Some(InitiatorCommand::IntersectChain(peer_id, Point::Origin))
            }

            InitiatorEvent::BlockHeaderReceived(peer_id, _) => {
                tracing::debug!(%peer_id, "block header received");
                None
            }
            InitiatorEvent::BlockBodyReceived(peer_id, _) => {
                tracing::warn!(%peer_id, "block body received");
                None
            }
            InitiatorEvent::TxRequested(peer_id, _) => {
                tracing::info!(%peer_id, "tx requested");
                Some(InitiatorCommand::SendTx(
                    peer_id,
                    txsubmission::EraTxId(0, vec![]),
                    txsubmission::EraTxBody(0, vec![]),
                ))
            }
        };

        if let Some(cmd) = next_cmd {
            self.network.enqueue(cmd);
        }
    }

    pub fn enqueue(&mut self, cmd: InitiatorCommand) {
        self.network.enqueue(cmd);
    }
}

impl<I: Interface<AnyMessage>> MyNode<I> {
    pub fn new(interface: I) -> Self {
        Self {
            network: Manager::new(interface, InitiatorBehavior::default()),
        }
    }
}
