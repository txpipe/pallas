use byteorder::{ByteOrder, NetworkEndian, WriteBytesExt};
use mio::{Events, Interest, Poll, Token};
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::Debug;
use std::io::{self, ErrorKind};
use std::io::{Read, Write};
use std::net::ToSocketAddrs;
use std::sync::mpsc::TryRecvError;
use std::time::{Duration, Instant};
use thiserror::Error;
use tracing::{debug, error, event_enabled, instrument, trace, warn};

use mio::net::{TcpListener, TcpStream};

#[cfg(target_family = "unix")]
use mio::net::{UnixListener, UnixStream};

use pallas_multiplexer::agents::{self, ChannelBuffer};

#[derive(Error, Debug)]
pub enum Error {
    #[error("unexpected IO error from bearer")]
    BearerIO(#[source] io::Error),

    #[error("bearer is not ready for IO operation")]
    BearerNotReady,

    #[error("bearer was closed by other party")]
    BearerClosed,

    #[error("{0}")]
    AddressResolution(#[source] io::Error),

    #[error("no address to connect to")]
    NoAddress,

    #[error("bearer is not registered")]
    InvalidBearer,

    #[error("protocol {0} is not registered")]
    InvalidProtocol(Protocol),

    #[error("message for protocol {0} failed to be dispatched")]
    DispatchFailure(Protocol),
}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        match value.kind() {
            ErrorKind::WouldBlock => Self::BearerNotReady,
            _ => Self::BearerIO(value),
        }
    }
}

pub type Timestamp = u32;

pub type Payload<'a> = Cow<'a, [u8]>;

pub type Protocol = u16;

pub struct Header {
    pub protocol: Protocol,
    pub timestamp: Timestamp,
    pub payload_len: u16,
}

pub struct Segment<'a> {
    pub header: Header,
    pub payload: Payload<'a>,
}

impl<'a> Segment<'a> {
    pub fn new(clock: &Instant, protocol: u16, payload: Payload<'a>) -> Self {
        Segment {
            header: Header {
                timestamp: clock.elapsed().as_micros() as u32,
                protocol,
                payload_len: payload.len() as u16,
            },
            payload,
        }
    }
}

fn write_segment(writer: &mut impl Write, segment: Segment) -> Result<(), std::io::Error> {
    debug!(protocol = segment.header.protocol, "outbound message");

    let mut msg = Vec::new();
    msg.write_u32::<NetworkEndian>(segment.header.timestamp)?;
    msg.write_u16::<NetworkEndian>(segment.header.protocol)?;
    msg.write_u16::<NetworkEndian>(segment.payload.len() as u16)?;
    msg.write_all(segment.payload.as_ref())?;

    if event_enabled!(tracing::Level::TRACE) {
        trace!(
            segment.header.protocol,
            length = segment.payload.len(),
            message = hex::encode(&msg),
            "writing segment"
        );
    }

    writer.write_all(&msg)?;
    writer.flush()
}

#[derive(Debug)]
pub enum Bearer {
    Tcp(TcpStream),

    #[cfg(target_family = "unix")]
    Unix(UnixStream),
}

impl Bearer {
    pub fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Bearer::Tcp(x) => x.read(buf),
            Bearer::Unix(x) => x.read(buf),
        }
    }

    pub fn write_segment(&mut self, segment: Segment) -> Result<(), Error> {
        match self {
            Bearer::Tcp(s) => write_segment(s, segment).map_err(Error::from),

            #[cfg(target_family = "unix")]
            Bearer::Unix(s) => write_segment(s, segment).map_err(Error::from),
        }
    }
}

impl From<TcpStream> for Bearer {
    fn from(stream: TcpStream) -> Self {
        Bearer::Tcp(stream)
    }
}

#[cfg(target_family = "unix")]
impl From<UnixStream> for Bearer {
    fn from(stream: UnixStream) -> Self {
        Bearer::Unix(stream)
    }
}

pub struct BearerHandler {
    inner: Bearer,
    outbound_buf: Vec<u8>,
    inbound_buf: Vec<u8>,
    socket_can_read: bool,
    socket_can_write: bool,
}

const HEADER_LEN: usize = 8;

impl BearerHandler {
    fn read_from_bearer(&mut self) -> Result<(), Error> {
        loop {
            let mut loop_buf = [0u8; 4096];

            match self.inner.read(&mut loop_buf) {
                Ok(0) => {
                    // Reading 0 bytes means the other side has closed the
                    // connection or is done writing, then so are we.
                    return Err(Error::BearerClosed);
                }
                Ok(n) => {
                    self.inbound_buf.extend_from_slice(&loop_buf[0..n]);
                }
                // Would block "errors" are the OS's way of saying that the
                // connection is not actually ready to perform this I/O operation.
                Err(err) => match err.kind() {
                    ErrorKind::Interrupted => continue,
                    ErrorKind::WouldBlock => return Ok(()),
                    _ => return Err(Error::BearerIO(err)),
                },
            }
        }
    }

    pub fn take_segment(&mut self) -> Option<Segment> {
        if self.inbound_buf.len() < HEADER_LEN {
            error!(
                len = self.inbound_buf.len(),
                "inbound buf too small for header"
            );
            return None;
        }

        let payload_len = NetworkEndian::read_u16(&self.inbound_buf[6..8]);

        let segment_len = HEADER_LEN + (payload_len as usize);

        if self.inbound_buf.len() < segment_len {
            return None;
        }

        let protocol = NetworkEndian::read_u16(&self.inbound_buf[4..6]) ^ 0x8000;
        error!(protocol, "found segment");

        let timestamp = NetworkEndian::read_u32(&self.inbound_buf[0..4]);

        let payload_source = &self.inbound_buf[HEADER_LEN..(HEADER_LEN + payload_len as usize)];

        let s = Segment {
            header: Header {
                protocol,
                timestamp,
                payload_len,
            },
            payload: Cow::Owned(payload_source.to_owned()),
        };

        error!(
            len = self.inbound_buf.len(),
            payload_len = payload_len,
            segment_len,
            "len before drain"
        );
        self.inbound_buf = Vec::from(&self.inbound_buf[segment_len..]);
        error!(len = self.inbound_buf.len(), "len after drain");

        Some(s)
    }
}

impl From<Bearer> for BearerHandler {
    fn from(value: Bearer) -> Self {
        BearerHandler {
            inner: value,
            outbound_buf: vec![],
            inbound_buf: vec![],
            socket_can_read: false,
            socket_can_write: false,
        }
    }
}

pub trait PlexerQueue {
    fn mux_peek(&mut self) -> Option<(Protocol, Payload)>;
    fn mux_commit(&mut self);
    fn demux_dispatch(&mut self, protocol: Protocol, payload: Payload) -> Result<(), Error>;
}

pub struct MioPlexer {
    bearers: HashMap<Token, BearerHandler>,
    poll: Poll,
    clock: Instant,
}

impl MioPlexer {
    pub fn new() -> Self {
        Self {
            bearers: Default::default(),
            clock: Instant::now(),
            poll: Poll::new().expect("mio syscall for system selector failed"),
        }
    }

    pub fn connect_tcp_bearer(&mut self, address: impl ToSocketAddrs) -> Result<Token, Error> {
        let address = address
            .to_socket_addrs()
            .map_err(Error::AddressResolution)?
            .into_iter()
            .next()
            .ok_or(Error::NoAddress)?;

        let mut stream = TcpStream::connect(address).map_err(Error::BearerIO)?;

        let token = self.bearers.keys().map(|t| t.0).max().unwrap_or(0);
        let token = Token(token);

        self.poll
            .registry()
            .register(
                &mut stream,
                token,
                Interest::READABLE.add(Interest::WRITABLE),
            )
            .map_err(Error::BearerIO)?;

        let bearer = Bearer::Tcp(stream);

        self.bearers.insert(token, bearer.into());

        Ok(token)
    }

    fn try_get_bearer(&mut self, token: Token) -> Result<&mut BearerHandler, Error> {
        self.bearers.get_mut(&token).ok_or(Error::InvalidBearer)
    }

    #[instrument(skip_all)]
    fn try_mux<Q: PlexerQueue>(&mut self, token: Token, queue: &mut Q) -> Result<(), Error> {
        if let Some((protocol, payload)) = queue.mux_peek() {
            let segment = Segment::new(&self.clock, protocol, Cow::Borrowed(&payload));
            let bearer = self.try_get_bearer(token)?;

            match bearer.inner.write_segment(segment) {
                Ok(_) => queue.mux_commit(),
                Err(Error::BearerNotReady) => (),
                Err(err) => return Err(err),
            }

            debug!("saving bearer as NOT writable for next time");
            bearer.socket_can_write = false;
        } else {
            let bearer = self.try_get_bearer(token)?;
            debug!("saving bearer as writable for next time");
            bearer.socket_can_write = true;
        }

        Ok(())
    }

    #[instrument(skip_all)]
    fn try_demux<Q: PlexerQueue>(&mut self, token: Token, queue: &mut Q) -> Result<(), Error> {
        let bearer = self.try_get_bearer(token)?;

        bearer.read_from_bearer()?;

        while let Some(x) = bearer.take_segment() {
            queue.demux_dispatch(x.header.protocol, x.payload)?;
        }

        Ok(())
    }

    #[instrument(skip_all)]
    pub fn poll<Q>(&mut self, queue: &mut Q, timeout: Duration) -> Result<(), Error>
    where
        Q: PlexerQueue,
    {
        let old_writable: Vec<_> = self
            .bearers
            .iter()
            .filter(|(_, b)| b.socket_can_write)
            .map(|(t, _)| *t)
            .collect();

        for token in old_writable {
            debug!("old writable event");
            self.try_mux(token, queue)?;
        }

        let mut events = Events::with_capacity(64);
        self.poll.poll(&mut events, Some(timeout))?;

        if events.is_empty() {
            debug!("poll events is empty");
            return Ok(());
        }

        for evt in events.iter() {
            let token = evt.token();

            if !evt.is_writable() && !evt.is_readable() {
                error!(?evt, "polling resulted in error");
            }

            if evt.is_writable() {
                debug!("writable event");
                self.try_mux(token, queue)?;
            }

            if evt.is_readable() {
                debug!("readable event");
                self.try_demux(token, queue)?;
            }
        }

        Ok(())
    }
}

pub type SimpleSender = std::sync::mpsc::Sender<Vec<u8>>;

pub type SimpleReceiver = std::sync::mpsc::Receiver<(Protocol, Vec<u8>)>;

#[derive(Debug)]
pub struct SimplePlexerQueue {
    senders: HashMap<Protocol, SimpleSender>,
    receiver: SimpleReceiver,
    temp: Option<(Protocol, Vec<u8>)>,
}

impl SimplePlexerQueue {
    pub fn new(receiver: SimpleReceiver) -> Self {
        Self {
            receiver,
            temp: None,
            senders: Default::default(),
        }
    }

    pub fn register_channel(&mut self, protocol: Protocol, sender: SimpleSender) {
        self.senders.insert(protocol, sender);
    }
}

impl PlexerQueue for SimplePlexerQueue {
    fn demux_dispatch(&mut self, protocol: Protocol, payload: Payload) -> Result<(), Error> {
        let sender = self
            .senders
            .get(&protocol)
            .ok_or(Error::InvalidProtocol(protocol))?;

        sender
            .send(payload.into())
            .map_err(|_| Error::DispatchFailure(protocol))?;

        Ok(())
    }

    fn mux_peek(&mut self) -> Option<(Protocol, Payload)> {
        if self.temp.is_none() {
            match self.receiver.try_recv() {
                Ok(x) => {
                    self.temp = Some(x);
                }
                //Err(TryRecvError::Disconnected) => todo!(),
                _ => (),
            }
        }

        if let Some((protocol, payload)) = &self.temp {
            return Some((*protocol, Cow::Borrowed(payload)));
        }

        None
    }

    fn mux_commit(&mut self) {
        self.temp = None;
    }
}

pub struct SimpleChannel(
    pub Protocol,
    pub std::sync::mpsc::Sender<(Protocol, Vec<u8>)>,
    pub std::sync::mpsc::Receiver<Vec<u8>>,
);

impl agents::Channel for SimpleChannel {
    fn enqueue_chunk(&mut self, payload: Vec<u8>) -> Result<(), agents::ChannelError> {
        self.1
            .send((self.0, payload))
            .map_err(|_| agents::ChannelError::NotConnected(None))
    }

    fn dequeue_chunk(&mut self) -> Result<Vec<u8>, agents::ChannelError> {
        let payload = self
            .2
            .recv()
            .map_err(|_| agents::ChannelError::NotConnected(None))?;

        Ok(payload)
    }
}
