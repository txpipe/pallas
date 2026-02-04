use pallas_codec::minicbor::{decode, encode, Decode, Decoder, Encode, Encoder};

use crate::miniprotocols::localmsgsubmission::DmqMsgOperationalCertificate;

use super::{DmqMsg, DmqMsgPayload, DmqMsgRejectReason, DmqMsgValidationError};

impl<'b> Decode<'b, ()> for DmqMsg {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        d.array()?;
        let msg_payload = DmqMsgPayload::decode(d, _ctx)?;
        let kes_signature = d.bytes()?.to_vec();
        let operational_certificate = DmqMsgOperationalCertificate::decode(d, _ctx)?;
        let cold_verification_key = d.bytes()?.to_vec();
        Ok(DmqMsg {
            msg_payload,
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
        e.array(4)?;
        e.encode(&self.msg_payload)?;
        e.bytes(&self.kes_signature)?;
        e.encode(&self.operational_certificate)?;
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

impl<'b> Decode<'b, ()> for DmqMsgOperationalCertificate {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        d.array()?;
        let kes_vk = d.bytes()?.to_vec();
        let issue_number = d.u64()?;
        let start_kes_period = d.u64()?;
        let cert_sig = d.bytes()?.to_vec();
        Ok(DmqMsgOperationalCertificate {
            kes_vk,
            issue_number,
            start_kes_period,
            cert_sig,
        })
    }
}

impl Encode<()> for DmqMsgOperationalCertificate {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        e.array(4)?;
        e.bytes(&self.kes_vk)?;
        e.u64(self.issue_number)?;
        e.u64(self.start_kes_period)?;
        e.bytes(&self.cert_sig)?;

        Ok(())
    }
}

impl<'b, C> Decode<'b, C> for DmqMsgRejectReason {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut C) -> Result<Self, decode::Error> {
        let len = d.array()?;
        let expected_len = len.ok_or_else(|| {
            decode::Error::message(
                "Could not decode DmqMsgRejectReason: expected definite length array",
            )
        })?;

        if expected_len == 0 {
            return Err(decode::Error::message(
                "Could not decode DmqMsgRejectReason: empty array is not valid",
            ));
        }

        let tag: u8 = d.u8()?;
        match (tag, expected_len) {
            (0, 2) => {
                let reason = d.str()?.to_string();
                Ok(DmqMsgRejectReason::Invalid(reason))
            }
            (1, 1) => Ok(DmqMsgRejectReason::AlreadyReceived),
            (2, 1) => Ok(DmqMsgRejectReason::Expired),
            (3, 2) => {
                let reason = d.str()?.to_string();
                Ok(DmqMsgRejectReason::Other(reason))
            }
            (tag, len) => Err(decode::Error::message(format!(
                "Could not decode DmqMsgRejectReason: unknown tag {} with length {}",
                tag, len
            ))),
        }
    }
}

impl<C> Encode<C> for DmqMsgRejectReason {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut C,
    ) -> Result<(), encode::Error<W::Error>> {
        match self {
            DmqMsgRejectReason::Invalid(reason) => {
                e.array(2)?;
                e.u8(0)?;
                e.str(reason)?;
            }
            DmqMsgRejectReason::AlreadyReceived => {
                e.array(1)?;
                e.u8(1)?;
            }
            DmqMsgRejectReason::Expired => {
                e.array(1)?;
                e.u8(2)?;
            }
            DmqMsgRejectReason::Other(reason) => {
                e.array(2)?;
                e.u8(3)?;
                e.str(reason)?;
            }
        }
        Ok(())
    }
}

impl<'b, C> Decode<'b, C> for DmqMsgValidationError {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut C) -> Result<Self, decode::Error> {
        let reason = DmqMsgRejectReason::decode(d, _ctx)?;
        Ok(DmqMsgValidationError(reason))
    }
}

impl<C> Encode<C> for DmqMsgValidationError {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut C,
    ) -> Result<(), encode::Error<W::Error>> {
        self.0.encode(e, _ctx)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use pallas_codec::minicbor;

    use super::*;

    #[test]
    fn dmq_msg_encode_decode_simple_bytes() {
        let msg = DmqMsg {
            msg_payload: DmqMsgPayload {
                msg_id: vec![1, 2, 3],
                msg_body: vec![4, 5, 6],
                kes_period: 7,
                expires_at: 8,
            },
            kes_signature: vec![9, 10, 11],
            operational_certificate: DmqMsgOperationalCertificate {
                kes_vk: vec![12, 13, 14],
                issue_number: 15,
                start_kes_period: 16,
                cert_sig: vec![17],
            },
            cold_verification_key: vec![18, 19, 20],
        };
        let msg_cbor = vec![
            132, 132, 67, 1, 2, 3, 67, 4, 5, 6, 7, 8, 67, 9, 10, 11, 132, 67, 12, 13, 14, 15, 16,
            65, 17, 67, 18, 19, 20,
        ];

        let msg_encoded = minicbor::to_vec(&msg).unwrap();
        assert_eq!(msg_cbor, msg_encoded);

        let mut decoder = Decoder::new(&msg_cbor);
        let msg_decoded = DmqMsg::decode(&mut decoder, &mut ()).unwrap();
        assert_eq!(msg, msg_decoded);
    }

    #[test]
    fn dmq_msg_encode_decode_real_bytes() {
        let msg = DmqMsg {
            msg_payload: DmqMsgPayload {
                msg_id: vec![
                    27, 119, 238, 171, 12, 117, 96, 254, 9, 156, 130, 50, 211, 19, 8, 85, 151, 37,
                    65, 233, 58, 41, 28, 202, 219, 25, 104, 53, 26, 194, 164, 222, 98, 78, 191,
                    134, 182, 123, 78, 39, 123, 140, 87, 43, 34, 61, 147, 126, 188, 191, 59, 206,
                    163, 46, 157, 11, 10, 181, 186, 156, 224, 91, 48, 221,
                ],
                msg_body: vec![
                    100, 117, 109, 109, 121, 32, 109, 101, 115, 115, 97, 103, 101,
                ],
                kes_period: 0,
                expires_at: 1759769827,
            },
            kes_signature: vec![
                27, 138, 87, 160, 8, 168, 198, 216, 110, 38, 199, 29, 213, 25, 121, 65, 134, 218,
                255, 17, 57, 228, 172, 60, 186, 128, 157, 40, 188, 56, 122, 232, 59, 225, 190, 13,
                114, 42, 197, 120, 203, 71, 248, 129, 182, 171, 10, 2, 79, 195, 178, 195, 31, 118,
                239, 156, 56, 38, 3, 240, 197, 218, 65, 6, 67, 197, 81, 193, 139, 81, 122, 52, 234,
                209, 112, 172, 30, 153, 128, 211, 148, 112, 190, 84, 126, 6, 253, 123, 156, 253,
                36, 76, 23, 94, 139, 215, 29, 12, 155, 172, 3, 179, 6, 62, 196, 81, 176, 58, 176,
                101, 6, 116, 110, 163, 9, 108, 4, 199, 131, 144, 86, 215, 73, 170, 66, 181, 27,
                109, 0, 50, 149, 14, 247, 116, 23, 124, 98, 157, 164, 79, 47, 254, 156, 39, 188,
                200, 153, 66, 125, 13, 87, 128, 115, 197, 223, 181, 57, 146, 101, 23, 55, 29, 255,
                149, 121, 242, 4, 170, 63, 159, 198, 118, 36, 160, 10, 201, 165, 74, 138, 243, 129,
                44, 18, 106, 194, 69, 134, 148, 26, 108, 94, 171, 227, 186, 187, 91, 145, 111, 80,
                195, 22, 227, 29, 69, 97, 152, 19, 194, 49, 4, 240, 166, 40, 63, 133, 13, 202, 122,
                212, 10, 10, 193, 199, 242, 120, 224, 59, 103, 133, 42, 31, 253, 27, 63, 18, 100,
                168, 113, 29, 187, 121, 36, 226, 235, 19, 126, 65, 102, 67, 61, 50, 201, 33, 144,
                186, 87, 83, 27, 9, 51, 71, 40, 175, 93, 74, 242, 192, 98, 111, 206, 89, 68, 85,
                182, 173, 238, 185, 85, 42, 124, 90, 175, 241, 65, 191, 11, 225, 213, 30, 111, 140,
                171, 9, 190, 246, 129, 148, 120, 96, 121, 131, 28, 162, 93, 181, 135, 1, 230, 249,
                162, 31, 0, 101, 108, 124, 68, 185, 20, 80, 196, 167, 216, 17, 61, 156, 3, 146, 90,
                96, 247, 250, 199, 50, 212, 52, 15, 198, 175, 252, 83, 62, 89, 143, 247, 184, 218,
                102, 41, 159, 116, 106, 240, 98, 152, 45, 176, 133, 116, 253, 103, 198, 147, 136,
                70, 149, 62, 7, 77, 140, 254, 96, 55, 181, 180, 94, 139, 162, 80, 47, 176, 175,
                246, 143, 94, 224, 104, 255, 240, 130, 177, 242, 114, 12, 175, 254, 23, 79, 194,
                129, 32, 196, 69, 201, 98, 145, 11, 224, 107, 213, 139, 74, 28, 214, 124, 63, 84,
                80, 206, 207, 146, 175, 54, 46, 176, 87, 176, 133, 193, 5, 125, 43, 42, 217, 99,
                116, 126, 210, 250, 29, 244, 106, 236, 147, 92, 139, 35, 176, 85,
            ],
            operational_certificate: DmqMsgOperationalCertificate {
                kes_vk: vec![
                    50, 45, 160, 42, 80, 78, 184, 20, 210, 77, 140, 152, 63, 49, 165, 168, 5, 131,
                    101, 152, 110, 242, 144, 157, 176, 210, 5, 10, 166, 91, 196, 168,
                ],
                issue_number: 0,
                start_kes_period: 0,
                cert_sig: vec![
                    207, 135, 144, 168, 238, 41, 179, 216, 245, 74, 164, 231, 4, 158, 234, 141, 5,
                    19, 166, 11, 78, 34, 210, 211, 183, 72, 127, 83, 185, 156, 107, 55, 160, 190,
                    73, 251, 204, 47, 197, 86, 174, 231, 13, 49, 7, 83, 173, 177, 27, 53, 209, 66,
                    24, 203, 226, 152, 3, 91, 66, 56, 244, 206, 79, 0,
                ],
            },
            cold_verification_key: vec![
                77, 75, 24, 6, 47, 133, 2, 89, 141, 224, 69, 202, 123, 105, 240, 103, 245, 159,
                147, 177, 110, 58, 248, 115, 58, 152, 138, 220, 35, 65, 245, 200,
            ],
        };
        let msg_cbor = vec![
            132, 132, 88, 64, 27, 119, 238, 171, 12, 117, 96, 254, 9, 156, 130, 50, 211, 19, 8, 85,
            151, 37, 65, 233, 58, 41, 28, 202, 219, 25, 104, 53, 26, 194, 164, 222, 98, 78, 191,
            134, 182, 123, 78, 39, 123, 140, 87, 43, 34, 61, 147, 126, 188, 191, 59, 206, 163, 46,
            157, 11, 10, 181, 186, 156, 224, 91, 48, 221, 77, 100, 117, 109, 109, 121, 32, 109,
            101, 115, 115, 97, 103, 101, 0, 26, 104, 227, 244, 227, 89, 1, 192, 27, 138, 87, 160,
            8, 168, 198, 216, 110, 38, 199, 29, 213, 25, 121, 65, 134, 218, 255, 17, 57, 228, 172,
            60, 186, 128, 157, 40, 188, 56, 122, 232, 59, 225, 190, 13, 114, 42, 197, 120, 203, 71,
            248, 129, 182, 171, 10, 2, 79, 195, 178, 195, 31, 118, 239, 156, 56, 38, 3, 240, 197,
            218, 65, 6, 67, 197, 81, 193, 139, 81, 122, 52, 234, 209, 112, 172, 30, 153, 128, 211,
            148, 112, 190, 84, 126, 6, 253, 123, 156, 253, 36, 76, 23, 94, 139, 215, 29, 12, 155,
            172, 3, 179, 6, 62, 196, 81, 176, 58, 176, 101, 6, 116, 110, 163, 9, 108, 4, 199, 131,
            144, 86, 215, 73, 170, 66, 181, 27, 109, 0, 50, 149, 14, 247, 116, 23, 124, 98, 157,
            164, 79, 47, 254, 156, 39, 188, 200, 153, 66, 125, 13, 87, 128, 115, 197, 223, 181, 57,
            146, 101, 23, 55, 29, 255, 149, 121, 242, 4, 170, 63, 159, 198, 118, 36, 160, 10, 201,
            165, 74, 138, 243, 129, 44, 18, 106, 194, 69, 134, 148, 26, 108, 94, 171, 227, 186,
            187, 91, 145, 111, 80, 195, 22, 227, 29, 69, 97, 152, 19, 194, 49, 4, 240, 166, 40, 63,
            133, 13, 202, 122, 212, 10, 10, 193, 199, 242, 120, 224, 59, 103, 133, 42, 31, 253, 27,
            63, 18, 100, 168, 113, 29, 187, 121, 36, 226, 235, 19, 126, 65, 102, 67, 61, 50, 201,
            33, 144, 186, 87, 83, 27, 9, 51, 71, 40, 175, 93, 74, 242, 192, 98, 111, 206, 89, 68,
            85, 182, 173, 238, 185, 85, 42, 124, 90, 175, 241, 65, 191, 11, 225, 213, 30, 111, 140,
            171, 9, 190, 246, 129, 148, 120, 96, 121, 131, 28, 162, 93, 181, 135, 1, 230, 249, 162,
            31, 0, 101, 108, 124, 68, 185, 20, 80, 196, 167, 216, 17, 61, 156, 3, 146, 90, 96, 247,
            250, 199, 50, 212, 52, 15, 198, 175, 252, 83, 62, 89, 143, 247, 184, 218, 102, 41, 159,
            116, 106, 240, 98, 152, 45, 176, 133, 116, 253, 103, 198, 147, 136, 70, 149, 62, 7, 77,
            140, 254, 96, 55, 181, 180, 94, 139, 162, 80, 47, 176, 175, 246, 143, 94, 224, 104,
            255, 240, 130, 177, 242, 114, 12, 175, 254, 23, 79, 194, 129, 32, 196, 69, 201, 98,
            145, 11, 224, 107, 213, 139, 74, 28, 214, 124, 63, 84, 80, 206, 207, 146, 175, 54, 46,
            176, 87, 176, 133, 193, 5, 125, 43, 42, 217, 99, 116, 126, 210, 250, 29, 244, 106, 236,
            147, 92, 139, 35, 176, 85, 132, 88, 32, 50, 45, 160, 42, 80, 78, 184, 20, 210, 77, 140,
            152, 63, 49, 165, 168, 5, 131, 101, 152, 110, 242, 144, 157, 176, 210, 5, 10, 166, 91,
            196, 168, 0, 0, 88, 64, 207, 135, 144, 168, 238, 41, 179, 216, 245, 74, 164, 231, 4,
            158, 234, 141, 5, 19, 166, 11, 78, 34, 210, 211, 183, 72, 127, 83, 185, 156, 107, 55,
            160, 190, 73, 251, 204, 47, 197, 86, 174, 231, 13, 49, 7, 83, 173, 177, 27, 53, 209,
            66, 24, 203, 226, 152, 3, 91, 66, 56, 244, 206, 79, 0, 88, 32, 77, 75, 24, 6, 47, 133,
            2, 89, 141, 224, 69, 202, 123, 105, 240, 103, 245, 159, 147, 177, 110, 58, 248, 115,
            58, 152, 138, 220, 35, 65, 245, 200,
        ];

        let msg_encoded = minicbor::to_vec(&msg).unwrap();
        assert_eq!(msg_cbor, msg_encoded);

        let mut decoder = Decoder::new(&msg_cbor);
        let msg_decoded = DmqMsg::decode(&mut decoder, &mut ()).unwrap();
        assert_eq!(msg, msg_decoded);
    }

    #[test]
    fn dmq_msg_reject_reason_invalid() {
        let reason = DmqMsgRejectReason::Invalid(
            "InvalidKESSignature (KESPeriod 0) (KESPeriod 0)".to_string(),
        );
        let encoded = minicbor::to_vec(&reason).unwrap();

        let decoded: DmqMsgRejectReason = minicbor::decode(&encoded).unwrap();
        assert_eq!(decoded, reason);
    }

    #[test]
    fn dmq_msg_reject_reason_already_received() {
        let reason = DmqMsgRejectReason::AlreadyReceived;
        let encoded = minicbor::to_vec(&reason).unwrap();

        let decoded: DmqMsgRejectReason = minicbor::decode(&encoded).unwrap();
        assert_eq!(decoded, reason);
    }

    #[test]
    fn dmq_msg_reject_reason_expired() {
        let reason = DmqMsgRejectReason::Expired;
        let encoded = minicbor::to_vec(&reason).unwrap();

        let decoded: DmqMsgRejectReason = minicbor::decode(&encoded).unwrap();
        assert_eq!(decoded, reason);
    }

    #[test]
    fn dmq_msg_reject_reason_other() {
        let reason = DmqMsgRejectReason::Other("custom error".to_string());
        let encoded = minicbor::to_vec(&reason).unwrap();

        let decoded: DmqMsgRejectReason = minicbor::decode(&encoded).unwrap();
        assert_eq!(decoded, reason);
    }

    #[test]
    fn dmq_msg_validation_error() {
        let error = DmqMsgValidationError(DmqMsgRejectReason::Invalid(
            "InvalidKESSignature".to_string(),
        ));
        let encoded = minicbor::to_vec(&error).unwrap();

        let decoded: DmqMsgValidationError = minicbor::decode(&encoded).unwrap();
        assert_eq!(decoded, error);
    }
}
