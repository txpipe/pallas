use pallas_codec::minicbor::{decode, encode, Decode, Decoder, Encode, Encoder};

use super::{
    protocol::{Message, TxIdAndSize},
    EraTxId, TxBody,
};

impl<TxId: Encode<()>> Encode<()> for TxIdAndSize<TxId> {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        e.array(2)?;
        e.encode(&self.0)?;
        e.u32(self.1)?;

        Ok(())
    }
}

impl<'b, TxId: Decode<'b, ()>> Decode<'b, ()> for TxIdAndSize<TxId> {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        d.array()?;

        let tx_id = d.decode()?;

        let size = d.u32()?;

        Ok(Self(tx_id, size))
    }
}

impl<TxId: Encode<()>> Encode<()> for Message<TxId> {
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
                e.begin_array()?;
                for id in ids {
                    e.encode(id)?;
                }
                e.end()?;
                Ok(())
            }
            Message::RequestTxs(ids) => {
                e.array(2)?.u16(2)?;
                e.begin_array()?;
                for id in ids {
                    e.encode(id)?;
                }
                e.end()?;
                Ok(())
            }
            Message::ReplyTxs(txs) => {
                e.array(2)?.u16(3)?;
                e.begin_array()?;
                for tx in txs {
                    e.bytes(&tx.0)?;
                }
                e.end()?;
                Ok(())
            }
            Message::Done => {
                e.array(1)?.u16(4)?;
                Ok(())
            }
        }
    }
}

impl<'b> Decode<'b, ()> for TxBody {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        d.array()?;
        // TODO: the TxBody encoding here needs to be pinned down and parameterized, the
        // same way we did TxId!
        d.u16()?; // Era?
        d.tag()?; // tag 24?
        Ok(TxBody(d.bytes()?.to_vec()))
    }
}

impl<'b, TxId: Decode<'b, ()>> Decode<'b, ()> for Message<TxId> {
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
            3 => Ok(Message::ReplyTxs(
                d.array_iter()?.collect::<Result<_, _>>()?,
            )),
            4 => Ok(Message::Done),
            6 => Ok(Message::Init),
            _ => Err(decode::Error::message(
                "unknown variant for txsubmission message",
            )),
        }
    }
}

impl Encode<()> for EraTxId {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        e.array(2)?;
        e.encode(self.0)?;
        e.bytes(&self.1)?;

        Ok(())
    }
}

impl<'b> Decode<'b, ()> for EraTxId {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        d.array()?;

        let era = d.u16()?;

        let tx_id = d.bytes()?;

        Ok(Self(era, tx_id.to_vec()))
    }
}
