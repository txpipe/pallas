use super::protocol::*;
use pallas_codec::minicbor::{decode, encode, Decode, Encode, Encoder};

impl Encode<()> for Message {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        match self {
            Message::KeepAlive(cookie) => {
                e.array(2)?.u16(0)?;
                e.encode(cookie)?;
            }
            Message::ResponseKeepAlive(cookie) => {
                e.array(2)?.u16(1)?;
                e.encode(cookie)?;
            }
            Message::Done => {
                e.array(1)?.u16(2)?;
            }
        }

        Ok(())
    }
}

impl<'b> Decode<'b, ()> for Message {
    fn decode(
        d: &mut pallas_codec::minicbor::Decoder<'b>,
        _ctx: &mut (),
    ) -> Result<Self, decode::Error> {
        d.array()?;
        let label = d.u16()?;

        match label {
            0 => {
                let cookie = d.decode()?;
                Ok(Message::KeepAlive(cookie))
            }
            1 => {
                let cookie = d.decode()?;
                Ok(Message::ResponseKeepAlive(cookie))
            }
            2 => Ok(Message::Done),
            _ => Err(decode::Error::message("can't decode Message")),
        }
    }
}
