use pallas_codec::minicbor::{data::Tag, decode, encode, Decode, Decoder, Encode, Encoder};

use super::Message;

impl Encode<()> for Message {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        match self {
            Message::RequestRange { range } => {
                e.array(3)?.u16(0)?;
                e.encode(&range.0)?;
                e.encode(&range.1)?;
                Ok(())
            }
            Message::ClientDone => {
                e.array(1)?.u16(1)?;
                Ok(())
            }
            Message::StartBatch => {
                e.array(1)?.u16(2)?;
                Ok(())
            }
            Message::NoBlocks => {
                e.array(1)?.u16(3)?;
                Ok(())
            }
            Message::Block { body } => {
                e.array(2)?.u16(4)?;
                e.tag(Tag::Cbor)?;
                e.bytes(body)?;
                Ok(())
            }
            Message::BatchDone => {
                e.array(1)?.u16(5)?;
                Ok(())
            }
        }
    }
}

impl<'b> Decode<'b, ()> for Message {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        d.array()?;
        let label = d.u16()?;

        match label {
            0 => {
                let point1 = d.decode()?;
                let point2 = d.decode()?;
                Ok(Message::RequestRange {
                    range: (point1, point2),
                })
            }
            1 => Ok(Message::ClientDone),
            2 => Ok(Message::StartBatch),
            3 => Ok(Message::NoBlocks),
            4 => {
                d.tag()?;
                let body = d.bytes()?;
                Ok(Message::Block {
                    body: Vec::from(body),
                })
            }
            5 => Ok(Message::BatchDone),
            _ => Err(decode::Error::message(
                "unknown variant for blockfetch message",
            )),
        }
    }
}
