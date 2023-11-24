use pallas_codec::minicbor::{decode, display, encode, Decode, Encode, Encoder};

use super::{AcquireFailure, Message};

impl Encode<()> for AcquireFailure {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        let code = match self {
            AcquireFailure::PointTooOld => 0,
            AcquireFailure::PointNotOnChain => 1,
        };

        e.u16(code)?;

        Ok(())
    }
}

impl<'b> Decode<'b, ()> for AcquireFailure {
    fn decode(
        d: &mut pallas_codec::minicbor::Decoder<'b>,
        _ctx: &mut (),
    ) -> Result<Self, pallas_codec::minicbor::decode::Error> {
        let code = d.u16()?;

        match code {
            0 => Ok(AcquireFailure::PointTooOld),
            1 => Ok(AcquireFailure::PointNotOnChain),
            _ => Err(decode::Error::message(
                "can't infer acquire failure from variant id",
            )),
        }
    }
}

impl Encode<()> for Message {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        println!("encoding message {:?}", self);
        match self {
            Message::Acquire(Some(point)) => {
                e.array(2)?.u16(0)?;
                e.encode(point)?;
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
                e.encode(failure)?;
                Ok(())
            }
            Message::Query(query) => {
                let cddl = "CDDL [6 #6.258([ *addr ])]".to_string();
                println!("encoding query to cddl: {}", cddl);
                // println!("query q : {:?}", query.raw_bytes());
                let code = format!("{}", display(query.raw_bytes()));
                println!("code1: {}", code);
                e.array(2)?.u16(3)?;
                e.encode(query)?;
                Ok(())
            }
            Message::Result(result) => {
                e.array(2)?.u16(4)?;
                e.encode(result)?;
                Ok(())
            }
            Message::ReAcquire(Some(point)) => {
                e.array(2)?.u16(6)?;
                e.encode(point)?;
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

impl<'b> Decode<'b, ()> for Message {
    fn decode(
        d: &mut pallas_codec::minicbor::Decoder<'b>,
        _ctx: &mut (),
    ) -> Result<Self, pallas_codec::minicbor::decode::Error> {
        println!("decoding message");
        d.array()?;
        let label = d.u16()?;
        println!("decoding message with label {}", label);

        match label {
            0 => {
                let point = d.decode()?;
                Ok(Message::Acquire(Some(point)))
            }
            8 => Ok(Message::Acquire(None)),
            1 => Ok(Message::Acquired),
            2 => {
                let failure = d.decode()?;
                Ok(Message::Failure(failure))
            }
            3 => {
                let query = d.decode()?;
                Ok(Message::Query(query))
            }
            4 => {
                println!("decoding result");
                let response = d.decode()?;
                Ok(Message::Result(response))
            }
            5 => Ok(Message::Release),
            6 => {
                let point = d.decode()?;
                Ok(Message::ReAcquire(point))
            }
            9 => Ok(Message::ReAcquire(None)),
            7 => Ok(Message::Done),
            _ => Err(decode::Error::message(
                "unknown variant for localstate message",
            )),
        }
    }
}
