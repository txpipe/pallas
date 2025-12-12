use std::convert::Infallible;

use pallas_codec::minicbor;

/// A message to be sent by the local message submission protocol.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DmqMsg {
    /// The payload of the message.
    pub msg_payload: DmqMsgPayload,

    /// The KES signature of the message created by the SPO sending the message.
    pub kes_signature: Vec<u8>,

    /// The operational certificate of the SPO that created the message.
    pub operational_certificate: DmqMsgOperationalCertificate,

    /// The cold verification key of the SPO that created the message.
    pub cold_verification_key: Vec<u8>,
}

/// The payload of a message to be submitted to the local message submission protocol.
///
/// Important: This message is not signed and should not be considered final.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DmqMsgPayload {
    /// The message id.
    pub msg_id: Vec<u8>,

    /// The message body.
    pub msg_body: Vec<u8>,

    /// The KES period at which the KES signature was created.
    pub kes_period: u64,

    /// The expiration timestamp of the message.
    pub expires_at: u32,
}

impl DmqMsgPayload {
    /// Returns the bytes to sign for the message payload.
    pub fn bytes_to_sign(&self) -> Result<Vec<u8>, minicbor::encode::Error<Infallible>> {
        minicbor::to_vec(self)
    }
}

/// The representation of an operational certificate in a DMQ message.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DmqMsgOperationalCertificate {
    pub kes_vk: Vec<u8>,
    pub issue_number: u64,
    pub start_kes_period: u64,
    pub cert_sig: Vec<u8>,
}

/// Reject reason.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DmqMsgValidationError(pub String);

impl From<String> for DmqMsgValidationError {
    fn from(string: String) -> DmqMsgValidationError {
        DmqMsgValidationError(string)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dmq_msg_payload_bytes_to_sign_golden() {
        let payload = DmqMsgPayload {
            msg_id: vec![1, 2, 3],
            msg_body: vec![4, 5, 6],
            kes_period: 7,
            expires_at: 14,
        };

        let bytes = payload.bytes_to_sign().unwrap();
        assert_eq!(vec![132, 67, 1, 2, 3, 67, 4, 5, 6, 7, 14], bytes);
    }
}
