use pallas_codec::minicbor::{Decode, Decoder, Encode, Encoder, data::IanaTag, decode, encode};

use crate::protocol::Error;

/// Protocol channel number for node-to-node tx-submission
pub const CHANNEL_ID: u16 = 4;

pub type Blocking = bool;

pub type TxCount = u16;

pub type TxSizeInBytes = u32;

// The bytes of a txId, tagged with an era number
#[derive(Debug, Clone)]
pub struct EraTxId(pub u16, pub Vec<u8>);

// The bytes of a transaction, with an era number and some raw CBOR
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct EraTxBody(pub u16, pub Vec<u8>);

#[derive(Debug, Clone)]
pub struct TxIdAndSize<TxID>(pub TxID, pub TxSizeInBytes);

#[derive(Debug, Clone)]
pub enum Message {
    Init,
    RequestTxIds(Blocking, TxCount, TxCount),
    ReplyTxIds(Vec<TxIdAndSize<EraTxId>>),
    RequestTxs(Vec<EraTxId>),
    ReplyTxs(Vec<EraTxBody>),
    Done,
}

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub enum State {
    #[default]
    Init,
    Idle,
    TxIdsNonBlocking,
    TxIdsBlocking,
    Txs(Vec<EraTxBody>),
    Done,
}

impl State {
    pub fn apply(&self, msg: &Message) -> Result<Self, Error> {
        match self {
            State::Init => match msg {
                Message::Init => Ok(State::Idle),
                _ => Err(Error::InvalidInbound),
            },
            State::Idle => match msg {
                Message::RequestTxIds(..) => Ok(State::TxIdsBlocking),
                Message::RequestTxs(..) => Ok(State::Txs(Vec::new())),
                _ => Err(Error::InvalidInbound),
            },
            State::TxIdsNonBlocking => match msg {
                Message::ReplyTxIds(..) => Ok(State::TxIdsNonBlocking),
                _ => Err(Error::InvalidInbound),
            },
            State::TxIdsBlocking => match msg {
                Message::ReplyTxIds(..) => Ok(State::TxIdsBlocking),

                _ => Err(Error::InvalidInbound),
            },
            State::Txs(_) => match msg {
                Message::ReplyTxs(txs) => Ok(State::Txs(txs.clone())),

                _ => Err(Error::InvalidInbound),
            },
            State::Done => Err(Error::InvalidInbound),
        }
    }
}

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
                    e.encode(tx)?;
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

impl<'b> Decode<'b, ()> for EraTxBody {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        d.array()?;
        let era = d.u16()?;
        let tag = d.tag()?;
        if tag != IanaTag::Cbor.tag() {
            return Err(decode::Error::message("Expected encoded CBOR data item"));
        }
        Ok(EraTxBody(era, d.bytes()?.to_vec()))
    }
}

impl Encode<()> for EraTxBody {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        e.array(2)?;
        e.u16(self.0)?;
        e.tag(IanaTag::Cbor)?;
        e.bytes(&self.1)?;
        Ok(())
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
