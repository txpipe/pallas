use std::time::{Duration, Instant};

use crate::{
    bearers::{Bearer, Segment},
    Message,
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
    fn recv_timeout(&mut self, duration: Duration) -> Result<Message, IngressError>;
}

pub enum TickOutcome {
    BearerError(std::io::Error),
    IngressDisconnected,
    Idle,
    Busy,
}

pub struct Muxer<I> {
    bearer: Bearer,
    ingress: I,
    clock: Instant,
}

impl<I> Muxer<I>
where
    I: Ingress,
{
    pub fn new(bearer: Bearer, ingress: I) -> Self {
        Self {
            bearer,
            ingress,
            clock: Instant::now(),
        }
    }

    pub fn tick(&mut self) -> TickOutcome {
        match self.ingress.recv_timeout(Duration::from_millis(1)) {
            Ok((id, payload)) => {
                let segment = Segment::new(self.clock, id, payload);

                match self.bearer.write_segment(segment) {
                    Err(err) => TickOutcome::BearerError(err),
                    _ => TickOutcome::Busy,
                }
            }
            Err(IngressError::Empty) => TickOutcome::Idle,
            Err(IngressError::Disconnected) => TickOutcome::IngressDisconnected,
        }
    }
}
