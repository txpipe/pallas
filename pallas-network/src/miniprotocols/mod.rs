//! Implementations for the different Ouroboros mini-protocols

mod common;

pub mod blockfetch;
pub mod chainsync;
pub mod handshake;
pub mod keepalive;
pub mod localmsgnotification;
pub mod localmsgsubmission;
pub mod localstate;
pub mod localtxsubmission;
pub mod peersharing;
pub mod txmonitor;
pub mod txsubmission;

pub use common::*;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("attempted to receive message while agency is ours")]
    AgencyIsOurs,

    #[error("attempted to send message while agency is theirs")]
    AgencyIsTheirs,

    #[error("inbound message is not valid for current state")]
    InvalidInbound,

    #[error("outbound message is not valid for current state")]
    InvalidOutbound,

    #[error("error while sending or receiving data through the channel")]
    Plexer(#[from] crate::multiplexer::Error),
}

pub trait Agent {
    type State;
    type Message: pallas_codec::Fragment;

    fn new(init: Self::State) -> Self;
    fn is_done(&self) -> bool;
    fn has_agency(&self) -> bool;
    fn state(&self) -> &Self::State;
    fn apply(&self, msg: &Self::Message) -> Result<Self::State, Error>;
}

pub struct PlexerAdapter<A: Agent> {
    agent: A,
    channel: crate::multiplexer::ChannelBuffer,
}

impl<A: Agent> PlexerAdapter<A> {
    pub fn new(agent: A, channel: crate::multiplexer::AgentChannel) -> Self {
        Self {
            agent,
            channel: crate::multiplexer::ChannelBuffer::new(channel),
        }
    }

    pub fn state(&self) -> &A::State {
        self.agent.state()
    }

    pub fn is_done(&self) -> bool {
        self.agent.is_done()
    }

    pub fn has_agency(&self) -> bool {
        self.agent.has_agency()
    }

    pub async fn recv(&mut self) -> Result<(), Error> {
        if self.agent.has_agency() {
            return Err(Error::AgencyIsOurs);
        }

        let msg = self.channel.recv_full_msg().await?;

        let new_state = self.agent.apply(&msg)?;

        self.agent = A::new(new_state);

        Ok(())
    }

    pub async fn send(&mut self, msg: &A::Message) -> Result<(), Error> {
        if !self.agent.has_agency() {
            return Err(Error::AgencyIsTheirs);
        }

        let new_state = self.agent.apply(msg)?;

        self.channel.send_msg_chunks(msg).await?;

        self.agent = A::new(new_state);

        Ok(())
    }
}
