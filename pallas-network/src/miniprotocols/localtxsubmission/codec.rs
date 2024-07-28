use pallas_codec::minicbor::data::Tag;
use pallas_codec::minicbor::{decode, encode, Decode, Decoder, Encode, Encoder};

use crate::miniprotocols::localtxsubmission::{EraTx, Message};

use super::cardano_node_errors::ApplyTxError;

impl<Tx, Reject> Encode<()> for Message<Tx, Reject>
where
    Tx: Encode<()>,
    Reject: Encode<()>,
{
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        match self {
            Message::SubmitTx(tx) => {
                e.array(2)?.u16(0)?;
                e.encode(tx)?;
                Ok(())
            }
            Message::AcceptTx => {
                e.array(1)?.u16(1)?;
                Ok(())
            }
            Message::RejectTx(rejection) => {
                e.array(2)?.u16(2)?;
                e.encode(rejection)?;
                Ok(())
            }
            Message::Done => {
                e.array(1)?.u16(3)?;
                Ok(())
            }
        }
    }
}

#[derive(Debug)]
pub enum DecodingResult<Entity> {
    Complete(Entity),
    Incomplete(Entity),
}

impl<Entity: Encode<()>> Encode<()> for DecodingResult<Entity> {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        match self {
            DecodingResult::Complete(errors) | DecodingResult::Incomplete(errors) => {
                errors.encode(e, _ctx)
            }
        }
    }
}

/// An implementor of this trait is able to decode an entity from CBOR with bytes that are split
/// over multiple payloads.
pub trait DecodeCBORSplitPayload {
    /// Type of entity to decode
    type Entity;
    /// Attempt to decode entity given a new slice of bytes.
    fn try_decode_with_new_bytes(
        &mut self,
        bytes: &[u8],
    ) -> Result<DecodingResult<Self::Entity>, decode::Error>;
    /// Returns true if there still remain CBOR bytes to be decoded.
    fn has_undecoded_bytes(&self) -> bool;
}

impl<'b, C> Decode<'b, C> for DecodingResult<Message<EraTx, Vec<ApplyTxError>>>
where
    C: DecodeCBORSplitPayload<Entity = Message<EraTx, Vec<ApplyTxError>>>,
{
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        ctx.try_decode_with_new_bytes(d.input())
    }
}

impl<'b> Decode<'b, ()> for EraTx {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        d.array()?;
        let era = d.u16()?;
        let tag = d.tag()?;
        if tag != Tag::Cbor {
            return Err(decode::Error::message("Expected encoded CBOR data item"));
        }
        Ok(EraTx(era, d.bytes()?.to_vec()))
    }
}

impl Encode<()> for EraTx {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        e.array(2)?;
        e.u16(self.0)?;
        e.tag(Tag::Cbor)?;
        e.bytes(&self.1)?;
        Ok(())
    }
}
