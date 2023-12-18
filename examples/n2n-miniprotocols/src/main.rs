use pallas::network::{
    facades::PeerClient,
    miniprotocols::{chainsync, Point, MAINNET_MAGIC},
};
use tracing::info;

async fn do_blockfetch(peer: &mut PeerClient) {
    let range = (
        Point::Specific(
            43847831,
            hex::decode("15b9eeee849dd6386d3770b0745e0450190f7560e5159b1b3ab13b14b2684a45")
                .unwrap(),
        ),
        Point::Specific(
            43847844,
            hex::decode("ff8d558a3d5a0e058beb3d94d26a567f75cd7d09ff5485aa0d0ebc38b61378d4")
                .unwrap(),
        ),
    );

    let blocks = peer.blockfetch().fetch_range(range).await.unwrap();

    for block in blocks {
        info!("received block of size: {}", block.len());
    }
}

async fn do_chainsync(peer: &mut PeerClient) {
    let known_points = vec![Point::Specific(
        43847831u64,
        hex::decode("15b9eeee849dd6386d3770b0745e0450190f7560e5159b1b3ab13b14b2684a45").unwrap(),
    )];

    let (point, _) = peer.chainsync().find_intersect(known_points).await.unwrap();

    info!("intersected point is {:?}", point);

    for _ in 0..100 {
        let next = peer.chainsync().request_next().await.unwrap();

        match next {
            chainsync::NextResponse::RollForward(h, _) => {
                log::info!("rolling forward, header size: {}", h.cbor.len())
            }
            chainsync::NextResponse::RollBackward(x, _) => log::info!("rollback to {:?}", x),
            chainsync::NextResponse::Await => log::info!("tip of chaing reached"),
        };
    }
}

#[tokio::main]
async fn main() {
    tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(tracing::Level::TRACE)
            .finish(),
    )
    .unwrap();

    // setup a TCP socket to act as data bearer between our agents and the remote
    // relay.
    let mut peer = PeerClient::connect("localhost:3000", MAINNET_MAGIC)
        .await
        .unwrap();

    // fetch an arbitrary batch of block
    //do_blockfetch(&mut peer).await;

    // execute the chainsync flow from an arbitrary point in the chain
    do_chainsync(&mut peer).await;
}
