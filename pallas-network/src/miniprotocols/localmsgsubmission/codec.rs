use pallas_codec::minicbor::{decode, encode, Decode, Decoder, Encode, Encoder};

use super::{DmqMsg, DmqMsgValidationError};

impl<'b> Decode<'b, ()> for DmqMsg {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        d.array()?;
        let msg_id = d.bytes()?.to_vec();
        let msg_body = d.bytes()?.to_vec();
        let block_number = d.u32()?;
        let ttl = d.u16()?;
        let kes_signature = d.bytes()?.to_vec();
        let operational_certificate = d.bytes()?.to_vec();
        let kes_period = d.u32()?;
        Ok(DmqMsg {
            msg_id,
            msg_body,
            block_number,
            ttl,
            kes_signature,
            operational_certificate,
            kes_period,
        })
    }
}

impl Encode<()> for DmqMsg {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        e.array(6)?;
        e.bytes(&self.msg_id)?;
        e.bytes(&self.msg_body)?;
        e.u32(self.block_number)?;
        e.u16(self.ttl)?;
        e.bytes(&self.kes_signature)?;
        e.bytes(&self.operational_certificate)?;
        e.u32(self.kes_period)?;

        Ok(())
    }
}

impl<'b, C> Decode<'b, C> for DmqMsgValidationError {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut C) -> Result<Self, decode::Error> {
        Ok(DmqMsgValidationError(d.str()?.to_string()))
    }
}

impl<C> Encode<C> for DmqMsgValidationError {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut C,
    ) -> Result<(), encode::Error<W::Error>> {
        e.str(&self.0)?;

        Ok(())
    }
}
