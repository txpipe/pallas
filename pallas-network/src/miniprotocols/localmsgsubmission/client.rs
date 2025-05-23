use crate::miniprotocols::localtxsubmission::GenericClient;

use super::{DmqMsg, DmqMsgValidationError};

/// DMQ specific instantiation of LocalTxSubmission client.
pub type Client = GenericClient<DmqMsg, DmqMsgValidationError>;
