use std::time::Duration;

use pallas_network2::behavior::{AnyMessage, InitiatorBehavior, InitiatorCommand, InitiatorEvent};
use pallas_network2::{Manager, PeerId, emulation};

#[derive(Default)]
struct MyEmulatorRules;

impl emulation::Rules for MyEmulatorRules {
    type Message = AnyMessage;

    fn reply_to(
        &self,
        pid: PeerId,
        msg: Self::Message,
        jitter: Duration,
        queue: &mut emulation::ReplyQueue<Self::Message>,
    ) {
        match msg {
            AnyMessage::Handshake(msg) => match msg {
                pallas_network2::protocol::handshake::Message::Propose(version_table) => {
                    let (version, data) = version_table.values.into_iter().next().unwrap();

                    let msg = pallas_network2::protocol::handshake::Message::Accept(version, data);

                    queue.push_jittered_msg(pid, AnyMessage::Handshake(msg), jitter);
                }
                _ => queue.push_jittered_disconnect(pid, jitter),
            },
            AnyMessage::KeepAlive(msg) => {
                let pallas_network2::protocol::keepalive::Message::KeepAlive(token) = msg else {
                    queue.push_jittered_disconnect(pid, jitter);
                    return;
                };

                let msg = pallas_network2::protocol::keepalive::Message::ResponseKeepAlive(token);

                queue.push_jittered_msg(pid, AnyMessage::KeepAlive(msg), jitter);
            }
            _ => todo!(),
        };
    }
}

type MyEmulator = emulation::Emulator<AnyMessage, MyEmulatorRules>;

struct MyNode {
    network: Manager<MyEmulator, InitiatorBehavior, AnyMessage>,
}

impl MyNode {
    async fn tick(&mut self) {
        let event = self.network.poll_next().await;

        let Some(event) = event else {
            return;
        };

        let next_cmd = match event {
            InitiatorEvent::PeerInitialized(peer_id, version) => {
                tracing::info!(%peer_id, ?version, "peer initialized");
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
            self.network.enqueue(cmd);
        }
    }
}

#[tokio::test]
async fn test_network() {
    let mut node = MyNode {
        network: Manager::new(MyEmulator::default(), InitiatorBehavior::default()),
    };

    [1234, 1235, 1236, 1237, 1238]
        .into_iter()
        .map(|port| PeerId {
            host: "127.0.0.1".to_string(),
            port,
        })
        .for_each(|x| node.network.enqueue(InitiatorCommand::IncludePeer(x)));

    node.network.enqueue(InitiatorCommand::StartSync(vec![
        pallas_network2::protocol::Point::Origin,
    ]));

    for _ in 0..20 {
        node.tick().await;
    }
}
