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
        49159253,
        hex::decode("d034a2d0e4c3076f57368ed59319010c265718f0923057f8ff914a3b6bfd1314").unwrap(),
    );

    let block = peer.blockfetch().fetch_single(point).await.unwrap();

    println!("downloaded block of size: {}", block.len());
    println!("{}", hex::encode(&block));
}
