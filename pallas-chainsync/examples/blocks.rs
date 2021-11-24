use pallas_chainsync::{ClientConsumer, Point};
use pallas_handshake::n2c::{Client, VersionTable};
use pallas_handshake::MAINNET_MAGIC;
use pallas_machines::run_agent;
use pallas_multiplexer::Multiplexer;
use std::os::unix::net::UnixStream;

fn main() {
    env_logger::init();

    // we connect to the unix socket of the local node. Make sure you have the right
    // path for your environment
    let bearer = UnixStream::connect("/tmp/node.socket").unwrap();

    let mut muxer = Multiplexer::try_setup(bearer, &vec![0, 4, 5]).unwrap();

    let (rx, tx) = muxer.use_channel(0);
    let versions = VersionTable::v1_and_above(MAINNET_MAGIC);
    let last = run_agent(Client::initial(versions), rx, &tx).unwrap();
    println!("last hanshake state: {:?}", last);

    // some random known-point in the chain to use as starting point for the sync
    let known_points = vec![Point(
        43847831u64,
        hex::decode("15b9eeee849dd6386d3770b0745e0450190f7560e5159b1b3ab13b14b2684a45").unwrap(),
    )];

    let (cs_rx, cs_tx) = muxer.use_channel(5);
    let cs = ClientConsumer::initial(known_points);
    let cs = run_agent(cs, cs_rx, &cs_tx).unwrap();
    println!("{:?}", cs);
}
