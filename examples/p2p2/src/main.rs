use pallas_network::miniprotocols::Point;
use pallas_network2::behavior::InitiatorCommand;
use pallas_network2::PeerId;
use std::time::Duration;
use tokio::select;

use crate::emulator::MyEmulator;
use crate::node::MyNode;

mod emulator;
mod node;
mod otel;

#[tokio::main]
async fn main_emulator() {
    otel::setup_otel();

    let mut node = MyNode::new(MyEmulator::default());
    let (cmd_send, mut cmd_recv) = tokio::sync::mpsc::channel::<InitiatorCommand>(5);

    // constant injection of new peers
    let cmd_send_1 = cmd_send.clone();
    tokio::spawn(async move {
        for i in 0..50 {
            tracing::info!("including peer {}", 1234 + i);

            cmd_send_1
                .send(InitiatorCommand::IncludePeer(PeerId {
                    host: "127.0.0.1".to_string(),
                    port: 1234 + i,
                }))
                .await
                .unwrap();

            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });

    // constant requests of block ranges
    let cmd_send_2 = cmd_send.clone();
    tokio::spawn(async move {
        for i in 0..50 {
            tracing::info!("requesting block range {}", i);

            cmd_send_2
                .send(InitiatorCommand::RequestBlockBatch((
                    Point::Specific(i * 10, vec![0; 32]),
                    Point::Specific((i + 1) * 10, vec![0; 32]),
                )))
                .await
                .unwrap();

            tokio::time::sleep(Duration::from_secs(10)).await;
        }
    });

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

#[tokio::main]
async fn main() {
    otel::setup_otel();

    let interface = pallas_network2::interface::TokioInterface::new();

    let (cmd_send, mut cmd_recv) = tokio::sync::mpsc::channel::<InitiatorCommand>(5);

    let mut node = MyNode::new(interface);

    node.enqueue(InitiatorCommand::IncludePeer(PeerId {
        host: "backbone.mainnet.cardanofoundation.org".to_string(),
        port: 3001,
    }));

    node.enqueue(InitiatorCommand::IncludePeer(PeerId {
        host: "relay.cnode-m1.demeter.run".to_string(),
        port: 3000,
    }));

    // node.enqueue(InitiatorCommand::IncludePeer(PeerId {
    //     host: "251.70.119.168".to_string(),
    //     port: 3001,
    // }));

    node.enqueue(InitiatorCommand::IncludePeer(PeerId {
        host: "r1.1percentpool.eu".to_string(),
        port: 19001,
    }));

    // constant requests of block ranges
    let cmd_send_2 = cmd_send.clone();
    tokio::spawn(async move {
        for i in 0..3 {
            tracing::info!("requesting block range {}", i);

            let point = Point::Specific(
                164256430,
                hex::decode("78709859196d89627faff31cf041449277258a6f78b1fe64cbf42e8448a1f219")
                    .unwrap(),
            );

            cmd_send_2
                .send(InitiatorCommand::RequestBlockBatch((point.clone(), point)))
                .await
                .unwrap();

            tokio::time::sleep(Duration::from_secs(10)).await;
        }
    });

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
