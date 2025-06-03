use thiserror::Error;

use crate::miniprotocols::localmsgsubmission::DmqMsg;
use crate::multiplexer;

#[derive(Error, Debug)]
pub enum Error {
    #[error("attempted to receive message while agency is ours")]
    AgencyIsOurs,

    #[error("attempted to send message while agency is theirs")]
    AgencyIsTheirs,

    #[error("inbound message is not valid for current state")]
    InvalidInbound,

    #[error("outbound message is not valid for current state")]
    InvalidOutbound,

    #[error("protocol is already initialized, no need to wait for init message")]
    AlreadyInitialized,

    #[error("error while sending or receiving data through the channel")]
    Plexer(multiplexer::Error),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum State {
    Idle,
    BusyBlocking,
    BusyNonBlocking,
    Done,
}

#[derive(Debug)]
pub enum Message {
    RequestMessagesNonBlocking,
    ReplyMessagesNonBlocking(Vec<DmqMsg>, bool),
    RequestMessagesBlocking,
    ReplyMessagesBlocking(Vec<DmqMsg>),
    ClientDone,
    ServerDone,
}
