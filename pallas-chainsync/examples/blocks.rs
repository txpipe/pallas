use net2::TcpStreamExt;
use pallas_txsubmission::NaiveProvider;
use std::net::TcpStream;
use std::thread;
use std::time::Duration;

use pallas_chainsync::{Consumer, Point};
use pallas_handshake::n2c::{Client, VersionTable};
use pallas_handshake::MAINNET_MAGIC;
use pallas_machines::run_agent;
use pallas_multiplexer::Multiplexer;

fn main() {
    env_logger::init();

    //let bearer = TcpStream::connect("localhost:6000").unwrap();
    let bearer = TcpStream::connect("relays-new.cardano-mainnet.iohk.io:3001").unwrap();

    bearer.set_nodelay(true).unwrap();
    bearer.set_keepalive_ms(Some(30_000u32)).unwrap();

    let mut handles = Multiplexer::new(bearer, &vec![0, 4, 5]).unwrap();
    
    let (_, rx, tx) = handles.remove(0);
    let versions = VersionTable::v1_and_above(MAINNET_MAGIC);
    let last = run_agent(Client::initial(versions), rx, &tx).unwrap();
    println!("last hanshake state: {:?}", last);

    let (_, ts_rx, ts_tx) = handles.remove(0);
    let ts = NaiveProvider::initial(vec![]);
    let ts = run_agent(ts, ts_rx, &ts_tx).unwrap();
    println!("last tx-submission state: {:?}", ts);

    let known_points = vec![Point(
        43847831u64,
        hex::decode("15b9eeee849dd6386d3770b0745e0450190f7560e5159b1b3ab13b14b2684a45").unwrap(),
    )];

    let (_, cs_rx, cs_tx) = handles.remove(0);
    let cs = Consumer::initial(known_points);
    let cs = run_agent(cs, cs_rx, &cs_tx).unwrap();
    println!("{:?}", cs);
}
