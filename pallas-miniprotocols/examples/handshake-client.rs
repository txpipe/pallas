use net2::TcpStreamExt;
use std::net::TcpStream;

use pallas_miniprotocols::handshake::n2c::{Client, VersionTable};
use pallas_miniprotocols::handshake::MAINNET_MAGIC;
use pallas_miniprotocols::run_agent;
use pallas_multiplexer::Multiplexer;

fn main() {
    env_logger::init();

    //let bearer = TcpStream::connect("localhost:6000").unwrap();
    let bearer = TcpStream::connect("relays-new.cardano-mainnet.iohk.io:3001").unwrap();

    bearer.set_nodelay(true).unwrap();
    bearer.set_keepalive_ms(Some(30_000u32)).unwrap();

    let mut muxer = Multiplexer::setup(bearer, &vec![0]).unwrap();

    let mut hs_channel = muxer.use_channel(0);
    let versions = VersionTable::v1_and_above(MAINNET_MAGIC);
    let last = run_agent(Client::initial(versions), &mut hs_channel).unwrap();

    println!("{:?}", last);
}
