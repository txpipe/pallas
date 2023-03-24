use pallas_crypto::hash::Hash;
use pallas_miniprotocols::Point;
use pallas_multiplexer as multiplexer;
use thiserror::Error;
use tracing::error;

pub type BlockSlot = u64;
pub type BlockHash = Hash<32>;
pub type RawBlock = Vec<u8>;

#[derive(Debug, Clone)]
pub enum ChainSyncEvent {
    RollForward(BlockSlot, BlockHash),
    Rollback(Point),
}

#[derive(Debug, Clone)]
pub enum BlockFetchEvent {
    RollForward(BlockSlot, BlockHash, RawBlock),
    Rollback(Point),
}

// ports used by plexer
pub type MuxOutputPort = gasket::messaging::crossbeam::OutputPort<(u16, multiplexer::Payload)>;
pub type DemuxInputPort = gasket::messaging::crossbeam::InputPort<multiplexer::Payload>;

// ports used by mini-protocols
pub type MuxInputPort = gasket::messaging::crossbeam::InputPort<(u16, multiplexer::Payload)>;
pub type DemuxOutputPort = gasket::messaging::crossbeam::OutputPort<multiplexer::Payload>;

// final output port
pub type DownstreamPort<A> = gasket::messaging::OutputPort<A, BlockFetchEvent>;

pub struct ProtocolChannel(pub u16, pub MuxOutputPort, pub DemuxInputPort);

impl multiplexer::agents::Channel for ProtocolChannel {
    fn enqueue_chunk(
        &mut self,
        payload: multiplexer::Payload,
    ) -> Result<(), multiplexer::agents::ChannelError> {
        match self
            .1
            .send(gasket::messaging::Message::from((self.0, payload)))
        {
            Ok(_) => Ok(()),
            Err(error) => {
                error!(?error, "enqueue chunk failed");
                Err(multiplexer::agents::ChannelError::NotConnected(None))
            }
        }
    }

    fn dequeue_chunk(&mut self) -> Result<multiplexer::Payload, multiplexer::agents::ChannelError> {
        match self.2.recv() {
            Ok(msg) => Ok(msg.payload),
            Err(error) => {
                error!(?error, "dequeue chunk failed");
                Err(multiplexer::agents::ChannelError::NotConnected(None))
            }
        }
    }
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("{0}")]
    Client(String),

    #[error("{0}")]
    Parse(String),

    #[error("{0}")]
    Server(String),

    #[error("{0}")]
    Message(String),

    #[error("{0}")]
    Custom(String),
}

impl Error {
    pub fn client(error: impl ToString) -> Error {
        Error::Client(error.to_string())
    }

    pub fn parse(error: impl ToString) -> Error {
        Error::Parse(error.to_string())
    }

    pub fn server(error: impl ToString) -> Error {
        Error::Server(error.to_string())
    }

    pub fn message(error: impl ToString) -> Error {
        Error::Message(error.to_string())
    }

    pub fn custom(error: Box<dyn std::error::Error>) -> Error {
        Error::Custom(format!("{error}"))
    }
}

impl From<Box<dyn std::error::Error>> for Error {
    fn from(err: Box<dyn std::error::Error>) -> Self {
        Error::custom(err)
    }
}
