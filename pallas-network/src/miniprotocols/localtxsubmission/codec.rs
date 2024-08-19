use pallas_codec::minicbor::data::Tag;
use pallas_codec::minicbor::{decode, encode, Decode, Decoder, Encode, Encoder};
use tracing::trace;

use crate::miniprotocols::localtxsubmission::{EraTx, Message};

use super::cardano_node_errors::{ApplyTxError, OuterScope};

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

/// Decodes Cardano node errors whose CBOR byte representation could be split over multiple
/// payloads.
pub struct NodeErrorDecoder {
    /// When decoding the error responses of the node, we use a stack to track the location of the
    /// decoding relative to an outer scope (most often a definite array). We need it because if we
    /// come across an error that we cannot handle, we must still consume all the CBOR bytes that
    /// represent this error.
    pub context_stack: Vec<OuterScope>,
    /// Response bytes from the cardano node. Note that there are payload limits and so the bytes
    /// may be truncated.
    pub response_bytes: Vec<u8>,
    /// This field is used to determine if there are still CBOR bytes that have yet to be decoded.
    ///
    /// It has a value of 0 if decoding has not yet started. Otherwise it takes the value of the
    /// index in `response_bytes` that is also pointed to by the minicbor decoder after a
    /// _successful_ decoding of a `TxApplyErrors` instance.
    pub ix_start_unprocessed_bytes: usize,
    /// This field is true if the current decoding of a `TXApplyErrors` instance is complete, which
    /// only happens once the CBOR BREAK token is decoded to terminate the indefinite array which is
    /// part of the `TxApplyErrors` encoded structure.
    pub cbor_break_token_seen: bool,
}

impl NodeErrorDecoder {
    pub fn new() -> Self {
        Self {
            context_stack: vec![],
            response_bytes: vec![],
            ix_start_unprocessed_bytes: 0,
            cbor_break_token_seen: false,
        }
    }
}

impl Default for NodeErrorDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl DecodeCBORSplitPayload for NodeErrorDecoder {
    type Entity = Message<EraTx, Vec<ApplyTxError>>;

    fn try_decode_with_new_bytes(
        &mut self,
        bytes: &[u8],
    ) -> Result<DecodingResult<Self::Entity>, decode::Error> {
        if self.has_undecoded_bytes() {
            self.response_bytes.extend_from_slice(bytes);
            let bytes = self.response_bytes.clone();
            let mut decoder = Decoder::new(&bytes);
            let mut errors = vec![];

            loop {
                match ApplyTxError::decode(&mut decoder, self) {
                    Ok(tx_err) => {
                        errors.push(tx_err);
                    }
                    Err(e) => {
                        if !e.is_end_of_input() {
                            return Err(e);
                        } else {
                            break;
                        }
                    }
                }
            }

            if self.has_undecoded_bytes() {
                Ok(DecodingResult::Incomplete(Message::RejectTx(errors)))
            } else {
                trace!(
                    "cardano node raw error bytes: {}",
                    hex::encode(&self.response_bytes)
                );
                self.response_bytes.clear();
                self.cbor_break_token_seen = false;
                self.ix_start_unprocessed_bytes = 0;
                assert!(self.context_stack.is_empty());
                Ok(DecodingResult::Complete(Message::RejectTx(errors)))
            }
        } else {
            // If it's not an error response then process it right here and return.
            let mut d = Decoder::new(bytes);
            let mut probe = d.probe();
            if probe.array().is_err() {
                // If we don't have any unprocessed bytes the first element should be an array
                return Err(decode::Error::message(
                    "Expecting an array (no unprocessed bytes)",
                ));
            }
            let label = probe.u16()?;
            match label {
                0 => {
                    d.array()?;
                    d.u16()?;
                    let tx = d.decode()?;
                    Ok(DecodingResult::Complete(Message::SubmitTx(tx)))
                }
                1 => Ok(DecodingResult::Complete(Message::AcceptTx)),
                2 => {
                    self.response_bytes.extend_from_slice(bytes);
                    let bytes = self.response_bytes.clone();
                    let mut decoder = Decoder::new(&bytes);
                    let mut errors = vec![];

                    loop {
                        match ApplyTxError::decode(&mut decoder, self) {
                            Ok(tx_err) => {
                                errors.push(tx_err);
                            }
                            Err(e) => {
                                if !e.is_end_of_input() {
                                    return Err(e);
                                } else {
                                    break;
                                }
                            }
                        }
                    }

                    if self.has_undecoded_bytes() {
                        Ok(DecodingResult::Incomplete(Message::RejectTx(errors)))
                    } else {
                        self.response_bytes.clear();
                        self.cbor_break_token_seen = false;
                        self.ix_start_unprocessed_bytes = 0;
                        assert!(self.context_stack.is_empty());
                        Ok(DecodingResult::Complete(Message::RejectTx(errors)))
                    }
                }
                3 => Ok(DecodingResult::Complete(Message::Done)),
                _ => Err(decode::Error::message("can't decode Message")),
            }
        }
    }

    fn has_undecoded_bytes(&self) -> bool {
        self.ix_start_unprocessed_bytes + 1 < self.response_bytes.len()
    }
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
