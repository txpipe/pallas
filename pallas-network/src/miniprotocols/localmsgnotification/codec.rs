use pallas_codec::minicbor::{decode, encode, Decode, Decoder, Encode, Encoder};

use super::Message;

impl Encode<()> for Message {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        match self {
            Message::RequestMessagesNonBlocking => {
                let is_blocking = false;
                e.array(2)?.u16(0)?.bool(is_blocking)?;
                Ok(())
            }
            Message::ReplyMessagesNonBlocking(msgs, has_more) => {
                e.array(3)?.u16(1)?;
                e.begin_array()?;
                for msg in msgs {
                    e.encode(msg)?;
                }
                e.end()?;
                e.bool(*has_more)?;
                Ok(())
            }
            Message::RequestMessagesBlocking => {
                let is_blocking = true;
                e.array(2)?.u16(0)?.bool(is_blocking)?;
                Ok(())
            }
            Message::ReplyMessagesBlocking(msgs) => {
                e.array(3)?.u16(2)?;
                e.begin_array()?;
                for msg in msgs {
                    e.encode(msg)?;
                }
                e.end()?;
                Ok(())
            }
            Message::ClientDone => {
                e.array(1)?.u16(3)?;
                Ok(())
            }
            Message::ServerDone => {
                e.array(1)?.u16(4)?;
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
                let is_blocking = d.bool()?;
                match is_blocking {
                    true => Ok(Message::RequestMessagesBlocking),
                    false => Ok(Message::RequestMessagesNonBlocking),
                }
            }
            1 => {
                let msgs = d.decode()?;
                let has_more = d.bool()?;
                Ok(Message::ReplyMessagesNonBlocking(msgs, has_more))
            }
            2 => {
                let msgs = d.decode()?;
                Ok(Message::ReplyMessagesBlocking(msgs))
            }
            3 => Ok(Message::ClientDone),
            4 => Ok(Message::ServerDone),
            _ => Err(decode::Error::message(
                "unknown variant for localmsgsubmission message",
            )),
        }
    }
}
