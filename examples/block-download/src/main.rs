use pallas::network::{
    facades::PeerClient,
    miniprotocols::{Point, MAINNET_MAGIC},
};

#[tokio::main]
async fn main() {
    tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(tracing::Level::TRACE)
            .finish(),
    )
    .unwrap();

    let mut peer = PeerClient::connect("relays-new.cardano-mainnet.iohk.io:3001", MAINNET_MAGIC)
        .await
        .unwrap();

    let point = Point::Specific(
        101516417,
        hex::decode("3d681e503fd9318d0f68c74a699895ce61f0a07010b516b80ce968a6b000e231").unwrap(),
    );

    let block = peer
        .blockfetch()
        .fetch_single(point)
        .await
        .unwrap()
        .unwrap();

    println!("downloaded block of size: {}", block.len());
    println!("{}", hex::encode(&block));
}
