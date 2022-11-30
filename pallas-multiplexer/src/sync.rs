use crate::{
    agents::{self, ChannelBuffer},
    bearers::{Bearer, Segment},
    Payload,
};

use std::time::Instant;

pub struct SyncPlexer {
    bearer: Bearer,
    protocol: u16,
    clock: Instant,
}

impl SyncPlexer {
    pub fn new(bearer: Bearer, protocol: u16) -> Self {
        Self {
            bearer,
            protocol,
            clock: Instant::now(),
        }
    }

    pub fn unwrap(self) -> Bearer {
        self.bearer
    }
}

pub type SyncChannel = ChannelBuffer<SyncPlexer>;

impl agents::Channel for SyncPlexer {
    fn enqueue_chunk(&mut self, payload: Payload) -> Result<(), agents::ChannelError> {
        let segment = Segment::new(self.clock, self.protocol, payload);

        self.bearer
            .write_segment(segment)
            .map_err(|_| agents::ChannelError::NotConnected(None))
    }

    fn dequeue_chunk(&mut self) -> Result<Payload, agents::ChannelError> {
        match self.bearer.read_segment() {
            Ok(segment) => match segment {
                Some(x) => {
                    assert_eq!(
                        x.protocol, self.protocol,
                        "sync plexer received payload for wrong protocol"
                    );
                    Ok(x.payload)
                }
                None => Err(agents::ChannelError::NotConnected(None)),
            },
            Err(_) => Err(agents::ChannelError::NotConnected(None)),
        }
    }
}
