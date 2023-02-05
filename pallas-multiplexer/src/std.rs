use crate::{
    agents::{self, ChannelBuffer},
    bearers::Bearer,
    demux, mux, Message, Payload,
};

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{channel, Receiver, RecvTimeoutError, SendError, Sender},
        Arc,
    },
    thread::{spawn, JoinHandle},
    time::Duration,
};

pub type StdIngress = Receiver<Message>;

impl mux::Ingress for StdIngress {
    fn recv_timeout(&mut self, duration: Duration) -> Result<Message, mux::IngressError> {
        match Receiver::recv_timeout(self, duration) {
            Ok(x) => Ok(x),
            Err(RecvTimeoutError::Disconnected) => Err(mux::IngressError::Disconnected),
            Err(RecvTimeoutError::Timeout) => Err(mux::IngressError::Empty),
        }
    }
}

pub type StdEgress = Sender<Payload>;

impl demux::Egress for StdEgress {
    fn send(&mut self, payload: Payload) -> Result<(), demux::EgressError> {
        match Sender::send(self, payload) {
            Ok(_) => Ok(()),
            Err(SendError(p)) => Err(demux::EgressError(p)),
        }
    }
}

pub struct StdPlexer {
    pub muxer: mux::Muxer<StdIngress>,
    pub demuxer: demux::Demuxer<StdEgress>,
    pub mux_tx: Sender<Message>,
}

const PROTOCOL_SERVER_BIT: u16 = 0x8000;

impl StdPlexer {
    pub fn new(bearer: Bearer) -> Self {
        let (mux_tx, mux_rx) = channel::<Message>();

        Self {
            muxer: mux::Muxer::new(bearer.clone(), mux_rx),
            demuxer: demux::Demuxer::new(bearer),
            mux_tx,
        }
    }

    pub fn use_channel(&mut self, protocol: u16) -> StdChannel {
        let (demux_tx, demux_rx) = channel::<Payload>();
        self.demuxer.register(protocol, demux_tx);

        let mux_tx = self.mux_tx.clone();

        (protocol, mux_tx, demux_rx)
    }

    /// Use the client-side channel for a given protocol
    /// Explicitly unsets the most significant bit, forcing use of the client
    /// side channel
    pub fn use_client_channel(&mut self, protocol: u16) -> StdChannel {
        self.use_channel(protocol & !PROTOCOL_SERVER_BIT)
    }

    /// Use the server-side channel for a given protocol
    /// Explicitly sets the most significant bit, forcing use of the server side
    /// channel
    pub fn use_server_channel(&mut self, protocol: u16) -> StdChannel {
        self.use_channel(protocol | PROTOCOL_SERVER_BIT)
    }
}

impl mux::Muxer<StdIngress> {
    pub fn block(&mut self, cancel: Cancel) -> Result<(), std::io::Error> {
        loop {
            match self.tick() {
                mux::TickOutcome::BearerError(err) => return Err(err),
                mux::TickOutcome::Idle => match cancel.is_set() {
                    true => break Ok(()),
                    false => (),
                },
                mux::TickOutcome::Busy => (),
                mux::TickOutcome::IngressDisconnected => break Ok(()),
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

pub type StdChannel = (u16, Sender<Message>, Receiver<Payload>);

pub type StdChannelBuffer = ChannelBuffer<StdChannel>;

impl agents::Channel for StdChannel {
    fn enqueue_chunk(&mut self, payload: Payload) -> Result<(), agents::ChannelError> {
        match self.1.send((self.0, payload)) {
            Ok(_) => Ok(()),
            Err(SendError((_, payload))) => Err(agents::ChannelError::NotConnected(Some(payload))),
        }
    }

    fn dequeue_chunk(&mut self) -> Result<Payload, agents::ChannelError> {
        match self.2.recv() {
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
