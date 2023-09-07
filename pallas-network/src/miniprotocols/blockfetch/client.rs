use thiserror::Error;
use tracing::{debug, info, warn};

use crate::miniprotocols::common::Point;
use crate::multiplexer;

use super::{Message, State};

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("attempted to receive message while agency is ours")]
    AgencyIsOurs,

    #[error("attempted to send message while agency is theirs")]
    AgencyIsTheirs,

    #[error("inbound message is not valid for current state")]
    InvalidInbound,

    #[error("outbound message is not valid for current state")]
    InvalidOutbound,

    #[error("requested range doesn't contain any blocks")]
    NoBlocks,

    #[error("error while sending or receiving data through the multiplexer")]
    Plexer(multiplexer::Error),
}

pub type Body = Vec<u8>;

pub type Range = (Point, Point);

pub type HasBlocks = Option<()>;

/// Represents the client for the BlockFetch mini-protocol.
///
/// This struct is used to interact with the Cardano network and fetch blocks
/// from a remote node. It handles the state transitions and message exchange
/// required to communicate with the network using the BlockFetch mini-protocol.
pub struct Client(State, multiplexer::ChannelBuffer);

impl Client {
    /// Create a new BlockFetch client from a multiplexer agent channel.
    ///
    /// # Arguments
    ///
    /// * `channel` - A multiplexer agent channel used for communication with
    ///   the remote node.
    pub fn new(channel: multiplexer::AgentChannel) -> Self {
        Self(State::Idle, multiplexer::ChannelBuffer::new(channel))
    }

    /// Get the current state of the client.
    ///
    /// Returns the current state of the client.
    pub fn state(&self) -> &State {
        &self.0
    }

    /// Check if the client is done.
    ///
    /// Returns true if the client is in the `Done` state, false otherwise.
    pub fn is_done(&self) -> bool {
        self.0 == State::Done
    }

    fn has_agency(&self) -> bool {
        match self.state() {
            State::Idle => true,
            State::Busy => false,
            State::Streaming => false,
            State::Done => false,
        }
    }

    fn assert_agency_is_ours(&self) -> Result<(), ClientError> {
        if !self.has_agency() {
            Err(ClientError::AgencyIsTheirs)
        } else {
            Ok(())
        }
    }

    fn assert_agency_is_theirs(&self) -> Result<(), ClientError> {
        if self.has_agency() {
            Err(ClientError::AgencyIsOurs)
        } else {
            Ok(())
        }
    }

    fn assert_outbound_state(&self, msg: &Message) -> Result<(), ClientError> {
        match (&self.0, msg) {
            (State::Idle, Message::RequestRange { .. }) => Ok(()),
            (State::Idle, Message::ClientDone) => Ok(()),
            _ => Err(ClientError::InvalidOutbound),
        }
    }

    fn assert_inbound_state(&self, msg: &Message) -> Result<(), ClientError> {
        match (&self.0, msg) {
            (State::Busy, Message::StartBatch) => Ok(()),
            (State::Busy, Message::NoBlocks) => Ok(()),
            (State::Streaming, Message::Block { .. }) => Ok(()),
            (State::Streaming, Message::BatchDone) => Ok(()),
            _ => Err(ClientError::InvalidInbound),
        }
    }

    pub async fn send_message(&mut self, msg: &Message) -> Result<(), ClientError> {
        self.assert_agency_is_ours()?;
        self.assert_outbound_state(msg)?;
        self.1
            .send_msg_chunks(msg)
            .await
            .map_err(ClientError::Plexer)?;

        Ok(())
    }

    pub async fn recv_message(&mut self) -> Result<Message, ClientError> {
        self.assert_agency_is_theirs()?;
        let msg = self.1.recv_full_msg().await.map_err(ClientError::Plexer)?;
        self.assert_inbound_state(&msg)?;

        Ok(msg)
    }

    pub async fn send_request_range(&mut self, range: (Point, Point)) -> Result<(), ClientError> {
        let msg = Message::RequestRange { range };
        self.send_message(&msg).await?;
        self.0 = State::Busy;

        Ok(())
    }

    pub async fn recv_while_busy(&mut self) -> Result<HasBlocks, ClientError> {
        match self.recv_message().await? {
            Message::StartBatch => {
                info!("batch start");
                self.0 = State::Streaming;
                Ok(Some(()))
            }
            Message::NoBlocks => {
                warn!("no blocks");
                self.0 = State::Idle;
                Ok(None)
            }
            _ => Err(ClientError::InvalidInbound),
        }
    }

    /// Send a request for a specific range of blocks to the remote node.
    ///
    /// # Arguments
    ///
    /// * `range` - A tuple of two `Point` instances representing the start and
    ///   end of the requested block range.
    pub async fn request_range(&mut self, range: Range) -> Result<HasBlocks, ClientError> {
        self.send_request_range(range).await?;
        debug!("range requested");
        self.recv_while_busy().await
    }

    /// Receive blocks while the client is in the `Streaming` state.
    ///
    /// Returns a block's body if a block is received, or `None` if the
    /// streaming has ended.
    pub async fn recv_while_streaming(&mut self) -> Result<Option<Body>, ClientError> {
        debug!("waiting for stream");

        match self.recv_message().await? {
            Message::Block { body } => Ok(Some(body)),
            Message::BatchDone => {
                self.0 = State::Idle;
                Ok(None)
            }
            _ => Err(ClientError::InvalidInbound),
        }
    }

    /// Fetch a single block by its `Point`.
    ///
    /// # Arguments
    ///
    /// * `point` - The `Point` of the block to fetch.
    ///
    /// Returns the block's body if the block is found, or an `Error` if the
    /// block is not found or an invalid message is received.
    pub async fn fetch_single(&mut self, point: Point) -> Result<Body, ClientError> {
        self.request_range((point.clone(), point))
            .await?
            .ok_or(ClientError::NoBlocks)?;

        let body = self
            .recv_while_streaming()
            .await?
            .ok_or(ClientError::InvalidInbound)?;

        debug!("body received");

        match self.recv_while_streaming().await? {
            Some(_) => Err(ClientError::InvalidInbound),
            None => Ok(body),
        }
    }

    /// Fetch a range of blocks.
    ///
    /// # Arguments
    ///
    /// * `range` - A tuple of two `Point` instances representing the start and
    ///   end of the requested block range.
    ///
    /// Returns a vector of block bodies for the requested range, or an `Error`
    /// if the range is not found.
    pub async fn fetch_range(&mut self, range: Range) -> Result<Vec<Body>, ClientError> {
        self.request_range(range)
            .await?
            .ok_or(ClientError::NoBlocks)?;

        let mut all = vec![];

        while let Some(block) = self.recv_while_streaming().await? {
            debug!("body received");
            all.push(block);
        }

        Ok(all)
    }

    /// Send a `ClientDone` message to the remote node and set the client's
    /// state to `Done`.
    ///
    /// Returns `Ok(())` if the message is sent successfully, or an `Error` if
    /// the agency is not ours.
    pub async fn send_done(&mut self) -> Result<(), ClientError> {
        let msg = Message::ClientDone;
        self.send_message(&msg).await?;
        self.0 = State::Done;

        Ok(())
    }
}
