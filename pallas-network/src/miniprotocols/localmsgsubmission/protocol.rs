/// A message to be sent by the local message submission protocol.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DmqMsg {
    /// The payload of the message.
    pub msg_payload: DmqMsgPayload,

    /// The KES signature of the message created by the SPO sending the message.
    pub kes_signature: Vec<u8>,
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

    /// The operational certificate of the SPO that created the message.
    pub operational_certificate: Vec<u8>,

    /// The cold verification key of the SPO that created the message.
    pub cold_verification_key: Vec<u8>,

    /// The expiration timestamp of the message.
    pub expires_at: u32,
}

/// Reject reason.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DmqMsgValidationError(pub String);

impl From<String> for DmqMsgValidationError {
    fn from(string: String) -> DmqMsgValidationError {
        DmqMsgValidationError(string)
    }
}
