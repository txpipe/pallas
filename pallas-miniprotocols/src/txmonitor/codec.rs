use super::{MempoolSizeAndCapacity, Message, MsgRequest, MsgResponse};
use pallas_codec::minicbor::{decode, encode, Decode, Encode, Encoder};

impl Encode<()> for Message {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        match self {
            Message::MsgDone => {
                e.array(1)?.u16(0)?;
            }
            Message::MsgAcquire => {
                e.array(1)?.u16(1)?;
            }
            Message::MsgAcquired(slot) => {
                e.array(2)?.u16(2)?;
                e.encode(slot)?;
            }
            Message::MsgQuery(query) => {
                query.encode(e, ctx)?;
            }
            Message::MsgResponse(response) => {
                response.encode(e, ctx)?;
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
            0 => Ok(Message::MsgDone),
            1 => Ok(Message::MsgAcquire),
            2 => {
                let slot = d.decode()?;
                Ok(Message::MsgAcquired(slot))
            }
            3 => Ok(Message::MsgQuery(MsgRequest::MsgRelease)),
            5 => Ok(Message::MsgQuery(MsgRequest::MsgNextTx)),
            6 => {
                d.array()?;
                let tag: Result<u8, pallas_codec::minicbor::decode::Error> = d.u8();
                let mut tx = None;

                if tag.is_ok() {
                    d.tag()?;
                    let cbor = d.bytes()?;
                    tx = Some(hex::encode(cbor));
                }
                Ok(Message::MsgResponse(MsgResponse::MsgReplyNextTx(tx)))
            }
            7 => {
                let txid = d.decode()?;
                Ok(Message::MsgQuery(MsgRequest::MsgHasTx(txid)))
            }
            8 => {
                let has = d.decode()?;
                Ok(Message::MsgResponse(MsgResponse::MsgReplyHasTx(has)))
            }
            9 => Ok(Message::MsgQuery(MsgRequest::MsgGetSizes)),
            10 => {
                d.array()?;
                let capacity_in_bytes = d.decode()?;
                let size_in_bytes = d.decode()?;
                let number_of_txs = d.decode()?;

                Ok(Message::MsgResponse(MsgResponse::MsgReplyGetSizes(
                    MempoolSizeAndCapacity {
                        capacity_in_bytes,
                        size_in_bytes,
                        number_of_txs,
                    },
                )))
            }
            _ => Err(decode::Error::message("can't decode Message")),
        }
    }

    fn nil() -> Option<Self> {
        None
    }
}

impl Encode<()> for MsgRequest {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        match self {
            MsgRequest::MsgAwaitAcquire => {
                e.array(1)?.u16(1)?;
            }
            MsgRequest::MsgGetSizes => {
                e.array(1)?.u16(9)?;
            }
            MsgRequest::MsgHasTx(tx) => {
                e.array(2)?.u16(7)?;
                e.encode(tx)?;
            }
            MsgRequest::MsgNextTx => {
                e.array(1)?.u16(5)?;
            }
            MsgRequest::MsgRelease => {
                e.array(1)?.u16(3)?;
            }
        }

        Ok(())
    }
}

impl Encode<()> for MsgResponse {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        match self {
            MsgResponse::MsgReplyGetSizes(sz) => {
                e.array(2)?.u16(10)?;
                e.array(3)?;
                e.encode(sz.capacity_in_bytes)?;
                e.encode(sz.size_in_bytes)?;
                e.encode(sz.number_of_txs)?;
            }
            MsgResponse::MsgReplyHasTx(tx) => {
                e.array(2)?.u16(8)?;
                e.encode(tx)?;
            }
            MsgResponse::MsgReplyNextTx(None) => {
                e.array(1)?.u16(6)?;
            }
            MsgResponse::MsgReplyNextTx(Some(tx)) => {
                e.array(2)?.u16(6)?;
                e.encode(tx.to_string())?;
            }
        }
        Ok(())
    }

    fn is_nil(&self) -> bool {
        false
    }
}
