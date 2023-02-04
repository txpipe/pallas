use pallas_codec::minicbor::{decode, encode, Decode, Decoder, Encode, Encoder};

use super::protocol::{Message, TxIdAndSize};

impl Encode<()> for TxIdAndSize {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        e.array(2)?;
        e.bytes(&self.0)?;
        e.u32(self.1)?;

        Ok(())
    }
}

impl<'b> Decode<'b, ()> for TxIdAndSize {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        d.array()?;
        let id = d.bytes()?;
        let size = d.u32()?;

        Ok(Self(Vec::from(id), size))
    }
}

impl Encode<()> for Message {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        match self {
            Message::Init => {
                e.array(1)?.u16(6)?;
                Ok(())
            }
            Message::RequestTxIds(blocking, ack, req) => {
                e.array(4)?.u16(0)?;
                e.bool(*blocking)?;
                e.u16(*ack)?;
                e.u16(*req)?;
                Ok(())
            }
            Message::ReplyTxIds(ids) => {
                e.array(2)?.u16(1)?;
                e.array(ids.len() as u64)?;
                for id in ids {
                    e.encode(id)?;
                }
                Ok(())
            }
            Message::RequestTxs(ids) => {
                e.array(2)?.u16(2)?;
                e.array(ids.len() as u64)?;
                for id in ids {
                    e.bytes(id)?;
                }
                Ok(())
            }
            Message::ReplyTxs(txs) => {
                e.array(2)?.u16(3)?;
                e.array(txs.len() as u64)?;
                for tx in txs {
                    e.bytes(tx)?;
                }
                Ok(())
            }
            Message::Done => {
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
                let blocking = d.bool()?;
                let ack = d.u16()?;
                let req = d.u16()?;
                Ok(Message::RequestTxIds(blocking, ack, req))
            }
            1 => {
                let items = d.decode()?;
                Ok(Message::ReplyTxIds(items))
            }
            2 => {
                let ids = d.decode()?;
                Ok(Message::RequestTxs(ids))
            }
            3 => {
                todo!()
            }
            4 => Ok(Message::Done),
            6 => Ok(Message::Init),
            _ => Err(decode::Error::message(
                "unknown variant for txsubmission message",
            )),
        }
    }
}
