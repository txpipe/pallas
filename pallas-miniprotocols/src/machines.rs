use pallas_codec::Fragment;
use pallas_multiplexer::agents::{Channel, ChannelBuffer, ChannelError};
use std::cell::Cell;

#[derive(Debug)]
pub enum MachineError<A: Agent> {
    InvalidMsgForState(A::State, A::Message),
    ChannelError(ChannelError),
    DownstreamError(Box<dyn std::error::Error>),
}

impl<A: Agent> MachineError<A> {
    pub fn channel(err: ChannelError) -> Self {
        Self::ChannelError(err)
    }

    pub fn downstream(err: Box<dyn std::error::Error>) -> Self {
        Self::DownstreamError(err)
    }
}

pub type Transition<A> = Result<A, MachineError<A>>;

pub trait Agent: Sized {
    type Message;
    type State;

    fn state(&self) -> &Self::State;
    fn is_done(&self) -> bool;
    fn has_agency(&self) -> bool;
    fn build_next(&self) -> Self::Message;
    fn apply_start(self) -> Transition<Self>;
    fn apply_outbound(self, msg: Self::Message) -> Transition<Self>;
    fn apply_inbound(self, msg: Self::Message) -> Transition<Self>;
}

pub struct Runner<'c, A, C>
where
    A: Agent,
    C: Channel,
{
    agent: Cell<Option<A>>,
    buffer: ChannelBuffer<'c, C>,
}

impl<'c, A, C> Runner<'c, A, C>
where
    A: Agent,
    A::Message: Fragment + std::fmt::Debug,
    C: Channel,
{
    pub fn new(agent: A, channel: &'c mut C) -> Self {
        Self {
            agent: Cell::new(Some(agent)),
            buffer: ChannelBuffer::new(channel),
        }
    }

    pub fn start(&mut self) -> Result<(), MachineError<A>> {
        let prev = self.agent.take().unwrap();
        let next = prev.apply_start()?;
        self.agent.set(Some(next));
        Ok(())
    }

    pub fn run_step(&mut self) -> Result<bool, MachineError<A>> {
        let prev = self.agent.take().unwrap();
        let next = run_agent_step(prev, &mut self.buffer)?;
        let is_done = next.is_done();

        self.agent.set(Some(next));

        Ok(is_done)
    }

    pub fn fulfill(mut self) -> Result<(), MachineError<A>> {
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
            log::trace!("processing outbound msg: {:?}", msg);

            channel
                .send_msg_chunks(&msg)
                .map_err(MachineError::channel)?;

            agent.apply_outbound(msg)
        }
        false => {
            let msg = channel.recv_full_msg().map_err(MachineError::channel)?;

            log::trace!("procesing inbound msg: {:?}", msg);

            agent.apply_inbound(msg)
        }
    }
}

pub fn run_agent<A, C>(agent: A, channel: &mut C) -> Transition<A>
where
    A: Agent,
    A::Message: Fragment + std::fmt::Debug,
    C: Channel,
{
    let mut buffer = ChannelBuffer::new(channel);

    let mut agent = agent.apply_start()?;

    while !agent.is_done() {
        agent = run_agent_step(agent, &mut buffer)?;
    }

    Ok(agent)
}
