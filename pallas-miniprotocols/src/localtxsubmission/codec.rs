/*

msgSubmitTx = [0, transaction ]
msgAcceptTx = [1]
msgRejectTx = [2, rejectReason ]
ltMsgDone   = [3]

*/

use crate::{CodecError, DecodePayload, EncodePayload, PayloadDecoder, PayloadEncoder};

use super::protocol::Message;

impl<T, E> EncodePayload for Message<T, E>
where
    T: EncodePayload + DecodePayload,
    E: EncodePayload + DecodePayload,
{
    fn encode_payload(&self, e: &mut PayloadEncoder) -> Result<(), Box<dyn std::error::Error>> {
        match self {
            Message::SubmitTx(tx) => {
                e.array(2)?;
                e.u8(0)?;

                tx.encode_payload(e)?;

                Ok(())
            }
            Message::AcceptTx => {
                e.array(1)?.u8(1)?;
                Ok(())
            }
            Message::RejectTx(reason) => {
                e.array(2)?.u8(2)?;

                reason.encode_payload(e)?;

                Ok(())
            }
            Message::Done => {
                e.array(1)?.u8(3)?;
                Ok(())
            }
        }
    }
}

impl<T, E> DecodePayload for Message<T, E>
where
    T: EncodePayload + DecodePayload,
    E: EncodePayload + DecodePayload,
{
    fn decode_payload(d: &mut PayloadDecoder) -> Result<Self, Box<dyn std::error::Error>> {
        d.array()?;
        let variant = d.u8()?;

        match variant {
            0 => {
                d.tag()?;
                Ok(Message::SubmitTx(d.decode_payload()?))
            }
            1 => Ok(Message::AcceptTx),
            2 => Ok(Message::RejectTx(d.decode_payload()?)),
            3 => Ok(Message::Done),
            x => Err(Box::new(CodecError::BadLabel(x as u16))),
        }
    }
}
