pub use crate::payloads::*;
use pallas_codec::{minicbor, Fragment};
use pallas_multiplexer::{Channel, Payload};
use std::cell::Cell;
use std::fmt::{Debug, Display};
use std::sync::mpsc::Sender;

#[derive(Debug)]
pub enum MachineError<State, Msg>
where
    State: Debug,
    Msg: Debug,
{
    InvalidMsgForState(State, Msg),
}

impl<S, M> Display for MachineError<S, M>
where
    S: Debug,
    M: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MachineError::InvalidMsgForState(msg, state) => {
                write!(
                    f,
                    "received invalid message ({:?}) for current state ({:?})",
                    msg, state
                )
            }
        }
    }
}

impl<S, M> std::error::Error for MachineError<S, M>
where
    S: Debug,
    M: Debug,
{
}

#[derive(Debug)]
pub enum CodecError {
    BadLabel(u16),
    UnexpectedCbor(&'static str),
}

impl std::error::Error for CodecError {}

impl Display for CodecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CodecError::BadLabel(label) => {
                write!(f, "unknown message label: {}", label)
            }
            CodecError::UnexpectedCbor(msg) => {
                write!(f, "unexpected cbor: {}", msg)
            }
        }
    }
}

pub trait MachineOutput {
    fn send_msg(&self, data: &impl Fragment) -> Result<(), Box<dyn std::error::Error>>;
}

impl MachineOutput for Sender<Payload> {
    fn send_msg(&self, data: &impl Fragment) -> Result<(), Box<dyn std::error::Error>> {
        let mut payload = Vec::new();
        minicbor::encode(data, &mut payload)?;
        self.send(payload)?;

        Ok(())
    }
}

pub type Transition<T> = Result<T, Box<dyn std::error::Error>>;

pub trait Agent: Sized {
    type Message;

    fn is_done(&self) -> bool;
    fn has_agency(&self) -> bool;
    fn build_next(&self) -> Self::Message;
    fn apply_start(self) -> Transition<Self>;
    fn apply_outbound(self, msg: Self::Message) -> Transition<Self>;
    fn apply_inbound(self, msg: Self::Message) -> Transition<Self>;
}

pub struct Runner<A>
where
    A: Agent,
{
    agent: Cell<Option<A>>,
    buffer: Vec<u8>,
}

impl<'a, A> Runner<A>
where
    A: Agent,
    A::Message: Fragment + Debug,
{
    pub fn new(agent: A) -> Self {
        Self {
            agent: Cell::new(Some(agent)),
            buffer: Vec::new(),
        }
    }

    pub fn start(&mut self) -> Result<(), Error> {
        let prev = self.agent.take().unwrap();
        let next = prev.apply_start()?;
        self.agent.set(Some(next));
        Ok(())
    }

    pub fn run_step(&mut self, channel: &mut Channel) -> Result<bool, Error> {
        let prev = self.agent.take().unwrap();
        let next = run_agent_step(prev, channel, &mut self.buffer)?;
        let is_done = next.is_done();

        self.agent.set(Some(next));

        Ok(is_done)
    }

    pub fn fulfill(mut self, channel: &mut Channel) -> Result<(), Error> {
        self.start()?;

        while self.run_step(channel)? {}

        Ok(())
    }
}

pub fn run_agent_step<T>(agent: T, channel: &mut Channel, buffer: &mut Vec<u8>) -> Transition<T>
where
    T: Agent,
    T::Message: Fragment + Debug,
{
    let Channel(tx, rx) = channel;

    match agent.has_agency() {
        true => {
            let msg = agent.build_next();
            log::trace!("processing outbound msg: {:?}", msg);

            let mut payload = Vec::new();
            minicbor::encode(&msg, &mut payload)?;
            tx.send(payload)?;

            agent.apply_outbound(msg)
        }
        false => {
            let msg = read_until_full_msg::<T::Message>(buffer, rx).unwrap();
            log::trace!("procesing inbound msg: {:?}", msg);

            agent.apply_inbound(msg)
        }
    }
}

pub fn run_agent<T>(agent: T, channel: &mut Channel) -> Result<T, Box<dyn std::error::Error>>
where
    T: Agent,
    T::Message: Fragment + Debug,
{
    let mut buffer = Vec::new();

    let mut agent = agent.apply_start()?;

    while !agent.is_done() {
        agent = run_agent_step(agent, channel, &mut buffer)?;
    }

    Ok(agent)
}
