use crate::miniprotocols::localtxsubmission::GenericServer;

use super::{DmqMsg, DmqMsgValidationError};

/// DMQ specific instantiation of LocalTxSubmission server.
pub type Server = GenericServer<DmqMsg, DmqMsgValidationError>;
