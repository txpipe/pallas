use std::thread::{self, JoinHandle};

use crate::{bearers::Bearer, demux, mux::Muxer, Cancel};

#[derive(Debug)]
pub struct Loop<TBearer>
where
    TBearer: Bearer,
{
    cancel: Cancel,
    thread: JoinHandle<Result<(), TBearer::Error>>,
}

impl<TBearer> Loop<TBearer>
where
    TBearer: Bearer,
{
    pub fn cancel(&self) {
        self.cancel.set();
    }

    pub fn join(self) -> Result<(), TBearer::Error> {
        self.thread.join().unwrap()
    }
}

pub fn spawn_muxer<TBearer>(mut muxer: Muxer<TBearer>) -> Loop<TBearer>
where
    TBearer: Bearer + 'static,
    TBearer::Error: Send,
{
    let cancel = Cancel::default();
    let cancel2 = cancel.clone();
    let thread = thread::spawn(move || muxer.block(cancel2));

    Loop { cancel, thread }
}

pub fn spawn_demuxer<TBearer>(mut demuxer: demux::Demuxer<TBearer>) -> Loop<TBearer>
where
    TBearer: Bearer + 'static,
    TBearer::Error: Send,
{
    let cancel = Cancel::default();
    let cancel2 = cancel.clone();
    let thread = thread::spawn(move || demuxer.block(cancel2));

    Loop { cancel, thread }
}
