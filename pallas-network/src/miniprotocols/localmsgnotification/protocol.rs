use thiserror::Error;

use crate::miniprotocols::localmsgsubmission::DmqMsg;
use crate::multiplexer;

/// Errors produced by the local-message-notification protocol.
#[derive(Error, Debug)]
pub enum Error {
    /// Tried to receive while we hold agency.
    #[error("attempted to receive message while agency is ours")]
    AgencyIsOurs,

    /// Tried to send while the peer holds agency.
    #[error("attempted to send message while agency is theirs")]
    AgencyIsTheirs,

    /// Inbound message is not valid for the current state.
    #[error("inbound message is not valid for current state")]
    InvalidInbound,

    /// Outbound message is not valid for the current state.
    #[error("outbound message is not valid for current state")]
    InvalidOutbound,

    /// Caller tried to re-initialize an already-running protocol.
    #[error("protocol is already initialized, no need to wait for init message")]
    AlreadyInitialized,

    /// Underlying multiplexer error.
    #[error("error while sending or receiving data through the channel")]
    Plexer(multiplexer::Error),
}

/// Local-message-notification state-machine state.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum State {
    /// Idle: client may request messages or terminate.
    Idle,
    /// Server is collecting messages, blocking until at least one arrives.
    BusyBlocking,
    /// Server is collecting messages, returning whatever is available immediately.
    BusyNonBlocking,
    /// Protocol terminated.
    Done,
}

/// Local-message-notification protocol message.
#[derive(Debug)]
pub enum Message {
    /// Client → server: return queued messages without blocking.
    RequestMessagesNonBlocking,
    /// Server → client: messages plus a flag indicating whether more are available.
    ReplyMessagesNonBlocking(Vec<DmqMsg>, bool),
    /// Client → server: block until at least one message is available.
    RequestMessagesBlocking,
    /// Server → client: the awaited batch of messages.
    ReplyMessagesBlocking(Vec<DmqMsg>),
    /// Client → server: terminate the protocol.
    ClientDone,
}
