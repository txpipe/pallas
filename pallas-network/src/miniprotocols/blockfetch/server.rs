use crate::miniprotocols::{blockfetch::Body, Agent, Error, PlexerAdapter};

use super::{Message, Range, ServerState};

/// Represents the server for the BlockFetch mini-protocol.
pub struct Server(ServerState);

impl Default for Server {
    fn default() -> Self {
        Self(ServerState::Idle)
    }
}

impl Agent for Server {
    type State = ServerState;
    type Message = Message;

    fn new(init: Self::State) -> Self {
        Self(init)
    }

    fn is_done(&self) -> bool {
        matches!(self.0, Self::State::Done)
    }

    fn state(&self) -> &Self::State {
        &self.0
    }

    fn has_agency(&self) -> bool {
        match self.state() {
            Self::State::Idle => false,
            Self::State::Busy(_) => true,
            Self::State::Streaming => true,
            Self::State::Done => false,
        }
    }

    fn apply(&self, msg: &Self::Message) -> Result<Self::State, Error> {
        match self.state() {
            Self::State::Idle => match msg {
                Message::RequestRange(range) => Ok(Self::State::Busy(range.clone())),
                Message::ClientDone => Ok(Self::State::Done),
                _ => Err(Error::InvalidOutbound),
            },
            Self::State::Busy(_) => match msg {
                Message::NoBlocks => Ok(Self::State::Idle),
                Message::StartBatch => Ok(Self::State::Streaming),
                _ => Err(Error::InvalidInbound),
            },
            Self::State::Streaming => match msg {
                Message::Block(body) => Ok(Self::State::Streaming),
                Message::BatchDone => Ok(Self::State::Idle),
                _ => Err(Error::InvalidInbound),
            },
            Self::State::Done => Err(Error::InvalidOutbound),
        }
    }
}

impl PlexerAdapter<Server> {
    pub async fn reply_start_batch(&mut self) -> Result<(), Error> {
        let msg = Message::StartBatch;

        self.send(&msg).await?;

        Ok(())
    }

    pub async fn reply_no_blocks(&mut self) -> Result<(), Error> {
        let msg = Message::NoBlocks;

        self.send(&msg).await?;

        Ok(())
    }

    pub async fn reply_block(&mut self, body: Body) -> Result<(), Error> {
        let msg = Message::Block(body);

        self.send(&msg).await?;

        Ok(())
    }

    pub async fn reply_batch_done(&mut self) -> Result<(), Error> {
        let msg = Message::BatchDone;

        self.send(&msg).await?;

        Ok(())
    }

    pub async fn recv_while_idle(&mut self) -> Result<Option<Range>, Error> {
        self.recv().await?;

        match self.state() {
            ServerState::Idle => todo!(),
            ServerState::Busy(_) => todo!(),
            ServerState::Streaming => todo!(),
            ServerState::Done => todo!(),
            _ => Err(Error::InvalidInbound),
        }
    }

    pub async fn reply_block_range(&mut self, blocks: Vec<Body>) -> Result<(), Error> {
        if blocks.is_empty() {
            self.reply_no_blocks().await
        } else {
            self.reply_start_batch().await?;

            for block in blocks {
                self.reply_block(block).await?;
            }

            self.reply_batch_done().await
        }
    }
}
