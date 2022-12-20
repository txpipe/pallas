use pallas_codec::Fragment;
use pallas_multiplexer::agents::{Channel, ChannelBuffer, ChannelError};
use std::{cell::Cell, fmt::Debug};
use thiserror::Error;
use tracing::trace;

#[derive(Debug, Error)]
pub enum MachineError {
    #[error("invalid message for state [{0}]: {1}")]
    InvalidMsgForState(String, String),

    #[error("channel error communicating with multiplexer: {0}")]
    ChannelError(ChannelError),

    #[error("downstream error while processing business logic {0}")]
    DownstreamError(Box<dyn std::error::Error>),
}

impl MachineError {
    pub fn channel(err: ChannelError) -> Self {
        Self::ChannelError(err)
    }

    pub fn downstream(err: Box<dyn std::error::Error>) -> Self {
        Self::DownstreamError(err)
    }

    pub fn invalid_msg<A: Agent>(state: &A::State, msg: &A::Message) -> Self {
        Self::InvalidMsgForState(format!("{:?}", state), format!("{:?}", msg))
    }
}

pub type Transition<A> = Result<A, MachineError>;

pub trait Agent: Sized {
    type Message: std::fmt::Debug;
    type State: std::fmt::Debug;

    fn state(&self) -> &Self::State;
    fn is_done(&self) -> bool;
    fn has_agency(&self) -> bool;
    fn build_next(&self) -> Self::Message;
    fn apply_start(self) -> Transition<Self>;
    fn apply_outbound(self, msg: Self::Message) -> Transition<Self>;
    fn apply_inbound(self, msg: Self::Message) -> Transition<Self>;
}

pub struct Runner<A, C>
where
    A: Agent,
    C: Channel,
{
    agent: Cell<Option<A>>,
    buffer: ChannelBuffer<C>,
}

impl<A, C> Runner<A, C>
where
    A: Agent,
    A::Message: Fragment + std::fmt::Debug,
    C: Channel,
{
    pub fn new(agent: A, channel: C) -> Self {
        Self {
            agent: Cell::new(Some(agent)),
            buffer: ChannelBuffer::new(channel),
        }
    }

    pub fn start(&mut self) -> Result<(), MachineError> {
        let prev = self.agent.take().unwrap();
        let next = prev.apply_start()?;
        self.agent.set(Some(next));
        Ok(())
    }

    pub fn run_step(&mut self) -> Result<bool, MachineError> {
        let prev = self.agent.take().unwrap();
        let next = run_agent_step(prev, &mut self.buffer)?;
        let is_done = next.is_done();

        self.agent.set(Some(next));

        Ok(is_done)
    }

    pub fn fulfill(mut self) -> Result<(), MachineError> {
        self.start()?;

        while self.run_step()? {}

        Ok(())
    }
}

pub fn run_agent_step<A, C>(agent: A, channel: &mut ChannelBuffer<C>) -> Transition<A>
where
    A: Agent,
    A::Message: Fragment + std::fmt::Debug,
    C: Channel,
{
    match agent.has_agency() {
        true => {
            let msg = agent.build_next();
            trace!(?msg, "processing outbound msg");

            channel
                .send_msg_chunks(&msg)
                .map_err(MachineError::channel)?;

            agent.apply_outbound(msg)
        }
        false => {
            let msg = channel.recv_full_msg().map_err(MachineError::channel)?;

            trace!(?msg, "processing inbound msg");

            agent.apply_inbound(msg)
        }
    }
}

pub fn run_agent<A, C>(agent: A, buffer: &mut ChannelBuffer<C>) -> Transition<A>
where
    A: Agent,
    A::Message: Fragment + std::fmt::Debug,
    C: Channel,
{
    let mut agent = agent.apply_start()?;

    while !agent.is_done() {
        agent = run_agent_step(agent, buffer)?;
    }

    Ok(agent)
}
