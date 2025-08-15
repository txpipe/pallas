use std::net::Ipv4Addr;

use pallas_network::miniprotocols::{keepalive, peersharing, txsubmission, Point};
use pallas_network2::standard::{AnyMessage, InitiatorBehavior, InitiatorCommand, InitiatorEvent};
use pallas_network2::{emulation, Manager, PeerId};

#[derive(Default)]
struct MyEmulatorRules;

impl emulation::Rules for MyEmulatorRules {
    type Message = AnyMessage;

    fn reply_to(&self, msg: Self::Message) -> emulation::ReplyAction<Self::Message> {
        let reply = match msg {
            AnyMessage::Handshake(msg) => match msg {
                pallas_network::miniprotocols::handshake::Message::Propose(version_table) => {
                    let (version, mut data) = version_table.values.into_iter().next().unwrap();

                    data.peer_sharing = Some(1);

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

                println!("emulation: received keepalive: {token}");

                let msg = keepalive::Message::ResponseKeepAlive(token);

                emulation::ReplyAction::Message(AnyMessage::KeepAlive(msg))
            }
            AnyMessage::PeerSharing(msg) => {
                let peersharing::Message::ShareRequest(amount) = msg else {
                    return emulation::ReplyAction::Disconnect;
                };

                println!("emulation: received peer sharing request: {amount}");

                let msg = peersharing::Message::SharePeers(vec![peersharing::PeerAddress::V4(
                    Ipv4Addr::new(123, 123, 123, 123),
                    9999,
                )]);

                emulation::ReplyAction::Message(AnyMessage::PeerSharing(msg))
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
            InitiatorEvent::PeerInitialized(peer_id, _) => {
                println!("Peer initialized: {peer_id}");
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

use opentelemetry_sdk::metrics::{MeterProviderBuilder, PeriodicReader};

#[tokio::main]
async fn main() {
    let exporter = opentelemetry_stdout::MetricExporter::default();

    let provider = MeterProviderBuilder::default()
        .with_periodic_exporter(exporter)
        .build();

    opentelemetry::global::set_meter_provider(provider);

    let mut node = MyNode {
        network: Manager::new(MyEmulator::default(), InitiatorBehavior::default()),
    };

    [
        1234, 1235, 1236, 1237, 1238, 1239, 1240, 1241, 1242, 1243, 1244, 1245, 1246, 1247, 1248,
        1249,
    ]
    .into_iter()
    .map(|port| PeerId {
        host: "127.0.0.1".to_string(),
        port,
    })
    .for_each(|x| node.network.enqueue(InitiatorCommand::IncludePeer(x)));

    loop {
        node.tick().await;
    }
}
