use std::{collections::HashMap, fmt::Debug};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionTable<T>
where
    T: Debug + Clone,
{
    pub values: HashMap<u64, T>,
}

pub type NetworkMagic = u64;

pub type VersionNumber = u64;

#[derive(Debug, Clone)]
pub enum Message<D>
where
    D: Debug + Clone,
{
    Propose(VersionTable<D>),
    Accept(VersionNumber, D),
    Refuse(RefuseReason),
    QueryReply(VersionTable<D>),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum DoneState<D>
where
    D: Debug + Clone,
{
    Accepted(VersionNumber, D),
    Rejected(RefuseReason),
    QueryReply(VersionTable<D>),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum State<D>
where
    D: Debug + Clone,
{
    Propose,
    Confirm(VersionTable<D>),
    Done(DoneState<D>),
}

impl<D> Default for State<D>
where
    D: Debug + Clone,
{
    fn default() -> Self {
        State::Propose
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefuseReason {
    VersionMismatch(Vec<VersionNumber>),
    HandshakeDecodeError(VersionNumber, String),
    Refused(VersionNumber, String),
}
