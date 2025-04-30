use std::{str::FromStr, time::Duration};

use pallas::{
    ledger::traverse::wellknown::PREVIEW_MAGIC,
    network::{
        manager::{
            behaviors::{
                ConnectPeersBehavior, ConnectPeersConfig, HandshakeBehavior, HandshakeConfig,
                InterleaveBehavior, KeepAliveBehavior, KeepAliveConfig, PeerDiscoveryBehavior,
                PeerDiscoveryConfig, PeerPromotionBehavior, PeerPromotionConfig,
            },
            IntrinsicCommand, Manager, PeerId,
        },
        miniprotocols::MAINNET_MAGIC,
    },
};
use tokio::runtime::Handle;
use tokio_stream::{self, StreamExt as TokioStreamExt};

#[tokio::main]
async fn main() {
    tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(tracing::Level::INFO)
            .finish(),
    )
    .unwrap();

    let behavior = PeerPromotionBehavior::new(PeerPromotionConfig {
        desired_hot_peers: (10, 10),
        desired_warm_peers: (10, 10),
        desired_cold_peers: (10, 10),
        trusted_peers: vec![
            PeerId::from_str("backbone.mainnet.cardanofoundation.org:3001").unwrap(),
            //PeerId::from_str("backbone.cardano.iog.io:3001").unwrap(),
            PeerId::from_str("relay.cnode-m1.demeter.run:3000").unwrap(),
            PeerId::from_str("r1.1percentpool.eu:19001").unwrap(),
            //PeerId::from_str("105.251.38.46:6000").unwrap(),
            //PeerId::from_str("preview.adastack.net:3001").unwrap(),
            //PeerId::from_str("preview-node.play.dev.cardano.org:3001").unwrap(),
            //PeerId::from_str("relay.cnode-m1.demeter.run:3002").unwrap(),
            //PeerId::from_str("84.67.210.50:3001").unwrap(),
        ],
    });

    let next = ConnectPeersBehavior::new(ConnectPeersConfig {});

    let behavior = InterleaveBehavior::new(behavior, next);

    let version_data = pallas::network::miniprotocols::handshake::n2n::VersionData {
        network_magic: MAINNET_MAGIC,
        initiator_only_diffusion_mode: false,
        peer_sharing: Some(1),
        query: Some(false),
    };

    let next = HandshakeBehavior::new(HandshakeConfig {
        handshake: pallas::network::miniprotocols::handshake::n2n::VersionTable {
            values: vec![(13, version_data)].into_iter().collect(),
        },
    });

    let behavior = InterleaveBehavior::new(behavior, next);

    let next = KeepAliveBehavior::new(KeepAliveConfig {
        interval: Duration::from_secs(5),
    });

    let behavior = InterleaveBehavior::new(behavior, next);

    let next = PeerDiscoveryBehavior::new(PeerDiscoveryConfig { desired_peers: 10 });

    let behavior = InterleaveBehavior::new(behavior, next);

    Manager::new(behavior).run().await;

    // let infinite_sprints = futures::stream::repeat_with(|| manager.sprint())
    //     .throttle(std::time::Duration::from_millis(5000));

    // infinite_sprints.for_each(|f| f).await;
}
