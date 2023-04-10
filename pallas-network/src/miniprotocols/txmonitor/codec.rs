use super::protocol::*;
use pallas_codec::minicbor::{decode, encode, Decode, Encode, Encoder};

impl Encode<()> for Message {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        match self {
            Message::Done => {
                e.array(1)?.u16(0)?;
            }
            Message::Acquire => {
                e.array(1)?.u16(1)?;
            }
            Message::Acquired(slot) => {
                e.array(2)?.u16(2)?;
                e.encode(slot)?;
            }
            Message::Release => {
                e.array(1)?.u16(3)?;
            }
            // TODO: confirm if this is valid, I'm just assuming that label 4 is AwaitAcquire, can't
            // find the specs
            Message::AwaitAcquire => {
                e.array(1)?.u16(4)?;
            }
            Message::RequestNextTx => {
                e.array(1)?.u16(5)?;
            }
            Message::ResponseNextTx(None) => {
                e.array(1)?.u16(6)?;
            }
            Message::ResponseNextTx(Some(tx)) => {
                e.array(2)?.u16(6)?;
                e.encode(tx)?;
            }
            Message::RequestHasTx(tx) => {
                e.array(2)?.u16(7)?;
                e.encode(tx)?;
            }
            Message::ResponseHasTx(tx) => {
                e.array(2)?.u16(8)?;
                e.encode(tx)?;
            }
            Message::RequestSizeAndCapacity => {
                e.array(1)?.u16(9)?;
            }
            Message::ResponseSizeAndCapacity(sz) => {
                e.array(2)?.u16(10)?;
                e.array(3)?;
                e.encode(sz.capacity_in_bytes)?;
                e.encode(sz.size_in_bytes)?;
                e.encode(sz.number_of_txs)?;
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
            0 => Ok(Message::Done),
            1 => Ok(Message::Acquire),
            2 => {
                let slot = d.decode()?;
                Ok(Message::Acquired(slot))
            }
            3 => Ok(Message::Release),
            // TODO: confirm if this is valid, I'm just assuming that label 4 is AwaitAcquire, can't
            // find the specs
            4 => Ok(Message::AwaitAcquire),
            5 => Ok(Message::RequestNextTx),
            6 => match d.array()? {
                Some(_) => {
                    let cbor: pallas_codec::utils::CborWrap<Tx> = d.decode()?;
                    Ok(Message::ResponseNextTx(Some(cbor.unwrap())))
                }
                None => Ok(Message::ResponseNextTx(None)),
            },
            7 => {
                let id = d.decode()?;
                Ok(Message::RequestHasTx(id))
            }
            8 => {
                let has = d.decode()?;
                Ok(Message::ResponseHasTx(has))
            }
            9 => Ok(Message::RequestSizeAndCapacity),
            10 => {
                d.array()?;
                let capacity_in_bytes = d.decode()?;
                let size_in_bytes = d.decode()?;
                let number_of_txs = d.decode()?;

                Ok(Message::ResponseSizeAndCapacity(MempoolSizeAndCapacity {
                    capacity_in_bytes,
                    size_in_bytes,
                    number_of_txs,
                }))
            }
            _ => Err(decode::Error::message("can't decode Message")),
        }
    }
}
