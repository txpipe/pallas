use super::*;
use crate::machines::*;

impl EncodePayload for AcquireFailure {
    fn encode_payload(&self, e: &mut PayloadEncoder) -> Result<(), Box<dyn std::error::Error>> {
        let code = match self {
            AcquireFailure::PointTooOld => 0,
            AcquireFailure::PointNotInChain => 1,
        };

        e.u16(code)?;

        Ok(())
    }
}

impl DecodePayload for AcquireFailure {
    fn decode_payload(d: &mut PayloadDecoder) -> Result<Self, Box<dyn std::error::Error>> {
        let code = d.u16()?;

        match code {
            0 => Ok(AcquireFailure::PointTooOld),
            1 => Ok(AcquireFailure::PointNotInChain),
            _ => Err(Box::new(CodecError::UnexpectedCbor(
                "can't infer acquire failure from variant id",
            ))),
        }
    }
}

impl<Q: Query> EncodePayload for Message<Q> {
    fn encode_payload(&self, e: &mut PayloadEncoder) -> Result<(), Box<dyn std::error::Error>> {
        match self {
            Message::Acquire(Some(point)) => {
                e.array(2)?.u16(0)?;
                e.encode_payload(point)?;
                Ok(())
            }
            Message::Acquire(None) => {
                e.array(1)?.u16(8)?;
                Ok(())
            }
            Message::Acquired => {
                e.array(1)?.u16(1)?;
                Ok(())
            }
            Message::Failure(failure) => {
                e.array(2)?.u16(2)?;
                e.encode_payload(failure)?;
                Ok(())
            }
            Message::Query(query) => {
                e.array(2)?.u16(3)?;
                e.array(1)?;
                e.encode_payload(query)?;
                Ok(())
            }
            Message::Result(result) => {
                e.array(2)?.u16(4)?;
                e.array(1)?;
                e.encode_payload(result)?;
                Ok(())
            }
            Message::ReAcquire(Some(point)) => {
                e.array(2)?.u16(6)?;
                e.encode_payload(point)?;
                Ok(())
            }
            Message::ReAcquire(None) => {
                e.array(1)?.u16(9)?;
                Ok(())
            }
            Message::Release => {
                e.array(1)?.u16(5)?;
                Ok(())
            }
            Message::Done => {
                e.array(1)?.u16(7)?;
                Ok(())
            }
        }
    }
}

impl<Q: Query> DecodePayload for Message<Q> {
    fn decode_payload(d: &mut PayloadDecoder) -> Result<Self, Box<dyn std::error::Error>> {
        d.array()?;
        let label = d.u16()?;

        match label {
            0 => {
                let point = d.decode_payload()?;
                Ok(Message::Acquire(Some(point)))
            }
            8 => Ok(Message::Acquire(None)),
            1 => Ok(Message::Acquired),
            2 => {
                let failure = d.decode_payload()?;
                Ok(Message::Failure(failure))
            }
            3 => {
                let query = d.decode_payload()?;
                Ok(Message::Query(query))
            }
            4 => {
                let response = d.decode_payload()?;
                Ok(Message::Result(response))
            }
            5 => Ok(Message::Release),
            6 => {
                let point = d.decode_payload()?;
                Ok(Message::ReAcquire(point))
            }
            9 => Ok(Message::ReAcquire(None)),
            7 => Ok(Message::Done),
            x => Err(Box::new(CodecError::BadLabel(x))),
        }
    }
}
