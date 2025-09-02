use pallas_codec::minicbor::{decode, encode, Decode, Decoder, Encode, Encoder};

use super::{DmqMsg, DmqMsgValidationError};

impl<'b> Decode<'b, ()> for DmqMsg {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        d.array()?;
        let msg_id = d.bytes()?.to_vec();
        let msg_body = d.bytes()?.to_vec();
        let kes_signature = d.bytes()?.to_vec();
        let kes_period = d.u64()?;
        let operational_certificate = d.bytes()?.to_vec();
        let cold_verification_key = d.bytes()?.to_vec();
        let expires_at = d.u32()?;
        Ok(DmqMsg {
            msg_id,
            msg_body,
            kes_signature,
            kes_period,
            operational_certificate,
            cold_verification_key,
            expires_at,
        })
    }
}

impl Encode<()> for DmqMsg {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        e.array(7)?;
        e.bytes(&self.msg_id)?;
        e.bytes(&self.msg_body)?;
        e.bytes(&self.kes_signature)?;
        e.u64(self.kes_period)?;
        e.bytes(&self.operational_certificate)?;
        e.bytes(&self.cold_verification_key)?;
        e.u32(self.expires_at)?;

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
