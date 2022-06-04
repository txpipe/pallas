use std::{collections::HashMap, time::Instant};

use rand::seq::SliceRandom;
use rand::thread_rng;

use crate::{
    bearers::{Bearer, Segment},
    Payload,
};

pub enum IngressError {
    Disconnected,
    Empty,
}

/// Source of payloads for a particular protocol
///
/// To be implemented by any mechanism that allows to submit a payloads from a
/// particular protocol that need to be muxed by the multiplexer.
pub trait Ingress {
    fn try_recv(&mut self) -> Result<Payload, IngressError>;
}

type Message = (u16, Payload);

pub enum TickOutcome {
    BearerError(std::io::Error),
    Idle,
    Busy,
}

pub struct Muxer<I> {
    bearer: Bearer,
    ingress: HashMap<u16, I>,
    clock: Instant,
}

impl<I> Muxer<I>
where
    I: Ingress,
{
    pub fn new(bearer: Bearer) -> Self {
        Self {
            bearer,
            ingress: Default::default(),
            clock: Instant::now(),
        }
    }

    /// Register the receiver end of an ingress channel
    pub fn register(&mut self, id: u16, rx: I) {
        self.ingress.insert(id, rx);
    }

    /// Remove a protocol from the ingress
    ///
    /// Meant to be used after a receive error in a previous tick
    pub fn deregister(&mut self, id: u16) {
        self.ingress.remove(&id);
    }

    #[inline]
    fn randomize_ids(&self) -> Vec<u16> {
        let mut rng = thread_rng();
        let mut keys: Vec<_> = self.ingress.keys().cloned().collect();
        keys.shuffle(&mut rng);
        keys
    }

    /// Select the next segment to be muxed
    ///
    /// This method iterates over the existing receivers checking for the first
    /// available message. The order of the checks is random to ensure a fair
    /// use of the multiplexer amongst all protocols.
    pub fn select(&mut self) -> Option<Message> {
        for id in self.randomize_ids() {
            let rx = self.ingress.get_mut(&id).unwrap();

            match rx.try_recv() {
                Ok(payload) => return Some((id, payload)),
                Err(IngressError::Disconnected) => {
                    self.deregister(id);
                }
                _ => (),
            };
        }

        None
    }

    pub fn tick(&mut self) -> TickOutcome {
        match self.select() {
            Some((id, payload)) => {
                let segment = Segment::new(self.clock, id, payload);

                match self.bearer.write_segment(segment) {
                    Err(err) => TickOutcome::BearerError(err),
                    _ => TickOutcome::Busy,
                }
            }
            None => TickOutcome::Idle,
        }
    }
}
