use pallas_network2::behavior::InitiatorCommand;
use pallas_network2::protocol::Point;
use pallas_network2::PeerId;
use std::time::Duration;
use tokio::select;

#[allow(unused_imports)]
use crate::emulator::MyEmulator;

use crate::node::MyNode;

mod emulator;
mod node;
mod otel;

#[tokio::main]
async fn main() {
    otel::setup_otel();

    // if you want to use the emulator instead of the real interface use MyEmulator
    // instead of the TcpInterface

    // let interface = MyEmulator::default();

    let interface = pallas_network2::interface::TcpInterface::new();

    let mut node = MyNode::new(interface);

    let (cmd_send, mut cmd_recv) = tokio::sync::mpsc::channel::<InitiatorCommand>(5);

    node.enqueue(InitiatorCommand::IncludePeer(PeerId {
        host: "backbone.mainnet.cardanofoundation.org".to_string(),
        port: 3001,
    }));

    node.enqueue(InitiatorCommand::IncludePeer(PeerId {
        host: "relay.cnode-m1.demeter.run".to_string(),
        port: 3000,
    }));

    node.enqueue(InitiatorCommand::IncludePeer(PeerId {
        host: "r1.1percentpool.eu".to_string(),
        port: 19001,
    }));

    node.enqueue(InitiatorCommand::IncludePeer(PeerId {
        host: "backbone.cardano.iog.io".to_string(),
        port: 3001,
    }));

    node.enqueue(InitiatorCommand::IncludePeer(PeerId {
        host: "backbone.mainnet.emurgornd.com".to_string(),
        port: 3001,
    }));

    node.enqueue(InitiatorCommand::IntersectChain(vec![Point::Origin]));

    // constant requests of block ranges
    // let cmd_send_2 = cmd_send.clone();
    // tokio::spawn(async move {
    //     for i in 0..30 {
    //         tracing::info!("requesting block range {}", i);

    //         let point = Point::Specific(
    //             164256430,
    //
    // hex::decode("
    // 78709859196d89627faff31cf041449277258a6f78b1fe64cbf42e8448a1f219")
    //                 .unwrap(),
    //         );

    //         cmd_send_2
    //             .send(InitiatorCommand::RequestBlockBatch((point.clone(),
    // point)))             .await
    //             .unwrap();

    //         tokio::time::sleep(Duration::from_secs(1)).await;
    //     }
    // });

    loop {
        select! {
            cmd = cmd_recv.recv() => {
                if let Some(cmd) = cmd {
                    node.enqueue(cmd);
                }
            }
            _ = node.tick() => {
                tokio::task::yield_now().await;
            }
        }
    }
}
