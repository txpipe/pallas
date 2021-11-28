use minicbor::data::Cbor;
use pallas_localstate::{OneShotClient, Point, Query};
use pallas_handshake::n2c::{Client, VersionTable};
use pallas_handshake::{MAINNET_MAGIC};
use pallas_machines::{DecodePayload, EncodePayload, run_agent};
use pallas_multiplexer::Multiplexer;
use std::net::TcpStream;
use std::os::unix::net::UnixStream;
use net2::*;

#[derive(Debug, Clone)]
struct BlockQuery {}

#[derive(Debug, Clone)]
enum Request {
    BlockQuery(BlockQuery),
    GetSystemStart,
    GetChainBlockNo,
    GetChainPoint,
}


impl EncodePayload for Request {
    fn encode_payload(&self, e: &mut pallas_machines::PayloadEncoder) -> Result<(), Box<dyn std::error::Error>> {
        match self {
            Request::BlockQuery(block_query) => {
                e.u16(0)?;
                e.array(0)?;
                Ok(())
            }
            Request::GetSystemStart => {
                e.u16(1)?;
                Ok(())
            }
            Request::GetChainBlockNo => {
                e.u16(2)?;
                Ok(())
            }
            Request::GetChainPoint => {
                e.u16(3)?;
                Ok(())
            }
        }
    }
}

impl DecodePayload for Request {
    fn decode_payload(d: &mut pallas_machines::PayloadDecoder) -> Result<Self, Box<dyn std::error::Error>> {
        todo!()
    }
}

#[derive(Debug, Clone)]
enum Response {
    Generic(Vec<u8>),
}

impl EncodePayload for Response {
    fn encode_payload(&self, e: &mut pallas_machines::PayloadEncoder) -> Result<(), Box<dyn std::error::Error>> {
        todo!()
    }
}

impl DecodePayload for Response {
    fn decode_payload(d: &mut pallas_machines::PayloadDecoder) -> Result<Self, Box<dyn std::error::Error>> {
        let cbor: Cbor = d.decode()?;
        let slice = cbor.as_ref();
        let vec = slice.to_vec();
        Ok(Response::Generic(vec))
    }
}

#[derive(Debug, Clone)]
struct ShelleyQuery {}

impl Query for ShelleyQuery {
    type Request = Request;
    type Response = Response;
}

fn main() {
    env_logger::init();

    // we connect to the unix socket of the local node. Make sure you have the right
    // path for your environment
    let bearer = UnixStream::connect("/tmp/node.socket").unwrap();

    let mut muxer = Multiplexer::setup(bearer, &vec![0, 7]).unwrap();

    let hs_channel = muxer.use_channel(0);
    let versions = VersionTable::only_v10(MAINNET_MAGIC);
    let last = run_agent(Client::initial(versions), hs_channel).unwrap();
    println!("last hanshake state: {:?}", last);

    let ls_channel = muxer.use_channel(7);
    let cs = OneShotClient::<ShelleyQuery>::initial(None, Request::GetChainPoint);
    let cs = run_agent(cs, ls_channel).unwrap();
    println!("{:?}", cs);
}
