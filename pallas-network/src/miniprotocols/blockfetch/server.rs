use thiserror::Error;

use crate::multiplexer;

use super::{Body, Message, Range, State};

/// Errors produced by the block-fetch server agent.
#[derive(Error, Debug)]
pub enum ServerError {
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

    /// Underlying multiplexer error.
    #[error("error while sending or receiving data through the multiplexer")]
    Plexer(multiplexer::Error),
}

/// Block range a client just asked the server to serve.
#[derive(Debug)]
pub struct BlockRequest(pub Range);

/// Represents the server for the BlockFetch mini-protocol.
pub struct Server(State, multiplexer::ChannelBuffer);

impl Server {
    /// Create a new BlockFetch server from a multiplexer agent channel.
    ///
    /// # Arguments
    ///
    /// * `channel` - A multiplexer agent channel used for communication with
    ///   the server.
    pub fn new(channel: multiplexer::AgentChannel) -> Self {
        Self(State::Idle, multiplexer::ChannelBuffer::new(channel))
    }

    /// Get the current state of the server.
    ///
    /// Returns the current state of the server.
    pub fn state(&self) -> &State {
        &self.0
    }

    /// Check if the server is done.
    ///
    /// Returns true if server is in the `Done` state, false otherwise.
    pub fn is_done(&self) -> bool {
        self.0 == State::Done
    }

    fn has_agency(&self) -> bool {
        match self.state() {
            State::Idle => false,
            State::Busy => true,
            State::Streaming => true,
            State::Done => false,
        }
    }

    fn assert_agency_is_ours(&self) -> Result<(), ServerError> {
        if !self.has_agency() {
            Err(ServerError::AgencyIsTheirs)
        } else {
            Ok(())
        }
    }

    fn assert_agency_is_theirs(&self) -> Result<(), ServerError> {
        if self.has_agency() {
            Err(ServerError::AgencyIsOurs)
        } else {
            Ok(())
        }
    }

    fn assert_outbound_state(&self, msg: &Message) -> Result<(), ServerError> {
        match (&self.0, msg) {
            (State::Busy, Message::NoBlocks) => Ok(()),
            (State::Busy, Message::StartBatch) => Ok(()),
            (State::Streaming, Message::Block { .. }) => Ok(()),
            (State::Streaming, Message::BatchDone) => Ok(()),
            _ => Err(ServerError::InvalidOutbound),
        }
    }

    fn assert_inbound_state(&self, msg: &Message) -> Result<(), ServerError> {
        match (&self.0, msg) {
            (State::Idle, Message::RequestRange { .. }) => Ok(()),
            (State::Idle, Message::ClientDone) => Ok(()),
            _ => Err(ServerError::InvalidInbound),
        }
    }

    /// Low-level send.
    pub async fn send_message(&mut self, msg: &Message) -> Result<(), ServerError> {
        self.assert_agency_is_ours()?;
        self.assert_outbound_state(msg)?;
        self.1
            .send_msg_chunks(msg)
            .await
            .map_err(ServerError::Plexer)?;

        Ok(())
    }

    /// Low-level receive.
    pub async fn recv_message(&mut self) -> Result<Message, ServerError> {
        self.assert_agency_is_theirs()?;
        let msg = self.1.recv_full_msg().await.map_err(ServerError::Plexer)?;
        self.assert_inbound_state(&msg)?;

        Ok(msg)
    }

    /// Tell the client a batch is starting; transition to `Streaming`.
    pub async fn send_start_batch(&mut self) -> Result<(), ServerError> {
        let msg = Message::StartBatch;
        self.send_message(&msg).await?;
        self.0 = State::Streaming;

        Ok(())
    }

    /// Tell the client no blocks are available for the requested range.
    pub async fn send_no_blocks(&mut self) -> Result<(), ServerError> {
        let msg = Message::NoBlocks;
        self.send_message(&msg).await?;
        self.0 = State::Idle;

        Ok(())
    }

    /// Stream a single block body to the client.
    pub async fn send_block(&mut self, body: Body) -> Result<(), ServerError> {
        let msg = Message::Block { body };
        self.send_message(&msg).await?;

        Ok(())
    }

    /// Signal end-of-batch and transition back to `Idle`.
    pub async fn send_batch_done(&mut self) -> Result<(), ServerError> {
        let msg = Message::BatchDone;
        self.send_message(&msg).await?;
        self.0 = State::Idle;

        Ok(())
    }

    /// Receive a message from the client while the miniprotocol is in the
    /// `Idle` state.
    ///
    /// If the message is a `RequestRange`, return the requested range and
    /// progess the server state to `Busy`. If the message is a `ClientDone`,
    /// return None and progress the server state to `Done`. For any other
    /// incoming message type return an `Error`.
    pub async fn recv_while_idle(&mut self) -> Result<Option<BlockRequest>, ServerError> {
        match self.recv_message().await? {
            Message::RequestRange { range } => {
                self.0 = State::Busy;

                Ok(Some(BlockRequest(range)))
            }
            Message::ClientDone => {
                self.0 = State::Done;

                Ok(None)
            }
            _ => Err(ServerError::InvalidInbound),
        }
    }

    /// Return a range of blocks to the client, starting in the `Busy` state and
    /// progressing the state machine as required to send all the blocks to the
    /// client.
    ///
    /// # Arguments
    ///
    /// * `blocks` - Ordered list of block bodies corresponding to the client's
    ///   requested range.
    pub async fn send_block_range(&mut self, blocks: Vec<Body>) -> Result<(), ServerError> {
        if blocks.is_empty() {
            self.send_no_blocks().await
        } else {
            self.send_start_batch().await?;

            for block in blocks {
                self.send_block(block).await?;
            }

            self.send_batch_done().await
        }
    }
}
