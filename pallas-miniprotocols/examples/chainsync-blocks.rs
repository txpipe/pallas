use pallas_miniprotocols::chainsync::{BlockContent, Consumer, NoopObserver};
use pallas_miniprotocols::handshake::{n2c::VersionTable, Initiator};
use pallas_miniprotocols::{run_agent, Point, MAINNET_MAGIC};
use pallas_multiplexer::Multiplexer;
use std::os::unix::net::UnixStream;

fn main() {
    env_logger::init();

    // we connect to the unix socket of the local node. Make sure you have the right
    // path for your environment
    let bearer = UnixStream::connect("/tmp/node.socket").unwrap();

    let mut muxer = Multiplexer::setup(bearer, &vec![0, 4, 5]).unwrap();

    let mut hs_channel = muxer.use_channel(0);
    let versions = VersionTable::v1_and_above(MAINNET_MAGIC);
    let last = run_agent(Initiator::initial(versions), &mut hs_channel).unwrap();
    println!("last hanshake state: {:?}", last);

    // some random known-point in the chain to use as starting point for the sync
    let known_points = vec![Point::Specific(
        45147459,
        hex::decode("bee16ef28ac02abb50c340a7deff085a77f3a7b84c66250b3318dcb125c19a10").unwrap(),
    )];

    let mut cs_channel = muxer.use_channel(5);
    let cs = Consumer::<BlockContent, _>::initial(Some(known_points), NoopObserver {});
    let cs = run_agent(cs, &mut cs_channel).unwrap();
    println!("{:?}", cs);
}
