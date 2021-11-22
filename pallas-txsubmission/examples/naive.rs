use net2::TcpStreamExt;
use std::net::TcpStream;

use pallas_txsubmission::{NaiveProvider};
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

    let mut muxer = Multiplexer::try_setup(bearer, &vec![0, 4]).unwrap();

    let (hs_rx, hs_tx) = muxer.use_channel(0);
    let versions = VersionTable::v1_and_above(MAINNET_MAGIC);
    let last = run_agent(Client::initial(versions), hs_rx, &hs_tx).unwrap();
    println!("{:?}", last);


    let (ts_rx, ts_tx) = muxer.use_channel(4);
    let ts = NaiveProvider::initial(vec![]);
    let ts = run_agent(ts, ts_rx, &ts_tx).unwrap();

    println!("{:?}", ts);
}
