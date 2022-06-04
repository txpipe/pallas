pub mod agents;
pub mod bearers;
pub mod demux;
pub mod mux;

use bearers::Bearer;

#[cfg(feature = "std")]
mod std;

#[cfg(feature = "std")]
pub use crate::std::*;

pub type Payload = Vec<u8>;

pub struct Multiplexer<I, E>
where
    I: mux::Ingress,
    E: demux::Egress,
{
    pub muxer: mux::Muxer<I>,
    pub demuxer: demux::Demuxer<E>,
}

impl<I, E> Multiplexer<I, E>
where
    I: mux::Ingress,
    E: demux::Egress,
{
    pub fn new(bearer: Bearer) -> Self {
        Multiplexer {
            muxer: mux::Muxer::new(bearer.clone()),
            demuxer: demux::Demuxer::new(bearer.clone()),
        }
    }

    pub fn register_channel(&mut self, protocol: u16, ingress: I, egress: E) {
        self.muxer.register(protocol, ingress);
        self.demuxer.register(protocol, egress);
    }
}
