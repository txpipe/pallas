use net2::TcpStreamExt;
use pallas_primitives::alonzo::Header;

use pallas_miniprotocols::Point;
use std::net::TcpStream;

use pallas_miniprotocols::chainsync::{Consumer, HeaderContent, NoopObserver};
use pallas_miniprotocols::handshake::{n2n::VersionTable, Initiator};
use pallas_miniprotocols::{run_agent, MAINNET_MAGIC};
use pallas_multiplexer::Multiplexer;

#[derive(Debug)]
pub struct Content(u32, Header);

fn main() {
    env_logger::init();

    let bearer = TcpStream::connect("relays-new.cardano-mainnet.iohk.io:3001").unwrap();
    bearer.set_nodelay(true).unwrap();
    bearer.set_keepalive_ms(Some(30_000u32)).unwrap();

    let mut muxer = Multiplexer::setup(bearer, &vec![0, 2]).unwrap();
    let mut hs_channel = muxer.use_channel(0);

    let versions = VersionTable::v4_and_above(MAINNET_MAGIC);
    let last = run_agent(Initiator::initial(versions), &mut hs_channel).unwrap();
    println!("{:?}", last);

    let known_points = vec![Point::Specific(
        43847831u64,
        hex::decode("15b9eeee849dd6386d3770b0745e0450190f7560e5159b1b3ab13b14b2684a45").unwrap(),
    )];

    let mut cs_channel = muxer.use_channel(2);

    let cs = Consumer::<HeaderContent, _>::initial(Some(known_points), NoopObserver {});
    let cs = run_agent(cs, &mut cs_channel).unwrap();

    println!("{:?}", cs);
}
