use minicbor::data::Tag;
use net2::TcpStreamExt;
use pallas_primitives::alonzo::Header;
use pallas_primitives::Fragment;

use pallas_miniprotocols::Point;
use std::net::TcpStream;

use pallas_miniprotocols::chainsync::{Consumer, NoopObserver};
use pallas_miniprotocols::handshake::n2n::{Client, VersionTable};
use pallas_miniprotocols::{
    run_agent, DecodePayload, EncodePayload, PayloadDecoder, PayloadEncoder, MAINNET_MAGIC,
};
use pallas_multiplexer::Multiplexer;

#[derive(Debug)]
pub struct Content(u32, Header);

impl EncodePayload for Content {
    fn encode_payload(&self, e: &mut PayloadEncoder) -> Result<(), Box<dyn std::error::Error>> {
        e.array(2)?;
        e.u32(self.0)?;
        e.tag(Tag::Cbor)?;
        e.bytes(&self.1.encode_fragment()?)?;

        Ok(())
    }
}

impl DecodePayload for Content {
    fn decode_payload(d: &mut PayloadDecoder) -> Result<Self, Box<dyn std::error::Error>> {
        d.array()?;
        let unknown = d.u32()?; // WTF is this value?
        d.tag()?;
        let bytes = d.bytes()?;
        let header = Header::decode_fragment(bytes)?;
        Ok(Content(unknown, header))
    }
}

fn main() {
    env_logger::init();

    let bearer = TcpStream::connect("relays-new.cardano-mainnet.iohk.io:3001").unwrap();
    bearer.set_nodelay(true).unwrap();
    bearer.set_keepalive_ms(Some(30_000u32)).unwrap();

    let mut muxer = Multiplexer::setup(bearer, &vec![0, 2]).unwrap();
    let mut hs_channel = muxer.use_channel(0);

    let versions = VersionTable::v4_and_above(MAINNET_MAGIC);
    let last = run_agent(Client::initial(versions), &mut hs_channel).unwrap();
    println!("{:?}", last);

    let known_points = vec![Point(
        43847831u64,
        hex::decode("15b9eeee849dd6386d3770b0745e0450190f7560e5159b1b3ab13b14b2684a45").unwrap(),
    )];

    let mut cs_channel = muxer.use_channel(2);

    let cs = Consumer::<Content, _>::initial(known_points, NoopObserver {});
    let cs = run_agent(cs, &mut cs_channel).unwrap();

    println!("{:?}", cs);
}
