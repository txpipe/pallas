use std::collections::HashMap;

use crate::{bearers::Bearer, std::Cancel, Payload};

pub struct EgressError(pub Payload);

pub trait Egress {
    fn send(&self, payload: Payload) -> Result<(), EgressError>;
}

pub enum DemuxError<B: Bearer> {
    BearerError(B::Error),
    EgressDisconnected(u16, Payload),
    EgressUnknown(u16, Payload),
}

pub enum TickOutcome {
    Busy,
    Idle,
}

/// A demuxer that reads from a bearer into the corresponding egress
pub struct Demuxer<B, E> {
    bearer: B,
    egress: HashMap<u16, E>,
}

impl<B, E> Demuxer<B, E>
where
    B: Bearer,
    E: Egress,
{
    pub fn new(bearer: B) -> Self {
        Demuxer {
            bearer,
            egress: Default::default(),
        }
    }

    pub fn register(&mut self, id: u16, tx: E) {
        self.egress.insert(id, tx);
    }

    fn dispatch(&self, protocol: u16, payload: Payload) -> Result<(), DemuxError<B>> {
        match self.egress.get(&protocol) {
            Some(tx) => match tx.send(payload) {
                Err(EgressError(p)) => Err(DemuxError::EgressDisconnected(protocol, p)),
                Ok(_) => Ok(()),
            },
            None => Err(DemuxError::EgressUnknown(protocol, payload)),
        }
    }

    pub fn tick(&mut self) -> Result<TickOutcome, DemuxError<B>> {
        match self.bearer.read_segment() {
            Err(err) => Err(DemuxError::BearerError(err)),
            Ok(None) => Ok(TickOutcome::Idle),
            Ok(Some(segment)) => match self.dispatch(segment.protocol, segment.payload) {
                Err(err) => Err(err),
                Ok(()) => Ok(TickOutcome::Busy),
            },
        }
    }

    pub fn block(&mut self, cancel: Cancel) -> Result<(), B::Error> {
        loop {
            match self.tick() {
                Ok(TickOutcome::Busy) => (),
                Ok(TickOutcome::Idle) => match cancel.is_set() {
                    true => break Ok(()),
                    false => (),
                },
                Err(DemuxError::BearerError(err)) => return Err(err),
                Err(DemuxError::EgressDisconnected(id, _)) => {
                    log::warn!("disconnected protocol {}", id)
                }
                Err(DemuxError::EgressUnknown(id, _)) => {
                    log::warn!("unknown protocol {}", id)
                }
            }
        }
    }
}
