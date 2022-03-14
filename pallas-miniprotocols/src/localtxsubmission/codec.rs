use pallas_codec::{
    minicbor::{decode, encode, Decode, Decoder, Encode, Encoder},
    DecodeOwned,
};

use super::protocol::Message;

impl<T, E> Encode for Message<T, E>
where
    T: Encode,
    E: Encode,
{
    fn encode<W: encode::Write>(&self, e: &mut Encoder<W>) -> Result<(), encode::Error<W::Error>> {
        match self {
            Message::SubmitTx(tx) => {
                e.array(2)?;
                e.u8(0)?;

                e.encode(tx)?;

                Ok(())
            }
            Message::AcceptTx => {
                e.array(1)?.u8(1)?;
                Ok(())
            }
            Message::RejectTx(reason) => {
                e.array(2)?.u8(2)?;

                e.encode(reason)?;

                Ok(())
            }
            Message::Done => {
                e.array(1)?.u8(3)?;
                Ok(())
            }
        }
    }
}

impl<'b, T, E> Decode<'b> for Message<T, E>
where
    T: Decode<'b>,
    E: Decode<'b>,
{
    fn decode(d: &mut Decoder<'b>) -> Result<Self, decode::Error> {
        d.array()?;
        let variant = d.u8()?;

        match variant {
            0 => {
                d.tag()?;
                Ok(Message::SubmitTx(d.decode()?))
            }
            1 => Ok(Message::AcceptTx),
            2 => Ok(Message::RejectTx(d.decode()?)),
            3 => Ok(Message::Done),
            _ => Err(decode::Error::message(
                "unkown variant for localtxsubmission message",
            )),
        }
    }
}
