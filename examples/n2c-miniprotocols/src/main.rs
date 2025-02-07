use std::{collections::BTreeSet, str::FromStr};

use pallas::{
    codec::utils::Bytes,
    crypto::hash::Hash,
    ledger::{addresses::Address, traverse::MultiEraBlock},
    network::{
        facades::NodeClient,
        miniprotocols::{
            chainsync,
            localstate::queries_v16::{
                self, Addr, Addrs, Credential, DRep, StakeAddr, TransactionInput,
            },
            localtxsubmission::SMaybe,
            Point, PRE_PRODUCTION_MAGIC,
        },
    },
};
use tracing::info;

async fn do_localstate_query(client: &mut NodeClient) {
    let client = client.statequery();

    client.acquire(None).await.unwrap();

    // Get UTxO from a (singleton) set of tx inputs.
    let transaction_id =
        Hash::<32>::from_str("15244950ed56a3af61a00f62584779fb53a9f3910468013a2b00b94b8bbc10e0")
            .unwrap();
    let tx_in = TransactionInput {
        transaction_id,
        index: 0,
    };
    let mut txins = BTreeSet::new();
    txins.insert(tx_in);

    let result = queries_v16::get_utxo_by_txin(client, 6, txins)
        .await
        .unwrap();
    info!("result: {:?}", result);

    let result = queries_v16::get_chain_point(client).await.unwrap();
    info!("result: {:?}", result);

    let result = queries_v16::get_system_start(client).await.unwrap();
    info!("result: {:?}", result);

    let result = queries_v16::get_chain_block_no(client).await.unwrap();
    info!("result: {:?}", result);

    let era = queries_v16::get_current_era(client).await.unwrap();
    info!("result: {:?}", era);

    let result = queries_v16::get_proposed_pparams_updates(client, era)
        .await
        .unwrap();
    info!("result: {:02x?}", result);

    let result = queries_v16::get_ratify_state(client, era).await.unwrap();
    info!("result: {:02x?}", result);

    let result = queries_v16::get_account_state(client, era).await.unwrap();
    info!("result: {:02x?}", result);

    // Not yet available in Demeter nodes to test.
    // let result = queries_v16::get_big_ledger_snapshot(client, era)
    //     .await
    //     .unwrap();
    // info!("result: {:02x?}", result);

    let gov_ids: BTreeSet<_> = [].into();
    let result = queries_v16::get_proposals(client, era, gov_ids.into())
        .await
        .unwrap();
    info!("result: {:02x?}", result);

    let result = queries_v16::get_future_protocol_params(client, era)
        .await
        .unwrap();
    info!("result: {:02x?}", result);

    // This one is large (~120MB in preprod).
    // let result = queries_v16::get_gov_state(client, era).await.unwrap();
    // info!("result: {:02x?}", result);

    // CC Member cc_cold1zwn2tcqxl48g75gx9hzrzd3rdxe2gv2q408d32307gjk67cp3tktt
    let cold_bytes =
        Bytes::from_str("a6a5e006fd4e8f51062dc431362369b2a43140abced8aa2ff2256d7b").unwrap();
    let colds: BTreeSet<_> = [Credential::from((0x01, cold_bytes))].into();
    let hots: BTreeSet<_> = [].into();
    let status: BTreeSet<_> = [].into();

    let result = queries_v16::get_committee_members_state(
        client,
        era,
        colds.into(),
        hots.into(),
        status.into(),
    )
    .await
    .unwrap();
    println!("result: {:?}", result);

    let result = queries_v16::get_constitution(client, era).await.unwrap();
    info!("result: {:02x?}", result);

    // Getting delegation and rewards for preprod stake addresses:
    let mut addrs = BTreeSet::new();
    // 1. `stake_test1uqfp3atrunssjk8a4w7lk3ct97wnscs4wc7v3ynnmx7ll7s2ea9p2`
    let addr: Addr = hex::decode("1218F563E4E10958FDABBDFB470B2F9D386215763CC89273D9BDFFFA")
        .unwrap()
        .into();
    addrs.insert(StakeAddr::from((0x00, addr)));
    // 2. `stake_test1upvpjglz8cz97wc26qf2glnz3q7egxq58xxul28ma2yhwkqhfylwh`
    let addr: Addr = hex::decode("581923e23e045f3b0ad012a47e62883d941814398dcfa8fbea897758")
        .unwrap()
        .into();
    addrs.insert(StakeAddr::from((0x00, addr)));

    let result = queries_v16::get_filtered_delegations_rewards(client, era, addrs.clone())
        .await
        .unwrap();
    info!("result: {:?}", result);

    let result = queries_v16::get_filtered_vote_delegatees(client, era, addrs.into())
        .await
        .unwrap();
    info!("result: {:02x?}", result);

    let pool_id1 = "fdb5834ba06eb4baafd50550d2dc9b3742d2c52cc5ee65bf8673823b";
    let pool_id1 = Bytes::from_str(pool_id1).unwrap();
    let pool_id2 = "1e3105f23f2ac91b3fb4c35fa4fe301421028e356e114944e902005b";
    let pool_id2 = Bytes::from_str(pool_id2).unwrap();
    let pools: BTreeSet<_> = [pool_id1, pool_id2].into();

    let result = queries_v16::get_stake_pool_params(client, era, pools.clone().into())
        .await
        .unwrap();
    info!("result: {:?}", result);

    let result = queries_v16::get_pool_state(client, era, SMaybe::Some(pools.clone().into()))
        .await
        .unwrap();
    info!("result: {:?}", result);

    let result = queries_v16::get_pool_distr(client, era, SMaybe::Some(pools.clone().into()))
        .await
        .unwrap();
    info!("result: {:02x?}", result);

    let result = queries_v16::get_spo_stake_distr(client, era, pools.into())
        .await
        .unwrap();
    info!("result: {:02x?}", result);

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
    // Empty Set means all pools.
    let pools: BTreeSet<Bytes> = BTreeSet::new();
    let result = queries_v16::get_stake_snapshots(client, era, SMaybe::Some(pools.into()))
        .await
        .unwrap();
    println!("result: {:?}", result);

    let result = queries_v16::get_genesis_config(client, era).await.unwrap();
    println!("result: {:?}", result);

    // DRep drep1ygpuetneftlmufa97hm5mf3xvqpdkyw656hyg6h20qaewtg3csnkc
    let drep_bytes =
        Bytes::from_str("03ccae794affbe27a5f5f74da6266002db11daa6ae446aea783b972d").unwrap();
    let dreps: BTreeSet<_> = [DRep::KeyHash(drep_bytes.clone())].into();

    let result = queries_v16::get_drep_stake_distr(client, era, dreps.into())
        .await
        .unwrap();
    println!("result: {:?}", result);

    let dreps: BTreeSet<_> = [StakeAddr::from((0x00, drep_bytes))].into();
    let result = queries_v16::get_drep_state(client, era, dreps.into())
        .await
        .unwrap();
    info!("result: {:02x?}", result);

    client.send_release().await.unwrap();
}

async fn do_chainsync(client: &mut NodeClient) {
    let known_points = vec![Point::Specific(
        77110778u64,
        hex::decode("18e6eeaa592c42113280ba47a0829355e6bed1c9ce67cce4be502d6031d0679a").unwrap(),
    )];

    let (point, _) = client
        .chainsync()
        .find_intersect(known_points)
        .await
        .unwrap();

    info!("intersected point is {:?}", point);

    loop {
        let next = client.chainsync().request_or_await_next().await.unwrap();
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
