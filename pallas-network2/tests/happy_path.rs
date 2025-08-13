use pallas_network::miniprotocols::{Point, keepalive, txsubmission};
use pallas_network2::standard::{AnyMessage, InitiatorBehavior, InitiatorCommand, InitiatorEvent};
use pallas_network2::{Manager, PeerId, emulation};

#[derive(Default)]
struct MyEmulatorRules;

impl emulation::Rules for MyEmulatorRules {
    type Message = AnyMessage;

    fn reply_to(&self, msg: Self::Message) -> emulation::ReplyAction<Self::Message> {
        let reply = match msg {
            AnyMessage::Handshake(msg) => match msg {
                pallas_network::miniprotocols::handshake::Message::Propose(version_table) => {
                    let (version, data) = version_table.values.into_iter().next().unwrap();

                    let msg =
                        pallas_network::miniprotocols::handshake::Message::Accept(version, data);

                    emulation::ReplyAction::Message(AnyMessage::Handshake(msg))
                }
                _ => emulation::ReplyAction::Disconnect,
            },
            AnyMessage::KeepAlive(msg) => {
                let keepalive::Message::KeepAlive(token) = msg else {
                    return emulation::ReplyAction::Disconnect;
                };

                let msg = keepalive::Message::ResponseKeepAlive(token);

                emulation::ReplyAction::Message(AnyMessage::KeepAlive(msg))
            }
            _ => todo!(),
        };

        reply
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
                println!("Peer initialized: {peer_id} with version {version:?}");
                Some(InitiatorCommand::IntersectChain(peer_id, Point::Origin))
            }

            InitiatorEvent::BlockHeaderReceived(peer_id, _) => {
                println!("Block header received from {peer_id}");
                None
            }
            InitiatorEvent::BlockBodyReceived(peer_id, _, _) => {
                println!("Block body received from {peer_id}");
                None
            }
            InitiatorEvent::TxRequested(peer_id, _) => {
                println!("Tx requested from {peer_id}");
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

    for _ in 0..20 {
        node.tick().await;
    }
}
