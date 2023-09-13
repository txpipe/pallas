use pallas_codec::minicbor::{decode, Decode, Decoder, encode, Encode, Encoder};

use crate::miniprotocols::localtxsubmission::Message;

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

impl<'b, Tx: Decode<'b, ()>, Reject: Decode<'b, ()>> Decode<'b, ()> for Message<Tx, Reject> {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        d.array()?;
        let label = d.u16()?;
        match label {
            0 => {
                let tx = d.decode()?;
                Ok(Message::SubmitTx(tx))
            }
            1 => Ok(Message::AcceptTx),
            2 => {
                let rejection = d.decode()?;
                Ok(Message::RejectTx(rejection))
            }
            3 => Ok(Message::Done),
            _ => Err(decode::Error::message("can't decode Message")),
        }
    }
}
