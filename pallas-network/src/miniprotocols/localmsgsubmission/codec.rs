use pallas_codec::minicbor::{decode, encode, Decode, Decoder, Encode, Encoder};

use super::{DmqMsg, DmqMsgPayload, DmqMsgValidationError};

impl<'b> Decode<'b, ()> for DmqMsg {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        d.array()?;
        let msg_id = d.bytes()?.to_vec();
        let msg_body = d.bytes()?.to_vec();
        let kes_period = d.u64()?;
        let expires_at = d.u32()?;
        let kes_signature = d.bytes()?.to_vec();
        let operational_certificate = d.bytes()?.to_vec();
        let cold_verification_key = d.bytes()?.to_vec();
        Ok(DmqMsg {
            msg_payload: DmqMsgPayload {
                msg_id,
                msg_body,
                kes_period,
                expires_at,
            },
            kes_signature,
            operational_certificate,
            cold_verification_key,
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
        e.bytes(&self.msg_payload.msg_id)?;
        e.bytes(&self.msg_payload.msg_body)?;
        e.u64(self.msg_payload.kes_period)?;
        e.u32(self.msg_payload.expires_at)?;
        e.bytes(&self.kes_signature)?;
        e.bytes(&self.operational_certificate)?;
        e.bytes(&self.cold_verification_key)?;

        Ok(())
    }
}

impl<'b> Decode<'b, ()> for DmqMsgPayload {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        d.array()?;
        let msg_id = d.bytes()?.to_vec();
        let msg_body = d.bytes()?.to_vec();
        let kes_period = d.u64()?;
        let expires_at = d.u32()?;
        Ok(DmqMsgPayload {
            msg_id,
            msg_body,
            kes_period,
            expires_at,
        })
    }
}

impl Encode<()> for DmqMsgPayload {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        e.array(4)?;
        e.bytes(&self.msg_id)?;
        e.bytes(&self.msg_body)?;
        e.u64(self.kes_period)?;
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

#[cfg(test)]
mod tests {
    use pallas_codec::minicbor;

    use super::*;

    #[test]
    fn dmq_msg_encode_decode() {
        let msg = DmqMsg {
            msg_payload: DmqMsgPayload {
                msg_id: vec![1, 2, 3],
                msg_body: vec![4, 5, 6],
                kes_period: 7,
                expires_at: 8,
            },
            kes_signature: vec![9, 10, 11],
            operational_certificate: vec![12, 13, 14],
            cold_verification_key: vec![15, 16, 17],
        };
        let msg_cbor = vec![
            135, 67, 1, 2, 3, 67, 4, 5, 6, 7, 8, 67, 9, 10, 11, 67, 12, 13, 14, 67, 15, 16, 17,
        ];

        let msg_encoded = minicbor::to_vec(&msg).unwrap();
        assert_eq!(msg_cbor, msg_encoded);

        let mut decoder = Decoder::new(&msg_cbor);
        let msg_decoded = DmqMsg::decode(&mut decoder, &mut ()).unwrap();
        assert_eq!(msg, msg_decoded);
    }
}
