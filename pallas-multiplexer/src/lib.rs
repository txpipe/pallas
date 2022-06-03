pub mod agents;
pub mod bearers;
pub mod demux;
pub mod mux;

pub type Payload = Vec<u8>;

pub struct Multiplexer<B, I, E>
where
    B: bearers::Bearer,
    I: mux::Ingress,
    E: demux::Egress,
{
    pub muxer: mux::Muxer<B, I>,
    pub demuxer: demux::Demuxer<B, E>,
}

impl<B, I, E> Multiplexer<B, I, E>
where
    B: bearers::Bearer,
    I: mux::Ingress,
    E: demux::Egress,
{
    pub fn new(bearer: B) -> Self {
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

#[cfg(feature = "std")]
mod std;

#[cfg(feature = "std")]
pub use crate::std::*;
