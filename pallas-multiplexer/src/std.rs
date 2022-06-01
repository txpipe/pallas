use crate::{bearers::Bearer, demux, mux, Payload};

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{channel, Receiver, SendError, Sender, TryRecvError},
        Arc,
    },
    thread::{spawn, JoinHandle},
};

pub type Ingress = Receiver<Payload>;

impl mux::Ingress for Ingress {
    fn try_recv(&mut self) -> Result<Payload, mux::IngressError> {
        match Receiver::try_recv(&self) {
            Ok(x) => Ok(x),
            Err(TryRecvError::Disconnected) => Err(mux::IngressError::Disconnected),
            Err(TryRecvError::Empty) => Err(mux::IngressError::Empty),
        }
    }
}

pub type Egress = Sender<Payload>;

impl demux::Egress for Egress {
    fn send(&self, payload: Payload) -> Result<(), demux::EgressError> {
        match Sender::send(&self, payload) {
            Ok(_) => Ok(()),
            Err(SendError(p)) => Err(demux::EgressError(p)),
        }
    }
}

pub type Multiplexer<B> = crate::Multiplexer<B, Ingress, Egress>;

pub type Channel = (Sender<Payload>, Receiver<Payload>);

pub fn use_channel<B: Bearer>(plexer: &mut Multiplexer<B>, protocol: u16) -> Channel {
    let (demux_tx, demux_rx) = channel::<Payload>();
    let (mux_tx, mux_rx) = channel::<Payload>();

    plexer.register_channel(protocol, mux_rx, demux_tx);

    (mux_tx, demux_rx)
}

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

#[derive(Debug)]
pub struct Loop<B>
where
    B: Bearer,
{
    cancel: Cancel,
    thread: JoinHandle<Result<(), B::Error>>,
}

impl<B> Loop<B>
where
    B: Bearer,
{
    pub fn cancel(&self) {
        self.cancel.set();
    }

    pub fn join(self) -> Result<(), B::Error> {
        self.thread.join().unwrap()
    }
}

pub fn spawn_muxer<B, I>(mut muxer: mux::Muxer<B, I>) -> Loop<B>
where
    B: Bearer + 'static,
    B::Error: Send,
    I: mux::Ingress + Send + 'static,
{
    let cancel = Cancel::default();
    let cancel2 = cancel.clone();
    let thread = spawn(move || muxer.block(cancel2));

    Loop { cancel, thread }
}

pub fn spawn_demuxer<B, E>(mut demuxer: demux::Demuxer<B, E>) -> Loop<B>
where
    B: Bearer + 'static,
    B::Error: Send,
    E: demux::Egress + Send + 'static,
{
    let cancel = Cancel::default();
    let cancel2 = cancel.clone();
    let thread = spawn(move || demuxer.block(cancel2));

    Loop { cancel, thread }
}
