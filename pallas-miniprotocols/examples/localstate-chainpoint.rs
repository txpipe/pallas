use pallas_miniprotocols::handshake::{n2c::VersionTable, Initiator};
use pallas_miniprotocols::localstate::{
    queries::{QueryV10, RequestV10},
    OneShotClient,
};
use pallas_miniprotocols::run_agent;
use pallas_miniprotocols::MAINNET_MAGIC;
use pallas_multiplexer::Multiplexer;
use std::os::unix::net::UnixStream;

fn main() {
    env_logger::init();

    // we connect to the unix socket of the local node. Make sure you have the right
    // path for your environment
    let bearer = UnixStream::connect("/tmp/node.socket").unwrap();

    let mut muxer = Multiplexer::setup(bearer, &vec![0, 7]).unwrap();

    let mut hs_channel = muxer.use_channel(0);
    let versions = VersionTable::only_v10(MAINNET_MAGIC);
    let last = run_agent(Initiator::initial(versions), &mut hs_channel).unwrap();
    println!("last hanshake state: {:?}", last);

    let mut ls_channel = muxer.use_channel(7);

    let cs = OneShotClient::<QueryV10>::initial(None, RequestV10::GetChainPoint);
    let cs = run_agent(cs, &mut ls_channel).unwrap();
    println!("{:?}", cs);
}
