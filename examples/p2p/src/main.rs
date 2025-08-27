use pallas_network2::protocol::Point;

#[allow(unused_imports)]
use crate::emulator::MyEmulator;

use crate::node::{MyConfig, MyNode, PromotionConfig};

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
                "relay.cnode-m1.demeter.run:3000".to_string(),
                "r1.1percentpool.eu:19001".to_string(),
                "backbone.mainnet.cardanofoundation.org:3001".to_string(),
                "backbone.cardano.iog.io:3001".to_string(),
            ],
            promotion: PromotionConfig {
                max_peers: 10,
                max_warm_peers: 5,
                max_hot_peers: 3,
                max_error_count: 10,
            },
        },
        interface,
    );

    node.run_forever().await.unwrap();
}
