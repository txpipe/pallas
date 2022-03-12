use pallas_codec::{
    impl_fragment,
    minicbor::{data::Cbor, decode, encode, Decode, Decoder, Encode, Encoder},
};

use super::{Message, Query};

#[derive(Debug, Clone)]
pub struct BlockQuery {}

#[derive(Debug, Clone)]
pub enum RequestV10 {
    BlockQuery(BlockQuery),
    GetSystemStart,
    GetChainBlockNo,
    GetChainPoint,
}

impl Encode for RequestV10 {
    fn encode<W: encode::Write>(&self, e: &mut Encoder<W>) -> Result<(), encode::Error<W::Error>> {
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

impl<'b> Decode<'b> for RequestV10 {
    fn decode(_d: &mut Decoder<'b>) -> Result<Self, decode::Error> {
        todo!()
    }
}

#[derive(Debug, Clone)]
pub struct GenericResponse(Vec<u8>);

impl Encode for GenericResponse {
    fn encode<W: encode::Write>(&self, _e: &mut Encoder<W>) -> Result<(), encode::Error<W::Error>> {
        todo!()
    }
}

impl<'b> Decode<'b> for GenericResponse {
    fn decode(d: &mut Decoder<'b>) -> Result<Self, decode::Error> {
        let cbor: Cbor = d.decode()?;
        let slice = cbor.as_ref();
        let vec = slice.to_vec();
        Ok(GenericResponse(vec))
    }
}

#[derive(Debug, Clone)]
pub struct QueryV10 {}

impl Query for QueryV10 {
    type Request = RequestV10;
    type Response = GenericResponse;
}

impl_fragment!(Message<QueryV10>);
