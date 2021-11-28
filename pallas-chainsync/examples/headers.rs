use net2::TcpStreamExt;
use std::net::TcpStream;

use pallas_chainsync::{ClientConsumer, NoopStorage, Point};
use pallas_handshake::n2n::{Client, VersionTable};
use pallas_handshake::MAINNET_MAGIC;
use pallas_machines::run_agent;
use pallas_multiplexer::Multiplexer;

fn main() {
    env_logger::init();

    let bearer = TcpStream::connect("relays-new.cardano-mainnet.iohk.io:3001").unwrap();
    bearer.set_nodelay(true).unwrap();
    bearer.set_keepalive_ms(Some(30_000u32)).unwrap();

    let mut muxer = Multiplexer::setup(bearer, &vec![0, 2]).unwrap();
    let hs_channel = muxer.use_channel(0);

    let versions = VersionTable::v4_and_above(MAINNET_MAGIC);
    let last = run_agent(Client::initial(versions), hs_channel).unwrap();
    println!("{:?}", last);

    let known_points = vec![Point(
        43847831u64,
        hex::decode("15b9eeee849dd6386d3770b0745e0450190f7560e5159b1b3ab13b14b2684a45").unwrap(),
    )];

    let cs_channel = muxer.use_channel(2);

    let cs = ClientConsumer::initial(known_points, NoopStorage {});
    let cs = run_agent(cs, cs_channel).unwrap();

    println!("{:?}", cs);
}
