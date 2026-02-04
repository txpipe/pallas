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

/// Rejection reason
///
/// The CBOR encoding follows this CDDL specification:
/// ```cddl
/// reason = invalid / alreadyReceived / expired / other
///
/// invalid         = [0, tstr]
/// alreadyReceived = [1]
/// expired         = [2]
/// other           = [3, tstr]
/// ```
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum DmqMsgRejectReason {
    /// The message is invalid.
    Invalid(String),
    /// The message has already been received and processed.
    AlreadyReceived,
    /// The message has expired.
    Expired,
    /// Other rejection reason.
    Other(String),
}

impl std::fmt::Display for DmqMsgRejectReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DmqMsgRejectReason::Invalid(reason) => write!(f, "Invalid: {}", reason),
            DmqMsgRejectReason::AlreadyReceived => write!(f, "Already received"),
            DmqMsgRejectReason::Expired => write!(f, "Expired"),
            DmqMsgRejectReason::Other(reason) => write!(f, "Other: {}", reason),
        }
    }
}

/// A DMQ message validation error.
///
/// This wraps a [DmqMsgRejectReason] according to CIP-137 specification.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DmqMsgValidationError(pub DmqMsgRejectReason);

impl From<DmqMsgRejectReason> for DmqMsgValidationError {
    fn from(reason: DmqMsgRejectReason) -> DmqMsgValidationError {
        DmqMsgValidationError(reason)
    }
}

impl From<String> for DmqMsgValidationError {
    fn from(s: String) -> Self {
        DmqMsgValidationError(DmqMsgRejectReason::Other(s))
    }
}

impl std::fmt::Display for DmqMsgValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
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
