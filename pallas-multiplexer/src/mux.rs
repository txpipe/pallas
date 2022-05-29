use std::{
    collections::HashMap,
    sync::mpsc::{Receiver, TryRecvError},
    time::Instant,
};

use crate::{
    bearers::{Bearer, Segment, MAX_SEGMENT_PAYLOAD_LENGTH},
    Cancel, Payload,
};

/// An unified view of all mux ingress channels
///
/// To be used as an abstration over the set of all ingress channels. Its main
/// pourpose is to select the next segment to be muxed from the list of
/// available channels.
#[derive(Default)]
pub struct Ingress(HashMap<u16, Receiver<Payload>>);

pub enum Selection {
    Message(u16, Payload),
    Empty,
    Disconnected(u16),
}

use rand::seq::SliceRandom;
use rand::thread_rng;

impl Ingress {
    /// Register the receiver end of an ingress channel
    pub fn register(&mut self, id: u16, rx: Receiver<Payload>) {
        self.0.insert(id, rx);
    }

    /// Remove a protocol from the ingress
    ///
    /// Meant to be used after a receive error in a previous tick
    pub fn deregister(&mut self, id: u16) {
        self.0.remove(&id);
    }

    #[inline]
    fn randomize_ids(&self) -> Vec<u16> {
        let mut rng = thread_rng();
        let mut keys: Vec<_> = self.0.keys().cloned().collect();
        keys.shuffle(&mut rng);
        keys
    }

    /// Select the next segment to be muxed
    ///
    /// This method iterates over the existing receivers checking for the first
    /// available message. The order of the checks is random to ensure a fair
    /// use of the multiplexer amongst all protocols.
    pub fn select(&mut self) -> Selection {
        for id in self.randomize_ids() {
            let rx = self.0.get_mut(&id).unwrap();

            match rx.try_recv() {
                Ok(payload) => return Selection::Message(id, payload),
                Err(TryRecvError::Disconnected) => return Selection::Disconnected(id),
                _ => (),
            };
        }

        Selection::Empty
    }
}

/// Custom sender that chunks the payload
///
/// Ouroboros has a max payload segment constraint. To hide the complexity from
/// the implementation of each mini-protocol, this sender takes a payload of
/// arbitrary size and submits individual chunks of the required size.
pub struct ChunkSender(std::sync::mpsc::Sender<Payload>);

impl From<std::sync::mpsc::Sender<Payload>> for ChunkSender {
    fn from(inner: std::sync::mpsc::Sender<Payload>) -> Self {
        ChunkSender(inner)
    }
}

impl ChunkSender {
    pub fn send_payload(
        &self,
        payload: Payload,
    ) -> Result<(), std::sync::mpsc::SendError<Payload>> {
        let chunks = payload.chunks(MAX_SEGMENT_PAYLOAD_LENGTH);

        for chunk in chunks {
            self.0.send(Vec::from(chunk))?;
        }

        Ok(())
    }
}

pub enum TickOutcome<TBearer>
where
    TBearer: Bearer,
{
    BearerError(TBearer::Error),
    Disconnected(u16),
    Empty,
    Busy,
}

pub struct Muxer<TBearer> {
    bearer: TBearer,
    pub ingress: Ingress,
    clock: Instant,
}

impl<TBearer> Muxer<TBearer>
where
    TBearer: Bearer,
{
    pub fn new(bearer: TBearer) -> Self {
        Self {
            bearer,
            ingress: Ingress::default(),
            clock: Instant::now(),
        }
    }

    pub fn tick(&mut self) -> TickOutcome<TBearer> {
        match self.ingress.select() {
            Selection::Message(id, payload) => {
                let segment = Segment::new(self.clock, id, payload);

                match self.bearer.write_segment(segment) {
                    Err(err) => TickOutcome::BearerError(err),
                    _ => TickOutcome::Busy,
                }
            }
            Selection::Empty => TickOutcome::Empty,
            Selection::Disconnected(id) => TickOutcome::Disconnected(id),
        }
    }

    pub fn block(&mut self, cancel: Cancel) -> Result<(), TBearer::Error> {
        loop {
            match self.tick() {
                TickOutcome::BearerError(err) => return Err(err),
                TickOutcome::Disconnected(id) => self.ingress.deregister(id),
                TickOutcome::Empty => match cancel.is_set() {
                    true => break Ok(()),
                    false => std::thread::yield_now(),
                },
                TickOutcome::Busy => (),
            }
        }
    }
}
