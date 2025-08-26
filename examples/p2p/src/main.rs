use pallas_network2::behavior::InitiatorCommand;
use pallas_network2::protocol::Point;
use pallas_network2::PeerId;
use std::time::Duration;
use tokio::select;

#[allow(unused_imports)]
use crate::emulator::MyEmulator;

use crate::node::{MyConfig, MyNode};

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

    let mut node = MyNode::new(
        MyConfig {
            chain_intersection: vec![Point::Origin],
            initial_peers: vec![
                PeerId {
                    host: "backbone.mainnet.cardanofoundation.org".to_string(),
                    port: 3001,
                },
                PeerId {
                    host: "relay.cnode-m1.demeter.run".to_string(),
                    port: 3000,
                },
                PeerId {
                    host: "r1.1percentpool.eu".to_string(),
                    port: 19001,
                },
                PeerId {
                    host: "backbone.cardano.iog.io".to_string(),
                    port: 3001,
                },
            ],
        },
        interface,
    );

    //let (cmd_send, cmd_recv) = tokio::sync::mpsc::channel(100);
    let chain = node.download_chain(10).await;

    println!("blocks downloaded: {:?}", chain.len());
}
