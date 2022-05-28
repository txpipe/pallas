use std::{collections::HashMap, sync::mpsc::Sender};

use crate::{Bearer, Payload};

/// An unified view of all demux egress channels
///
/// The inner structure is a hashmap where each entry represent a channel. Used
/// as an abstraction to dispatch segements to the correspoding channel by id.
#[derive(Default)]
pub struct Egress(HashMap<u16, Sender<Payload>>);

pub enum Dispatch {
    Disconnected(u16, Payload),
    Unknown(u16, Payload),
    Done,
}

impl Egress {
    pub fn register(&mut self, id: u16, tx: Sender<Payload>) {
        self.0.insert(id, tx);
    }

    pub fn dispatch(&self, protocol: u16, payload: Vec<u8>) -> Dispatch {
        match self.0.get(&protocol) {
            Some(tx) => match tx.send(payload) {
                Err(err) => Dispatch::Disconnected(protocol, err.0),
                Ok(_) => Dispatch::Done,
            },
            None => Dispatch::Unknown(protocol, payload),
        }
    }
}

pub enum TickOutcome<TBearer>
where
    TBearer: Bearer,
{
    BearerError(TBearer::Error),
    Disconnected(u16, Payload),
    Unknown(u16, Payload),
    Busy,
    Idle,
}

pub struct Demuxer<TBearer> {
    bearer: TBearer,
    pub egress: Egress,
}

impl<TBearer> Demuxer<TBearer>
where
    TBearer: Bearer,
{
    pub fn new(bearer: TBearer) -> Self {
        Demuxer {
            bearer,
            egress: Egress::default(),
        }
    }

    pub fn tick(&mut self) -> TickOutcome<TBearer> {
        match self.bearer.read_segment() {
            Err(err) => TickOutcome::BearerError(err),
            Ok(segment) => match self.egress.dispatch(segment.protocol, segment.payload) {
                Dispatch::Disconnected(id, payload) => TickOutcome::Disconnected(id, payload),
                Dispatch::Unknown(id, payload) => TickOutcome::Unknown(id, payload),
                Dispatch::Done => TickOutcome::Busy,
            },
        }
    }

    pub fn block(&mut self) -> Result<(), TBearer::Error> {
        loop {
            match self.tick() {
                TickOutcome::Busy => (),
                TickOutcome::Idle => (),
                TickOutcome::BearerError(err) => return Err(err),
                TickOutcome::Disconnected(id, _) => {
                    log::warn!("disconnected protocol {}", id)
                }
                TickOutcome::Unknown(id, _) => {
                    log::warn!("unknown protocol {}", id)
                }
            }
        }
    }
}
