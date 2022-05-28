mod bearers;
mod demux;
mod mux;

use std::{
    sync::mpsc::{self, Receiver},
    thread::{self, JoinHandle},
};

use bearers::Bearer;

pub type Payload = Vec<u8>;

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

pub fn spawn_muxer<TBearer>(
    mut muxer: mux::Muxer<TBearer>,
) -> JoinHandle<Result<(), TBearer::Error>>
where
    TBearer: Bearer + 'static,
    TBearer::Error: Send,
{
    thread::spawn(move || muxer.block())
}

pub fn spawn_demuxer<TBearer>(
    mut demuxer: demux::Demuxer<TBearer>,
) -> JoinHandle<Result<(), TBearer::Error>>
where
    TBearer: Bearer + 'static,
    TBearer::Error: Send,
{
    thread::spawn(move || demuxer.block())
}
