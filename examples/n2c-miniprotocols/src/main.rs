use pallas::{
    codec::utils::Bytes,
    ledger::{addresses::Address, traverse::MultiEraBlock},
    network::{
        facades::NodeClient,
        miniprotocols::{
            chainsync,
            localstate::queries_v16::{self, Addr, Addrs},
            Point, PRE_PRODUCTION_MAGIC,
        },
    },
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

    let result = queries_v16::get_stake_distribution(client, era)
        .await
        .unwrap();
    info!("result: {:?}", result);

    let addrx = "addr_test1vr80076l3x5uw6n94nwhgmv7ssgy6muzf47ugn6z0l92rhg2mgtu0".to_string();
    let addrx: Address = Address::from_bech32(&addrx).unwrap();
    let addrx: Addr = addrx.to_vec().into();

    let addry =
    "008c5bf0f2af6f1ef08bb3f6ec702dd16e1c514b7e1d12f7549b47db9f4d943c7af0aaec774757d4745d1a2c8dd3220e6ec2c9df23f757a2f8"
    .to_string();
    let addry: Address = Address::from_hex(&addry).unwrap();
    let addry: Addr = addry.to_vec().into();

    let addrs: Addrs = vec![addrx, addry];
    let result = queries_v16::get_utxo_by_address(client, era, addrs)
        .await
        .unwrap();
    info!("result: {:?}", result);

    let result = queries_v16::get_current_pparams(client, era).await.unwrap();
    println!("result: {:?}", result);

    // Stake pool ID/verification key hash (either Bech32-decoded or hex-decoded).
    // Empty list means all pools.
    let pools = vec![];
    let result = queries_v16::get_stake_snapshots(client, era, pools)
        .await
        .unwrap();
    println!("result: {:?}", result);

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

    loop {
        let next = client.chainsync().request_next().await.unwrap();
        match next {
            chainsync::NextResponse::RollForward(h, _) => {
                let block_number = MultiEraBlock::decode(&h).unwrap().number();
                info!("rolling forward {}, block size: {}", block_number, h.len())
            }
            chainsync::NextResponse::RollBackward(x, _) => info!("rollback to {:?}", x),
            chainsync::NextResponse::Await => info!("tip of chain reached"),
        };
    }
}

// change the following to match the Cardano node socket in your local
// environment
#[cfg(unix)]
const SOCKET_PATH: &str = "/tmp/node.socket";

#[cfg(unix)]
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

// change the following to match the Cardano node named-pipe in your local
// environment
#[cfg(target_family = "windows")]
const PIPE_NAME: &str = "\\\\.\\pipe\\cardano-pallas";

#[cfg(target_family = "windows")]
#[tokio::main]
async fn main() {
    tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(tracing::Level::TRACE)
            .finish(),
    )
    .unwrap();

    // we connect to the named-pipe of the local node. Make sure you have the right
    // path for your environment
    let mut client = NodeClient::connect(PIPE_NAME, PRE_PRODUCTION_MAGIC)
        .await
        .unwrap();

    // execute an arbitrary "Local State" query against the node
    do_localstate_query(&mut client).await;

    // execute the chainsync flow from an arbitrary point in the chain
    do_chainsync(&mut client).await;
}
