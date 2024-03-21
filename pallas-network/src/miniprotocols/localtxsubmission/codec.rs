use pallas_codec::minicbor::data::Tag;
use pallas_codec::minicbor::{decode, encode, Decode, Decoder, Encode, Encoder};

use crate::miniprotocols::localtxsubmission::{EraTx, Message, RejectReason};

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
        if let Err(_) = d.array() {
            // if the first element isn't an array, it's a plutus error
            // the node sends string data
            let rejection = d.decode()?;
            // skip this data via setting the decoder position, because it doesn't recognize it with rejection decode
            let _ = d.set_position(d.input().len());
            return Ok(Message::RejectTx(rejection));
        }
        let label = d.u16()?;
        match label {
            0 => {
                let tx = d.decode()?;
                Ok(Message::SubmitTx(tx))
            }
            1 => Ok(Message::AcceptTx),
            2 => {
                let rejection = d.decode()?;
                // skip this data via setting the decoder position, because it doesn't recognize it with rejection decode
                let _ = d.set_position(d.input().len());
                Ok(Message::RejectTx(rejection))
            }
            3 => Ok(Message::Done),
            _ => Err(decode::Error::message("can't decode Message")),
        }
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

impl<'b> Decode<'b, ()> for RejectReason {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        let remainder = d.input().to_vec();
        Ok(RejectReason(remainder))
    }
}

impl Encode<()> for RejectReason {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        e.writer_mut()
            .write_all(&self.0)
            .map_err(encode::Error::write)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use pallas_codec::{minicbor, Fragment};

    use crate::miniprotocols::localtxsubmission::{EraTx, Message, RejectReason};
    use crate::multiplexer::Error;

    #[test]
    fn decode_reject_message() {
        let mut bytes = hex::decode(RAW_REJECT_RESPONSE).unwrap();
        let msg_res = try_decode_message::<Message<EraTx, RejectReason>>(&mut bytes);
        assert!(msg_res.is_ok())
    }

    fn try_decode_message<M>(buffer: &mut Vec<u8>) -> Result<Option<M>, Error>
    where
        M: Fragment,
    {
        let mut decoder = minicbor::Decoder::new(buffer);
        let maybe_msg = decoder.decode();

        match maybe_msg {
            Ok(msg) => {
                let pos = decoder.position();
                buffer.drain(0..pos);
                Ok(Some(msg))
            }
            Err(err) if err.is_end_of_input() => Ok(None),
            Err(err) => Err(Error::Decoding(err.to_string())),
        }
    }

    const RAW_REJECT_RESPONSE: &str =
        "82028182059f820082018200820a81581c3b890fb5449baedf5342a48ee9c9ec6acbc995641be92ad21f08c686\
        8200820183038158202628ce6ff8cc7ff0922072d930e4a693c17f991748dedece0be64819a2f9ef7782582031d\
        54ce8d7e8cb262fc891282f44e9d24c3902dc38fac63fd469e8bf3006376b5820750852fdaf0f2dd724291ce007\
        b8e76d74bcf28076ed0c494cd90c0cfe1c9ca582008201820782000000018200820183048158201a547638b4cf4\
        a3cec386e2f898ac6bc987fadd04277e1d3c8dab5c505a5674e8158201457e4107607f83a80c3c4ffeb70910c2b\
        a3a35cf1699a2a7375f50fcc54a931820082028201830500821a00636185a2581c6f1a1f0c7ccf632cc9ff4b796\
        87ed13ffe5b624cce288b364ebdce50a144414749581b000000032a9f8800581c795ecedb09821cb922c13060c8\
        f6377c3344fa7692551e865d86ac5da158205399c766fb7c494cddb2f7ae53cc01285474388757bc05bd575c14a\
        713a432a901820082028201820085825820497fe6401e25733c073c01164c7f2a1a05de8c95e36580f9d1b05123\
        70040def028258207911ba2b7d91ac56b05ea351282589fe30f4717a707a1b9defaf282afe5ba44200825820791\
        1ba2b7d91ac56b05ea351282589fe30f4717a707a1b9defaf282afe5ba44201825820869bcb6f35e6b7912c25e5\
        cb33fb9906b097980a83f2b8ef40b51c4ef52eccd402825820efc267ad2c15c34a117535eecc877241ed836eb3e\
        643ec90de21ca1b12fd79c20282008202820181148200820283023a000f0f6d1a004944ce820082028201830d3a\
        000f0f6d1a00106253820082028201830182811a02409e10811a024138c01a0255e528ff";
}
