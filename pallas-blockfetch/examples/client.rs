use net2::TcpStreamExt;
use std::net::TcpStream;

use pallas_blockfetch::{BlockFetchClient, Point};
use pallas_handshake::n2n::{Client, VersionTable};
use pallas_handshake::MAINNET_MAGIC;
use pallas_machines::run_agent;
use pallas_multiplexer::Multiplexer;

fn main() {
    env_logger::init();

    //let bearer = TcpStream::connect("localhost:6000").unwrap();
    let bearer =
        TcpStream::connect("relays-new.cardano-mainnet.iohk.io:3001").unwrap();

    bearer.set_nodelay(true).unwrap();
    bearer.set_keepalive_ms(Some(30_000u32)).unwrap();

    let mut handles = Multiplexer::new(bearer, &vec![0, 3]).unwrap();
    let (_, rx, tx) = handles.remove(0);

    let versions = VersionTable::v4_and_above(MAINNET_MAGIC);
    let last = run_agent(Client::initial(versions), rx, &tx).unwrap();
    println!("{:?}", last);

    let range = (
        Point(
            43847831u64,
            hex::decode("15b9eeee849dd6386d3770b0745e0450190f7560e5159b1b3ab13b14b2684a45")
                .unwrap(),
        ),
        Point(
            43847831u64,
            hex::decode("15b9eeee849dd6386d3770b0745e0450190f7560e5159b1b3ab13b14b2684a45")
                .unwrap(),
        ),
    );

    let (_, bf_rx, bf_tx) = handles.remove(0);

    let bf = BlockFetchClient::initial(range);

    let bf_last = run_agent(bf, bf_rx, &bf_tx);

    println!("{:?}", bf_last);
}
