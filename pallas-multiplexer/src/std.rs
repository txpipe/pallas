use crate::{
    agents::{self, ChannelBuffer},
    demux, mux, Payload,
};

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{channel, Receiver, SendError, Sender, TryRecvError},
        Arc,
    },
    thread::{spawn, JoinHandle},
};

pub type StdIngress = Receiver<Payload>;

impl mux::Ingress for StdIngress {
    fn try_recv(&mut self) -> Result<Payload, mux::IngressError> {
        match Receiver::try_recv(self) {
            Ok(x) => Ok(x),
            Err(TryRecvError::Disconnected) => Err(mux::IngressError::Disconnected),
            Err(TryRecvError::Empty) => Err(mux::IngressError::Empty),
        }
    }
}

pub type StdEgress = Sender<Payload>;

impl demux::Egress for StdEgress {
    fn send(&self, payload: Payload) -> Result<(), demux::EgressError> {
        match Sender::send(self, payload) {
            Ok(_) => Ok(()),
            Err(SendError(p)) => Err(demux::EgressError(p)),
        }
    }
}

pub type StdPlexer = crate::Multiplexer<StdIngress, StdEgress>;

impl StdPlexer {
    pub fn use_channel(&mut self, protocol: u16) -> StdChannel {
        let (demux_tx, demux_rx) = channel::<Payload>();
        let (mux_tx, mux_rx) = channel::<Payload>();

        self.register_channel(protocol, mux_rx, demux_tx);

        (mux_tx, demux_rx)
    }
}

impl mux::Muxer<StdIngress> {
    pub fn block(&mut self, cancel: Cancel) -> Result<(), std::io::Error> {
        let backoff = crossbeam::utils::Backoff::new();

        loop {
            match self.tick() {
                mux::TickOutcome::BearerError(err) => return Err(err),
                mux::TickOutcome::Idle => match cancel.is_set() {
                    true => break Ok(()),
                    false => backoff.snooze(),
                },
                mux::TickOutcome::Busy => (),
            }
        }
    }

    pub fn spawn(mut self) -> Loop {
        let cancel = Cancel::default();
        let cancel2 = cancel.clone();
        let thread = spawn(move || self.block(cancel2));

        Loop { cancel, thread }
    }
}

impl demux::Demuxer<StdEgress> {
    pub fn block(&mut self, cancel: Cancel) -> Result<(), std::io::Error> {
        loop {
            match self.tick() {
                Ok(demux::TickOutcome::Busy) => (),
                Ok(demux::TickOutcome::Idle) => match cancel.is_set() {
                    true => break Ok(()),
                    false => (),
                },
                Err(demux::DemuxError::BearerError(err)) => return Err(err),
                Err(demux::DemuxError::EgressDisconnected(id, _)) => {
                    log::warn!("disconnected protocol {}", id)
                }
                Err(demux::DemuxError::EgressUnknown(id, _)) => {
                    log::warn!("unknown protocol {}", id)
                }
            }
        }
    }

    pub fn spawn(mut self) -> Loop {
        let cancel = Cancel::default();
        let cancel2 = cancel.clone();
        let thread = spawn(move || self.block(cancel2));

        Loop { cancel, thread }
    }
}

pub type StdChannel = (Sender<Payload>, Receiver<Payload>);

pub type StdChannelBuffer = ChannelBuffer<StdChannel>;

impl agents::Channel for StdChannel {
    fn enqueue_chunk(&mut self, payload: Payload) -> Result<(), agents::ChannelError> {
        match self.0.send(payload) {
            Ok(_) => Ok(()),
            Err(SendError(payload)) => Err(agents::ChannelError::NotConnected(Some(payload))),
        }
    }

    fn dequeue_chunk(&mut self) -> Result<Payload, agents::ChannelError> {
        match self.1.recv() {
            Ok(payload) => Ok(payload),
            Err(_) => Err(agents::ChannelError::NotConnected(None)),
        }
    }
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
pub struct Loop {
    cancel: Cancel,
    thread: JoinHandle<Result<(), std::io::Error>>,
}

impl Loop {
    pub fn cancel(&self) {
        self.cancel.set();
    }

    pub fn join(self) -> Result<(), std::io::Error> {
        self.thread.join().unwrap()
    }
}
