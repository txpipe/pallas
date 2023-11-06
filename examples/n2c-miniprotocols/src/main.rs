use pallas::network::{
    facades::NodeClient,
    miniprotocols::{
        chainsync,
        localstate::{self, queries::Request},
        Point, MAINNET_MAGIC, PRE_PRODUCTION_MAGIC,
    },
};
use tracing::info;

async fn do_localstate_query(client: &mut NodeClient, query: Request) {
    do_localstate_query_acquisition(client).await;

    let result = client.statequery().query(query).await.unwrap();
    info!("result: {:?}", result);

    client.statequery().send_release().await.unwrap();
}

async fn do_localstate_query_acquisition(client: &mut NodeClient) {
    if let localstate::State::Idle = client.statequery().state() {
        client.statequery().acquire(None).await.unwrap();
    }
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

async fn setup_client() -> NodeClient {
    // we connect to the unix socket of the local node. Make sure you have the right
    // path for your environment
    let socket_path = "/Users/falcucci/Downloads/cardano-node-8.1.2-macos/node.socket";

    // we connect to the unix socket of the local node and perform a handshake query
    let version_table = NodeClient::handshake_query(socket_path, PRE_PRODUCTION_MAGIC)
        .await
        .unwrap();

    info!("handshake query result: {:?}", version_table);

    NodeClient::connect(socket_path, PRE_PRODUCTION_MAGIC)
        .await
        .unwrap()
}

#[cfg(target_family = "unix")]
#[tokio::main]
async fn main() {
    tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(tracing::Level::TRACE)
            .finish(),
    )
    .unwrap();

    let mut client = setup_client().await;

    // specify the query we want to execute
    let get_system_start_query = localstate::queries::Request::GetSystemStart;
    let get_epoch_query =
        localstate::queries::Request::BlockQuery(localstate::queries::BlockQuery::GetEpochNo);

    // execute an arbitrary "Local State" query against the node
    do_localstate_query(&mut client, get_system_start_query).await;
    do_localstate_query(&mut client, get_epoch_query).await;

    client.statequery().send_done().await.unwrap();

    // execute the chainsync flow from an arbitrary point in the chain
    // do_chainsync(&mut client).await;
}

#[cfg(not(target_family = "unix"))]

fn main() {
    panic!("can't use n2c unix socket on non-unix systems");
}
