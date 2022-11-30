use std::collections::HashMap;

use crate::{bearers::Bearer, Payload};

pub struct EgressError(pub Payload);

pub trait Egress {
    fn send(&mut self, payload: Payload) -> Result<(), EgressError>;
}

pub enum DemuxError {
    BearerError(std::io::Error),
    EgressDisconnected(u16, Payload),
    EgressUnknown(u16, Payload),
}

pub enum TickOutcome {
    Busy,
    Idle,
}

/// A demuxer that reads from a bearer into the corresponding egress
pub struct Demuxer<E> {
    bearer: Bearer,
    egress: HashMap<u16, E>,
}

impl<E> Demuxer<E>
where
    E: Egress,
{
    pub fn new(bearer: Bearer) -> Self {
        Demuxer {
            bearer,
            egress: Default::default(),
        }
    }

    pub fn register(&mut self, id: u16, tx: E) {
        self.egress.insert(id, tx);
    }

    pub fn unregister(&mut self, id: u16) -> Option<E> {
        self.egress.remove(&id)
    }

    fn dispatch(&mut self, protocol: u16, payload: Payload) -> Result<(), DemuxError> {
        match self.egress.get_mut(&protocol) {
            Some(tx) => match tx.send(payload) {
                Err(EgressError(p)) => Err(DemuxError::EgressDisconnected(protocol, p)),
                Ok(_) => Ok(()),
            },
            None => Err(DemuxError::EgressUnknown(protocol, payload)),
        }
    }

    pub fn tick(&mut self) -> Result<TickOutcome, DemuxError> {
        match self.bearer.read_segment() {
            Err(err) => Err(DemuxError::BearerError(err)),
            Ok(None) => Ok(TickOutcome::Idle),
            Ok(Some(segment)) => match self.dispatch(segment.protocol, segment.payload) {
                Err(err) => Err(err),
                Ok(()) => Ok(TickOutcome::Busy),
            },
        }
    }
}
