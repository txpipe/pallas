use std::fmt::Debug;

use crate::{Agent, Transition};

use super::protocol::{Message, RefuseReason, State, VersionNumber, VersionTable};

#[derive(Debug)]
pub enum Output<D: Debug + Clone> {
    Pending,
    Accepted(VersionNumber, D),
    Refused(RefuseReason),
    QueryReply(VersionTable<D>),
}

#[derive(Debug)]
pub struct Initiator<D>
where
    D: Debug + Clone,
{
    pub state: State,
    pub output: Output<D>,
    pub version_table: VersionTable<D>,
}

impl<D> Initiator<D>
where
    D: Debug + Clone,
{
    pub fn initial(version_table: VersionTable<D>) -> Self {
        Initiator {
            state: State::Propose,
            output: Output::Pending,
            version_table,
        }
    }
}

impl<D> Agent for Initiator<D>
where
    D: Debug + Clone,
{
    type Message = Message<D>;
    type State = State;

    fn state(&self) -> &Self::State {
        &self.state
    }

    fn is_done(&self) -> bool {
        self.state == State::Done
    }

    fn has_agency(&self) -> bool {
        match self.state {
            State::Propose => true,
            State::Confirm => false,
            State::Done => false,
        }
    }

    fn build_next(&self) -> Self::Message {
        match self.state {
            State::Propose => Message::Propose(self.version_table.clone()),
            _ => panic!("I don't have agency, nothing to send"),
        }
    }

    fn apply_start(self) -> Transition<Self> {
        Ok(self)
    }

    fn apply_outbound(self, msg: Self::Message) -> Transition<Self> {
        match (self.state, msg) {
            (State::Propose, Message::Propose(_)) => Ok(Self {
                state: State::Confirm,
                ..self
            }),
            _ => panic!(""),
        }
    }

    fn apply_inbound(self, msg: Self::Message) -> Transition<Self> {
        match (self.state, msg) {
            (State::Confirm, Message::Accept(version, data)) => Ok(Self {
                state: State::Done,
                output: Output::Accepted(version, data),
                ..self
            }),
            (State::Confirm, Message::Refuse(reason)) => Ok(Self {
                state: State::Done,
                output: Output::Refused(reason),
                ..self
            }),
            (State::Confirm, Message::QueryReply(version_table)) => Ok(Self {
                state: State::Done,
                output: Output::QueryReply(version_table),
                ..self
            }),
            _ => panic!("Current state doesn't expect to receive a message"),
        }
    }
}
