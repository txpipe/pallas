use crate::common::Point;
use pallas_codec::{
    impl_fragment,
    minicbor::{decode, encode, Decode, Decoder, Encode, Encoder},
};

use super::{BlockContent, HeaderContent, Message, SkippedContent, Tip};

impl Encode for Tip {
    fn encode<W: encode::Write>(&self, e: &mut Encoder<W>) -> Result<(), encode::Error<W::Error>> {
        e.array(2)?;
        self.0.encode(e)?;
        e.u64(self.1)?;

        Ok(())
    }
}

impl<'b> Decode<'b> for Tip {
    fn decode(d: &mut Decoder<'b>) -> Result<Self, decode::Error> {
        d.array()?;
        let point = Point::decode(d)?;
        let block_num = d.u64()?;

        Ok(Tip(point, block_num))
    }
}

impl<C> Encode for Message<C>
where
    C: Encode,
{
    fn encode<W: encode::Write>(&self, e: &mut Encoder<W>) -> Result<(), encode::Error<W::Error>> {
        match self {
            Message::RequestNext => {
                e.array(1)?.u16(0)?;
                Ok(())
            }
            Message::AwaitReply => {
                e.array(1)?.u16(1)?;
                Ok(())
            }
            Message::RollForward(content, tip) => {
                e.array(3)?.u16(2)?;
                content.encode(e)?;
                tip.encode(e)?;
                Ok(())
            }
            Message::RollBackward(point, tip) => {
                e.array(3)?.u16(3)?;
                point.encode(e)?;
                tip.encode(e)?;
                Ok(())
            }
            Message::FindIntersect(points) => {
                e.array(2)?.u16(4)?;
                e.array(points.len() as u64)?;
                for point in points.iter() {
                    point.encode(e)?;
                }
                Ok(())
            }
            Message::IntersectFound(point, tip) => {
                e.array(3)?.u16(5)?;
                point.encode(e)?;
                tip.encode(e)?;
                Ok(())
            }
            Message::IntersectNotFound(tip) => {
                e.array(1)?.u16(6)?;
                tip.encode(e)?;
                Ok(())
            }
            Message::Done => {
                e.array(1)?.u16(7)?;
                Ok(())
            }
        }
    }
}

impl<'b, C> Decode<'b> for Message<C>
where
    C: Decode<'b>,
{
    fn decode(d: &mut Decoder<'b>) -> Result<Self, decode::Error> {
        d.array()?;
        let label = d.u16()?;

        match label {
            0 => Ok(Message::RequestNext),
            1 => Ok(Message::AwaitReply),
            2 => {
                let content = C::decode(d)?;
                let tip = Tip::decode(d)?;
                Ok(Message::RollForward(content, tip))
            }
            3 => {
                let point = Point::decode(d)?;
                let tip = Tip::decode(d)?;
                Ok(Message::RollBackward(point, tip))
            }
            4 => {
                let points = Vec::<Point>::decode(d)?;
                Ok(Message::FindIntersect(points))
            }
            5 => {
                let point = Point::decode(d)?;
                let tip = Tip::decode(d)?;
                Ok(Message::IntersectFound(point, tip))
            }
            6 => {
                let tip = Tip::decode(d)?;
                Ok(Message::IntersectNotFound(tip))
            }
            7 => Ok(Message::Done),
            _ => Err(decode::Error::message(
                "unknown variant for chainsync message",
            )),
        }
    }
}

impl<'b> Decode<'b> for HeaderContent {
    fn decode(d: &mut Decoder<'b>) -> Result<Self, decode::Error> {
        d.array()?;
        let variant = d.u8()?; // era variant

        match variant {
            // byron
            0 => {
                d.array()?;

                // can't find a reference anywhere about the structure of these values, but they
                // seem to provide the Byron-specific variant of the header
                let (a, b): (u8, u64) = d.decode()?;

                d.tag()?;
                let bytes = d.bytes()?;

                Ok(HeaderContent {
                    variant,
                    byron_prefix: Some((a, b)),
                    cbor: Vec::from(bytes),
                })
            }
            // shelley and beyond
            _ => {
                d.tag()?;
                let bytes = d.bytes()?;

                Ok(HeaderContent {
                    variant,
                    byron_prefix: None,
                    cbor: Vec::from(bytes),
                })
            }
        }
    }
}

impl Encode for HeaderContent {
    fn encode<W: encode::Write>(&self, _e: &mut Encoder<W>) -> Result<(), encode::Error<W::Error>> {
        todo!()
    }
}

impl_fragment!(Message<HeaderContent>);

impl<'b> Decode<'b> for BlockContent {
    fn decode(d: &mut Decoder<'b>) -> Result<Self, decode::Error> {
        d.tag()?;
        let bytes = d.bytes()?;
        Ok(BlockContent(Vec::from(bytes)))
    }
}

impl Encode for BlockContent {
    fn encode<W: encode::Write>(&self, _e: &mut Encoder<W>) -> Result<(), encode::Error<W::Error>> {
        todo!()
    }
}

impl_fragment!(Message<BlockContent>);

impl<'b> Decode<'b> for SkippedContent {
    fn decode(d: &mut Decoder<'b>) -> Result<Self, decode::Error> {
        d.skip()?;
        Ok(SkippedContent)
    }
}

impl Encode for SkippedContent {
    fn encode<W: encode::Write>(&self, _e: &mut Encoder<W>) -> Result<(), encode::Error<W::Error>> {
        todo!()
    }
}

impl_fragment!(Message<SkippedContent>);
