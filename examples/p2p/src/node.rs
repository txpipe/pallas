use pallas_network2::{
    behavior::{AnyMessage, InitiatorBehavior, InitiatorCommand, InitiatorEvent},
    protocol::{txsubmission, Point},
    Interface, Manager,
};

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
            InitiatorEvent::PeerInitialized(pid, _) => {
                tracing::info!(%pid, "peer initialized");
                None
            }
            InitiatorEvent::BlockHeaderReceived(pid, x, _) => {
                let tag = x.variant;
                let subtag = x.byron_prefix.map(|(x, _)| x);
                let cbor = &x.cbor;

                let header = pallas_traverse::MultiEraHeader::decode(tag, subtag, cbor).unwrap();

                tracing::info!(slot = header.slot(), %pid, "header received");
                None
            }
            InitiatorEvent::RollbackReceived(pid, _, _) => {
                tracing::info!(%pid, "rollback received");
                None
            }
            InitiatorEvent::BlockBodyReceived(pid, _) => {
                tracing::info!(%pid, "block body received");
                None
            }
            InitiatorEvent::TxRequested(pid, _) => {
                tracing::info!(%pid, "tx requested");
                Some(InitiatorCommand::SendTx(
                    pid,
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
