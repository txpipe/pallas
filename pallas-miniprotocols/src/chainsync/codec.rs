use crate::common::Point;
use crate::machines::{CodecError, DecodePayload, EncodePayload, PayloadDecoder, PayloadEncoder};

use super::{Message, Tip};

impl EncodePayload for Tip {
    fn encode_payload(&self, e: &mut PayloadEncoder) -> Result<(), Box<dyn std::error::Error>> {
        e.array(2)?;
        self.0.encode_payload(e)?;
        e.u64(self.1)?;

        Ok(())
    }
}

impl DecodePayload for Tip {
    fn decode_payload(d: &mut PayloadDecoder) -> Result<Self, Box<dyn std::error::Error>> {
        d.array()?;
        let point = Point::decode_payload(d)?;
        let block_num = d.u64()?;

        Ok(Tip(point, block_num))
    }
}

impl<C> EncodePayload for Message<C>
where
    C: EncodePayload + DecodePayload,
{
    fn encode_payload(&self, e: &mut PayloadEncoder) -> Result<(), Box<dyn std::error::Error>> {
        match self {
            Message::RequestNext => {
                e.array(1)?.u16(0)?;
                Ok(())
            }
            Message::AwaitReply => {
                e.array(1)?.u16(1)?;
                Ok(())
            }
            Message::RollForward(header, tip) => {
                e.array(3)?.u16(2)?;
                header.encode_payload(e)?;
                tip.encode_payload(e)?;
                Ok(())
            }
            Message::RollBackward(point, tip) => {
                e.array(3)?.u16(3)?;
                point.encode_payload(e)?;
                tip.encode_payload(e)?;
                Ok(())
            }
            Message::FindIntersect(points) => {
                e.array(2)?.u16(4)?;
                e.array(points.len() as u64)?;
                for point in points.iter() {
                    point.encode_payload(e)?;
                }
                Ok(())
            }
            Message::IntersectFound(point, tip) => {
                e.array(3)?.u16(5)?;
                point.encode_payload(e)?;
                tip.encode_payload(e)?;
                Ok(())
            }
            Message::IntersectNotFound(tip) => {
                e.array(1)?.u16(6)?;
                tip.encode_payload(e)?;
                Ok(())
            }
            Message::Done => {
                e.array(1)?.u16(7)?;
                Ok(())
            }
        }
    }
}

impl<C> DecodePayload for Message<C>
where
    C: EncodePayload + DecodePayload,
{
    fn decode_payload(d: &mut PayloadDecoder) -> Result<Self, Box<dyn std::error::Error>> {
        d.array()?;
        let label = d.u16()?;

        match label {
            0 => Ok(Message::RequestNext),
            1 => Ok(Message::AwaitReply),
            2 => {
                let content = C::decode_payload(d)?;
                let tip = Tip::decode_payload(d)?;
                Ok(Message::RollForward(content, tip))
            }
            3 => {
                let point = Point::decode_payload(d)?;
                let tip = Tip::decode_payload(d)?;
                Ok(Message::RollBackward(point, tip))
            }
            4 => {
                let points = Vec::<Point>::decode_payload(d)?;
                Ok(Message::FindIntersect(points))
            }
            5 => {
                let point = Point::decode_payload(d)?;
                let tip = Tip::decode_payload(d)?;
                Ok(Message::IntersectFound(point, tip))
            }
            6 => {
                let tip = Tip::decode_payload(d)?;
                Ok(Message::IntersectNotFound(tip))
            }
            7 => Ok(Message::Done),
            x => Err(Box::new(CodecError::BadLabel(x))),
        }
    }
}
