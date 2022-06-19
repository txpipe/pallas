use crate::{
    agents::{self, ChannelBuffer},
    bearers::Bearer,
    demux, mux, Payload,
};

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{channel, Receiver, SendError, Sender, TryRecvError},
        Arc, Condvar, Mutex,
    },
    thread::{spawn, JoinHandle},
    time::Duration,
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

pub struct EgressParking(Mutex<bool>, Condvar);

impl EgressParking {
    fn new() -> Self {
        Self(Mutex::new(false), Condvar::new())
    }

    fn set_no_data(&self) {
        let EgressParking(lock, _) = self;
        let mut has_data = lock.lock().unwrap();
        *has_data = false;
    }

    fn park(&self) -> bool {
        let EgressParking(lock, cvar) = self;
        let guard = lock.lock().unwrap();
        let (result, _) = cvar
            .wait_timeout(guard, Duration::from_millis(500))
            .unwrap();

        *result
    }

    fn unpark(&self) {
        let EgressParking(lock, cvar) = self;
        let mut has_data = lock.lock().unwrap();
        *has_data = true;
        cvar.notify_all();
    }
}

pub struct StdEgress(Sender<Payload>, Arc<EgressParking>);

impl demux::Egress for StdEgress {
    fn send(&self, payload: Payload) -> Result<(), demux::EgressError> {
        let StdEgress(chann, parking) = self;

        match Sender::send(chann, payload) {
            Ok(_) => {
                parking.unpark();
                Ok(())
            }
            Err(SendError(p)) => Err(demux::EgressError(p)),
        }
    }
}

pub struct StdPlexer {
    pub muxer: mux::Muxer<StdIngress>,
    pub demuxer: demux::Demuxer<StdEgress>,
    pub egress_parking: Arc<EgressParking>,
}

impl StdPlexer {
    pub fn new(bearer: Bearer) -> Self {
        Self {
            muxer: mux::Muxer::new(bearer.clone()),
            demuxer: demux::Demuxer::new(bearer),
            egress_parking: Arc::new(EgressParking::new()),
        }
    }
}

impl StdPlexer {
    pub fn use_channel(&mut self, protocol: u16) -> StdChannel {
        let (demux_tx, demux_rx) = channel::<Payload>();
        let (mux_tx, mux_rx) = channel::<Payload>();

        self.muxer.register(protocol, mux_rx);
        self.demuxer
            .register(protocol, StdEgress(demux_tx, self.egress_parking.clone()));

        (mux_tx, demux_rx)
    }
}

impl mux::Muxer<StdIngress> {
    pub fn block(
        &mut self,
        cancel: Cancel,
        parking: Arc<EgressParking>,
    ) -> Result<(), std::io::Error> {
        loop {
            match self.tick() {
                mux::TickOutcome::BearerError(err) => return Err(err),
                mux::TickOutcome::Idle => match cancel.is_set() {
                    true => break Ok(()),
                    false => {
                        parking.set_no_data();
                        parking.park();
                    }
                },
                mux::TickOutcome::Busy => (),
            }
        }
    }

    pub fn spawn(mut self, parking: Arc<EgressParking>) -> Loop {
        let cancel = Cancel::default();
        let cancel2 = cancel.clone();
        let thread = spawn(move || self.block(cancel2, parking));

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
