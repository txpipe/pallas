use pallas_alonzo::{crypto, Block, BlockWrapper, Fragment};
use pallas_chainsync::{BlockLike, Consumer, NoopObserver};
use pallas_handshake::n2c::{Client, VersionTable};
use pallas_handshake::MAINNET_MAGIC;
use pallas_machines::run_agent;
use pallas_machines::{
    primitives::Point, DecodePayload, EncodePayload, PayloadDecoder, PayloadEncoder,
};
use pallas_multiplexer::Multiplexer;
use std::os::unix::net::UnixStream;

#[derive(Debug)]
pub struct Content(Block);

impl EncodePayload for Content {
    fn encode_payload(&self, _e: &mut PayloadEncoder) -> Result<(), Box<dyn std::error::Error>> {
        todo!()
    }
}

impl DecodePayload for Content {
    fn decode_payload(d: &mut PayloadDecoder) -> Result<Self, Box<dyn std::error::Error>> {
        d.tag()?;
        let bytes = d.bytes()?;
        let BlockWrapper(_, block) = BlockWrapper::decode_fragment(bytes)?;
        Ok(Content(block))
    }
}

impl BlockLike for Content {
    fn block_point(&self) -> Result<Point, Box<dyn std::error::Error>> {
        let hash = crypto::hash_block_header(&self.0.header)?;
        Ok(Point(self.0.header.header_body.slot, Vec::from(hash)))
    }
}

fn main() {
    env_logger::init();

    // we connect to the unix socket of the local node. Make sure you have the right
    // path for your environment
    let bearer = UnixStream::connect("/tmp/node.socket").unwrap();

    let mut muxer = Multiplexer::setup(bearer, &vec![0, 4, 5]).unwrap();

    let mut hs_channel = muxer.use_channel(0);
    let versions = VersionTable::v1_and_above(MAINNET_MAGIC);
    let last = run_agent(Client::initial(versions), &mut hs_channel).unwrap();
    println!("last hanshake state: {:?}", last);

    // some random known-point in the chain to use as starting point for the sync
    let known_points = vec![Point(
        45147459,
        hex::decode("bee16ef28ac02abb50c340a7deff085a77f3a7b84c66250b3318dcb125c19a10").unwrap(),
    )];

    let mut cs_channel = muxer.use_channel(5);
    let cs = Consumer::<Content, _>::initial(known_points, NoopObserver {});
    let cs = run_agent(cs, &mut cs_channel).unwrap();
    println!("{:?}", cs);
}
