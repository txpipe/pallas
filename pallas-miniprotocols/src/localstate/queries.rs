use crate::machines::{DecodePayload, EncodePayload, PayloadDecoder};
use crate::payloads::PayloadEncoder;
use crate::primitives::Point;
use minicbor::{data::Cbor, Decoder};

use super::Query;

#[derive(Debug, Clone)]
pub struct BlockQuery {}

#[derive(Debug, Clone)]
pub enum RequestV10 {
    BlockQuery(BlockQuery),
    GetSystemStart,
    GetChainBlockNo,
    GetChainPoint,
}

impl EncodePayload for RequestV10 {
    fn encode_payload(&self, e: &mut PayloadEncoder) -> Result<(), Box<dyn std::error::Error>> {
        match self {
            Self::BlockQuery(..) => {
                todo!()
            }
            Self::GetSystemStart => {
                e.u16(1)?;
                Ok(())
            }
            Self::GetChainBlockNo => {
                e.u16(2)?;
                Ok(())
            }
            Self::GetChainPoint => {
                e.u16(3)?;
                Ok(())
            }
        }
    }
}

impl DecodePayload for RequestV10 {
    fn decode_payload(_d: &mut PayloadDecoder) -> Result<Self, Box<dyn std::error::Error>> {
        todo!()
    }
}

#[derive(Debug, Clone)]
pub struct GenericResponse(Vec<u8>);

impl EncodePayload for GenericResponse {
    fn encode_payload(&self, _e: &mut PayloadEncoder) -> Result<(), Box<dyn std::error::Error>> {
        todo!()
    }
}

impl DecodePayload for GenericResponse {
    fn decode_payload(d: &mut PayloadDecoder) -> Result<Self, Box<dyn std::error::Error>> {
        let cbor: Cbor = d.decode()?;
        let slice = cbor.as_ref();
        let vec = slice.to_vec();
        Ok(GenericResponse(vec))
    }
}

impl TryInto<Point> for GenericResponse {
    type Error = Box<dyn std::error::Error>;

    fn try_into(self) -> Result<Point, Self::Error> {
        let mut d = PayloadDecoder(Decoder::new(self.0.as_slice()));
        Point::decode_payload(&mut d)
    }
}

#[derive(Debug, Clone)]
pub struct QueryV10 {}

impl Query for QueryV10 {
    type Request = RequestV10;
    type Response = GenericResponse;
}
