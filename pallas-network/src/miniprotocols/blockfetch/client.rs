use tracing::{debug, warn};

use crate::miniprotocols::blockfetch::{Body, Range};
use crate::miniprotocols::{common::Point, PlexerAdapter};
use crate::miniprotocols::{Agent, Error};

use super::{ClientState, Message};

pub type HasBlocks = bool;

#[derive(Debug)]
pub struct Client(ClientState);

impl Default for Client {
    fn default() -> Self {
        Self(ClientState::Idle)
    }
}

impl Agent for Client {
    type State = ClientState;
    type Message = Message;

    fn new(init: Self::State) -> Self {
        Self(init)
    }

    fn is_done(&self) -> bool {
        matches!(self.0, Self::State::Done)
    }

    fn has_agency(&self) -> bool {
        match self.state() {
            Self::State::Idle => true,
            Self::State::Busy => false,
            Self::State::Streaming(_) => false,
            Self::State::Done => false,
        }
    }

    fn state(&self) -> &Self::State {
        &self.0
    }

    fn apply(&self, msg: &Self::Message) -> Result<Self::State, Error> {
        match self.state() {
            Self::State::Idle => match msg {
                Message::RequestRange { .. } => Ok(Self::State::Busy),
                Message::ClientDone => Ok(Self::State::Done),
                _ => Err(Error::InvalidOutbound),
            },
            Self::State::Busy => match msg {
                Message::StartBatch => Ok(Self::State::Streaming(None)),
                Message::NoBlocks => Ok(Self::State::Idle),
                _ => Err(Error::InvalidInbound),
            },
            Self::State::Streaming(_) => match msg {
                Message::Block(body) => Ok(Self::State::Streaming(Some(body.clone()))),
                Message::BatchDone => Ok(Self::State::Idle),
                _ => Err(Error::InvalidInbound),
            },
            Self::State::Done => Err(Error::InvalidOutbound),
        }
    }
}

impl PlexerAdapter<Client> {
    /// Send a request for a specific range of blocks to the remote node.
    ///
    /// # Arguments
    ///
    /// * `range` - A tuple of two `Point` instances representing the start and
    ///   end of the requested block range.
    pub async fn request_range(&mut self, range: Range) -> Result<HasBlocks, Error> {
        let req = Message::RequestRange(range);

        self.send(&req).await?;
        debug!("range requested");

        self.recv().await?;

        match self.state() {
            ClientState::Streaming(None) => {
                debug!("batch start");
                Ok(true)
            }
            ClientState::Idle => {
                warn!("no blocks");
                Ok(false)
            }
            _ => Err(Error::InvalidInbound),
        }
    }

    /// Receive blocks while the client is in the `Streaming` state.
    ///
    /// Returns a block's body if a block is received, or `None` if the
    /// streaming has ended.
    pub async fn recv_while_streaming(&mut self) -> Result<Option<Body>, Error> {
        self.recv().await?;

        match self.state() {
            ClientState::Streaming(Some(body)) => {
                debug!("block received");
                Ok(Some(body.clone()))
            }
            ClientState::Idle => {
                warn!("no more blocks");
                Ok(None)
            }
            _ => Err(Error::InvalidInbound),
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
    pub async fn fetch_single(&mut self, point: Point) -> Result<Option<Body>, Error> {
        let has_blocks = self.request_range((point.clone(), point)).await?;

        if !has_blocks {
            return Ok(None);
        }

        self.recv_while_streaming().await
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
    pub async fn fetch_range(&mut self, range: Range) -> Result<Vec<Body>, Error> {
        let has_blocks = self.request_range(range).await?;

        if !has_blocks {
            return Ok(vec![]);
        }

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
    pub async fn send_done(&mut self) -> Result<(), Error> {
        let msg = Message::ClientDone;

        self.send(&msg).await?;

        Ok(())
    }
}
