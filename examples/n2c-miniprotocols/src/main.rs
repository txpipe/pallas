use pallas::network::{
    facades::NodeClient,
    miniprotocols::{chainsync, localstate::queries_v16, Point, PRE_PRODUCTION_MAGIC},
};
use tracing::info;

async fn do_localstate_query(client: &mut NodeClient) {
    let client = client.statequery();

    client.acquire(None).await.unwrap();

    let result = queries_v16::get_chain_point(client).await.unwrap();
    info!("result: {:?}", result);

    let result = queries_v16::get_system_start(client).await.unwrap();
    info!("result: {:?}", result);

    let era = queries_v16::get_current_era(client).await.unwrap();
    info!("result: {:?}", era);

    let result = queries_v16::get_block_epoch_number(client, era)
        .await
        .unwrap();

    info!("result: {:?}", result);

    client.send_release().await.unwrap();
}

async fn do_chainsync(client: &mut NodeClient) {
    let known_points = vec![Point::Specific(
        43847831u64,
        hex::decode("15b9eeee849dd6386d3770b0745e0450190f7560e5159b1b3ab13b14b2684a45").unwrap(),
    )];

    let (point, _) = client
        .chainsync()
        .find_intersect(known_points)
        .await
        .unwrap();

    info!("intersected point is {:?}", point);

    for _ in 0..10 {
        let next = client.chainsync().request_next().await.unwrap();

        match next {
            chainsync::NextResponse::RollForward(h, _) => {
                log::info!("rolling forward, block size: {}", h.len())
            }
            chainsync::NextResponse::RollBackward(x, _) => log::info!("rollback to {:?}", x),
            chainsync::NextResponse::Await => log::info!("tip of chain reached"),
        };
    }
}

// change the following to match the Cardano node socket in your local
// environment
const SOCKET_PATH: &str = "/tmp/node.socket";

#[cfg(target_family = "unix")]
#[tokio::main]
async fn main() {
    tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(tracing::Level::TRACE)
            .finish(),
    )
    .unwrap();

    // we connect to the unix socket of the local node. Make sure you have the right
    // path for your environment
    let mut client = NodeClient::connect(SOCKET_PATH, PRE_PRODUCTION_MAGIC)
        .await
        .unwrap();

    // execute an arbitrary "Local State" query against the node
    do_localstate_query(&mut client).await;

    // execute the chainsync flow from an arbitrary point in the chain
    do_chainsync(&mut client).await;
}

#[cfg(not(target_family = "unix"))]
fn main() {
    panic!("can't use n2c unix socket on non-unix systems");
}
