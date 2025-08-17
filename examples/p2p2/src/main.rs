use pallas_network::miniprotocols::Point;
use pallas_network2::standard::InitiatorCommand;
use pallas_network2::PeerId;
use std::time::Duration;
use tokio::select;

use crate::node::MyNode;

mod emulator;
mod node;
mod otel;

#[tokio::main]
async fn main() {
    otel::setup_otel();

    let mut node = MyNode::default();
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
                .send(InitiatorCommand::RequestBlockBatch(
                    (
                        Point::Specific(i * 10, vec![0; 32]),
                        Point::Specific((i + 1) * 10, vec![0; 32]),
                    ),
                    None,
                ))
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
