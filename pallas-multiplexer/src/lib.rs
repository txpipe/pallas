pub mod bearers;
pub mod demux;
pub mod mux;
pub mod threads;

use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Receiver},
    Arc,
};

use bearers::Bearer;

pub type Payload = Vec<u8>;

#[derive(Clone, Debug, Default)]
pub struct Cancel(Arc<AtomicBool>);

impl Cancel {
    pub fn set(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    pub fn is_set(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

pub struct Channel(pub mux::ChunkSender, pub Receiver<Payload>);

pub struct Multiplexer<TBearer>
where
    TBearer: Bearer,
{
    pub muxer: mux::Muxer<TBearer>,
    pub demuxer: demux::Demuxer<TBearer>,
}

impl<TBearer> Multiplexer<TBearer>
where
    TBearer: Bearer,
{
    pub fn new(bearer: TBearer) -> Self {
        Multiplexer {
            muxer: mux::Muxer::new(bearer.clone()),
            demuxer: demux::Demuxer::new(bearer.clone()),
        }
    }

    pub fn use_channel(&mut self, protocol_id: u16) -> Channel {
        let (demux_tx, demux_rx) = mpsc::channel::<Payload>();
        let (mux_tx, mux_rx) = mpsc::channel::<Payload>();

        self.muxer.ingress.register(protocol_id, mux_rx);
        self.demuxer.egress.register(protocol_id, demux_tx);

        Channel(mux::ChunkSender::from(mux_tx), demux_rx)
    }
}
