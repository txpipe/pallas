//! A multiplexer of several mini-protocols through a single bearer
//!
//! # Message reassembly and size limits
//!
//! [`ChannelBuffer`](crate::multiplexer::ChannelBuffer) reassembles mini-protocol messages from segments. It finds
//! where a message ends by scanning each inbound byte once, then decodes the
//! message, so receiving takes time linear in the bytes received. A message is
//! expected to be exactly one CBOR item; see
//! [`recv_full_msg`](crate::multiplexer::ChannelBuffer::recv_full_msg) for how
//! other shapes are received.
//!
//! [`recv_full_msg`](crate::multiplexer::ChannelBuffer::recv_full_msg) buffers a message of any size.
//! [`recv_full_msg_with_limit`](crate::multiplexer::ChannelBuffer::recv_full_msg_with_limit) stops once the message being
//! received is larger than a given number of bytes, checking after each
//! segment, so it buffers at most one segment beyond the limit. It then fails
//! with [`Error::Decoding`](crate::multiplexer::Error::Decoding) and the message
//! `message of at least <seen> bytes exceeds the size limit of <limit> bytes`.
//! Nothing is sent to the peer; drop the connection, as for any decoding error.
//!
//! The handshake agents and the node-to-node agents (chain-sync, block-fetch,
//! tx-submission, keep-alive and peer-sharing) receive with the per-state
//! limits of the Ouroboros network specification, published as constants in
//! each protocol module. Each of these agents has a `disable_size_limits`
//! method that restores unlimited receiving for that agent. The node-to-client
//! agents receive without limits.
//!
//! Both receives refuse CBOR whose containers nest deeper than four times
//! [`pallas_codec::MAX_DECODE_DEPTH`].

mod scanner;

use std::collections::HashMap;

use byteorder::{ByteOrder, NetworkEndian};
use pallas_codec::{Fragment, minicbor};
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::task::JoinHandle;
use tokio::time::Instant;
use tokio::{select, sync::mpsc::error::SendError};
use tracing::{debug, error, trace, warn};

type IOResult<T> = tokio::io::Result<T>;

use tokio::net as tcp;

#[cfg(unix)]
use tokio::net as unix;

#[cfg(windows)]
use tokio::net::windows::named_pipe::NamedPipeClient;

#[cfg(windows)]
use tokio::io::{ReadHalf, WriteHalf};

const HEADER_LEN: usize = 8;

/// Wall-clock microseconds since the multiplexer started, used to stamp segments.
pub type Timestamp = u32;

/// Raw bytes of a single segment payload.
pub type Payload = Vec<u8>;

/// Mini-protocol channel identifier (16-bit).
pub type Protocol = u16;

/// 8-byte segment header that prefixes every multiplexed payload.
#[derive(Debug)]
pub struct Header {
    /// Mini-protocol channel that owns the segment.
    pub protocol: Protocol,
    /// Wall-clock microseconds since the multiplexer started.
    pub timestamp: Timestamp,
    /// Length in bytes of the payload that follows this header.
    pub payload_len: u16,
}

impl From<&[u8]> for Header {
    fn from(value: &[u8]) -> Self {
        let timestamp = NetworkEndian::read_u32(&value[0..4]);
        let protocol = NetworkEndian::read_u16(&value[4..6]);
        let payload_len = NetworkEndian::read_u16(&value[6..8]);

        Self {
            timestamp,
            protocol,
            payload_len,
        }
    }
}

impl From<Header> for [u8; 8] {
    fn from(value: Header) -> Self {
        let mut out = [0u8; 8];
        NetworkEndian::write_u32(&mut out[0..4], value.timestamp);
        NetworkEndian::write_u16(&mut out[4..6], value.protocol);
        NetworkEndian::write_u16(&mut out[6..8], value.payload_len);

        out
    }
}

/// A single segment: header plus its raw payload bytes.
pub struct Segment {
    /// Segment header (protocol, timestamp, length).
    pub header: Header,
    /// Raw payload bytes.
    pub payload: Payload,
}

/// Underlying transport carrying multiplexed segments.
pub enum Bearer {
    /// TCP socket (node-to-node).
    Tcp(tcp::TcpStream),

    /// Unix domain socket (node-to-client on Unix).
    #[cfg(unix)]
    Unix(unix::UnixStream),

    /// Windows named pipe (node-to-client on Windows).
    #[cfg(windows)]
    NamedPipe(NamedPipeClient),
}

impl Bearer {
    fn configure_tcp(stream: &tcp::TcpStream) -> IOResult<()> {
        let sock_ref = socket2::SockRef::from(&stream);
        let mut tcp_keepalive = socket2::TcpKeepalive::new();
        tcp_keepalive = tcp_keepalive.with_time(tokio::time::Duration::from_secs(20));
        tcp_keepalive = tcp_keepalive.with_interval(tokio::time::Duration::from_secs(20));
        sock_ref.set_tcp_keepalive(&tcp_keepalive)?;
        sock_ref.set_tcp_nodelay(true)?;
        sock_ref.set_linger(Some(std::time::Duration::from_secs(0)))?;

        Ok(())
    }

    /// Connect a TCP bearer to `addr`, applying the default keep-alive and
    /// no-delay socket settings.
    pub async fn connect_tcp(addr: impl tcp::ToSocketAddrs) -> Result<Self, tokio::io::Error> {
        let stream = tcp::TcpStream::connect(addr).await?;
        Self::configure_tcp(&stream)?;
        Ok(Self::Tcp(stream))
    }

    /// Same as [`Self::connect_tcp`] but aborts with `TimedOut` after `timeout`.
    pub async fn connect_tcp_timeout(
        addr: impl tcp::ToSocketAddrs,
        timeout: std::time::Duration,
    ) -> IOResult<Self> {
        select! {
            result = Self::connect_tcp(addr) => result,
            _ = tokio::time::sleep(timeout) => Err(tokio::io::Error::new(tokio::io::ErrorKind::TimedOut, "connect timeout")),
        }
    }

    /// Accept the next TCP connection from `listener` as a bearer.
    pub async fn accept_tcp(listener: &tcp::TcpListener) -> IOResult<(Self, std::net::SocketAddr)> {
        let (stream, addr) = listener.accept().await?;
        Self::configure_tcp(&stream)?;
        Ok((Self::Tcp(stream), addr))
    }

    /// Connect a Unix-domain bearer to `path`.
    #[cfg(unix)]
    pub async fn connect_unix(path: impl AsRef<std::path::Path>) -> IOResult<Self> {
        let stream = unix::UnixStream::connect(path).await?;
        Ok(Self::Unix(stream))
    }

    /// Accept the next Unix-domain connection from `listener` as a bearer.
    #[cfg(unix)]
    pub async fn accept_unix(
        listener: &unix::UnixListener,
    ) -> IOResult<(Self, unix::unix::SocketAddr)> {
        let (stream, addr) = listener.accept().await?;
        Ok((Self::Unix(stream), addr))
    }

    /// Connect to a Windows named pipe as a bearer.
    #[cfg(windows)]
    pub fn connect_named_pipe(pipe_name: impl AsRef<std::ffi::OsStr>) -> IOResult<Self> {
        let client = tokio::net::windows::named_pipe::ClientOptions::new().open(&pipe_name)?;
        Ok(Self::NamedPipe(client))
    }

    /// Split the bearer into independent read and write halves.
    pub fn into_split(self) -> (BearerReadHalf, BearerWriteHalf) {
        match self {
            Bearer::Tcp(x) => {
                let (r, w) = x.into_split();
                (BearerReadHalf::Tcp(r), BearerWriteHalf::Tcp(w))
            }

            #[cfg(unix)]
            Bearer::Unix(x) => {
                let (r, w) = x.into_split();
                (BearerReadHalf::Unix(r), BearerWriteHalf::Unix(w))
            }

            #[cfg(windows)]
            Bearer::NamedPipe(x) => {
                let (read, write) = tokio::io::split(x);
                let reader = BearerReadHalf::NamedPipe(read);
                let writer = BearerWriteHalf::NamedPipe(write);

                (reader, writer)
            }
        }
    }
}

/// Read half of a split [`Bearer`].
pub enum BearerReadHalf {
    /// TCP read half.
    Tcp(tcp::tcp::OwnedReadHalf),

    /// Unix-domain read half.
    #[cfg(unix)]
    Unix(unix::unix::OwnedReadHalf),

    /// Named-pipe read half (Windows).
    #[cfg(windows)]
    NamedPipe(ReadHalf<NamedPipeClient>),
}

impl BearerReadHalf {
    async fn read_exact(&mut self, buf: &mut [u8]) -> IOResult<usize> {
        match self {
            BearerReadHalf::Tcp(x) => x.read_exact(buf).await,

            #[cfg(unix)]
            BearerReadHalf::Unix(x) => x.read_exact(buf).await,

            #[cfg(windows)]
            BearerReadHalf::NamedPipe(x) => x.read_exact(buf).await,
        }
    }
}

/// Write half of a split [`Bearer`].
pub enum BearerWriteHalf {
    /// TCP write half.
    Tcp(tcp::tcp::OwnedWriteHalf),

    /// Unix-domain write half.
    #[cfg(unix)]
    Unix(unix::unix::OwnedWriteHalf),

    /// Named-pipe write half (Windows).
    #[cfg(windows)]
    NamedPipe(WriteHalf<NamedPipeClient>),
}

impl BearerWriteHalf {
    async fn write_all(&mut self, buf: &[u8]) -> IOResult<()> {
        match self {
            Self::Tcp(x) => x.write_all(buf).await,

            #[cfg(unix)]
            Self::Unix(x) => x.write_all(buf).await,

            #[cfg(windows)]
            Self::NamedPipe(x) => x.write_all(buf).await,
        }
    }

    async fn flush(&mut self) -> IOResult<()> {
        match self {
            Self::Tcp(x) => x.flush().await,

            #[cfg(unix)]
            Self::Unix(x) => x.flush().await,

            #[cfg(windows)]
            Self::NamedPipe(x) => x.flush().await,
        }
    }

    /// Let the OS deliver what was written after the bearer is closed, instead
    /// of resetting the connection and discarding it.
    fn deliver_on_close(&self) {
        if let Self::Tcp(x) = self {
            let _ = socket2::SockRef::from(x.as_ref()).set_linger(None);
        }
    }
}

/// Errors produced by the multiplexer and its agent channels.
#[derive(Debug, Error)]
pub enum Error {
    /// The bearer returned EOF before a segment could be fully read.
    #[error("no data available in bearer to complete segment")]
    EmptyBearer,

    /// Underlying bearer I/O error.
    #[error("bearer I/O error")]
    BearerIo(tokio::io::Error),

    /// Failed to decode a message off the wire.
    #[error("failure to encode channel message")]
    Decoding(String),

    /// Failed to encode a message for the wire.
    #[error("failure to decode channel message")]
    Encoding(String),

    /// Agent could not push an outbound chunk to the muxer for `Protocol`.
    #[error("agent failed to enqueue chunk for protocol {0}")]
    AgentEnqueue(Protocol, Payload),

    /// Agent could not pull an inbound chunk from the demuxer.
    #[error("agent failed to dequeue chunk")]
    AgentDequeue,

    /// Demuxer could not deliver an inbound chunk to the subscribed agent.
    #[error("plexer failed to dumux chunk for protocol {0}")]
    PlexerDemux(Protocol, Payload),

    /// Muxer could not write a chunk to the bearer.
    #[error("plexer failed to mux chunk")]
    PlexerMux,

    /// Aborting the spawned muxer / demuxer tasks failed.
    #[error("failure to abort the plexer threads")]
    AbortFailure,
}

type EgressChannel = tokio::sync::mpsc::Sender<Payload>;
type Egress = HashMap<Protocol, EgressChannel>;

const EGRESS_MSG_QUEUE_BUFFER: usize = 100;

/// Reads segments off the bearer and dispatches them to subscribed agents.
pub struct Demuxer(BearerReadHalf, Egress);

impl Demuxer {
    /// Build a demuxer over the read half of a bearer.
    pub fn new(bearer: BearerReadHalf) -> Self {
        let egress = HashMap::new();
        Self(bearer, egress)
    }

    /// Read the next segment off the bearer and return its `(protocol, payload)`.
    pub async fn read_segment(&mut self) -> Result<(Protocol, Payload), Error> {
        trace!("waiting for segment header");
        let mut buf = vec![0u8; HEADER_LEN];
        self.0.read_exact(&mut buf).await.map_err(Error::BearerIo)?;
        let header = Header::from(buf.as_slice());

        trace!("waiting for full segment");
        let segment_size = header.payload_len as usize;
        let mut buf = vec![0u8; segment_size];
        self.0.read_exact(&mut buf).await.map_err(Error::BearerIo)?;

        Ok((header.protocol, buf))
    }

    async fn demux(&mut self, protocol: Protocol, payload: Payload) -> Result<(), Error> {
        let channel = self.1.get(&protocol);

        if let Some(sender) = channel {
            sender
                .send(payload)
                .await
                .map_err(|err| Error::PlexerDemux(protocol, err.0))?;
        } else {
            warn!(protocol, "message for unregistered protocol");
        }

        Ok(())
    }

    /// Register an agent's interest in a protocol channel and return the
    /// receiver it will pull inbound payloads from.
    pub fn subscribe(&mut self, protocol: Protocol) -> tokio::sync::mpsc::Receiver<Payload> {
        let (sender, recv) = tokio::sync::mpsc::channel(EGRESS_MSG_QUEUE_BUFFER);

        // keep track of the sender
        self.1.insert(protocol, sender);

        // return the receiver for the agent
        recv
    }

    /// Read one segment and dispatch it. Returns `Ok` after a single iteration.
    pub async fn tick(&mut self) -> Result<(), Error> {
        let (protocol, payload) = self.read_segment().await?;
        trace!(protocol, "demux happening");
        self.demux(protocol, payload).await
    }

    /// Run the demux loop until an error occurs.
    pub async fn run(&mut self) -> Result<(), Error> {
        loop {
            if let Err(err) = self.tick().await {
                break Err(err);
            }
        }
    }
}

type Ingress = (
    tokio::sync::mpsc::Sender<(Protocol, Payload)>,
    tokio::sync::mpsc::Receiver<(Protocol, Payload)>,
);

type Clock = Instant;

const INGRESS_MSG_QUEUE_BUFFER: usize = 100;

/// Collects payloads from all agents and writes them to the bearer as segments.
pub struct Muxer(BearerWriteHalf, Clock, Ingress);

impl Muxer {
    /// Build a muxer over the write half of a bearer.
    pub fn new(bearer: BearerWriteHalf) -> Self {
        let ingress = tokio::sync::mpsc::channel(INGRESS_MSG_QUEUE_BUFFER);
        let clock = Instant::now();
        Self(bearer, clock, ingress)
    }

    async fn write_segment(&mut self, protocol: u16, payload: &[u8]) -> Result<(), std::io::Error> {
        let header = Header {
            protocol,
            timestamp: self.1.elapsed().as_micros() as u32,
            payload_len: payload.len() as u16,
        };

        let buf: [u8; 8] = header.into();
        self.0.write_all(&buf).await?;
        self.0.write_all(payload).await?;

        self.0.flush().await?;

        Ok(())
    }

    /// Write a single `(protocol, payload)` pair to the bearer as one segment.
    pub async fn mux(&mut self, msg: (Protocol, Payload)) -> Result<(), Error> {
        self.write_segment(msg.0, &msg.1)
            .await
            .map_err(|_| Error::PlexerMux)?;

        if tracing::event_enabled!(tracing::Level::TRACE) {
            trace!(
                protocol = msg.0,
                data = hex::encode(&msg.1),
                "write to bearer"
            );
        }

        Ok(())
    }

    /// Clone the ingress sender so an agent can push outbound payloads here.
    pub fn clone_sender(&self) -> tokio::sync::mpsc::Sender<(Protocol, Payload)> {
        self.2.0.clone()
    }

    /// Take one queued message and write it as a segment.
    pub async fn tick(&mut self) -> Result<(), Error> {
        let msg = self.2.1.recv().await;

        if let Some(x) = msg {
            trace!(protocol = x.0, "mux happening");
            self.mux(x).await?
        }

        Ok(())
    }

    /// Run the mux loop until an error occurs.
    pub async fn run(&mut self) -> Result<(), Error> {
        loop {
            if let Err(err) = self.tick().await {
                break Err(err);
            }
        }
    }

    /// Run the mux loop until an error occurs or `stop` fires. On stop, write
    /// the messages already queued and return. A dropped `stop` sender is not
    /// a stop: the loop keeps running, as [`Self::run`] does.
    async fn run_until(&mut self, mut stop: tokio::sync::mpsc::Receiver<()>) -> Result<(), Error> {
        let stop = async {
            if stop.recv().await.is_none() {
                std::future::pending::<()>().await
            }
        };
        tokio::pin!(stop);

        loop {
            select! {
                biased;
                _ = &mut stop => break,
                msg = self.2.1.recv() => {
                    if let Some(msg) = msg {
                        self.mux(msg).await?;
                    }
                }
            }
        }

        while let Ok(msg) = self.2.1.try_recv() {
            self.mux(msg).await?;
        }

        self.0.deliver_on_close();

        Ok(())
    }
}

type ToPlexerPort = tokio::sync::mpsc::Sender<(Protocol, Payload)>;
type FromPlexerPort = tokio::sync::mpsc::Receiver<Payload>;

/// Bidirectional channel exposed to a mini-protocol agent: send raw chunks out
/// through the muxer and receive inbound chunks from the demuxer.
pub struct AgentChannel {
    protocol: Protocol,
    to_plexer: ToPlexerPort,
    from_plexer: FromPlexerPort,
}

impl AgentChannel {
    fn for_client(
        protocol: Protocol,
        to_plexer: ToPlexerPort,
        from_plexer: FromPlexerPort,
    ) -> Self {
        Self {
            protocol,
            from_plexer,
            to_plexer,
        }
    }

    fn for_server(
        protocol: Protocol,
        to_plexer: ToPlexerPort,
        from_plexer: FromPlexerPort,
    ) -> Self {
        Self {
            protocol,
            from_plexer,
            to_plexer,
        }
    }

    /// Push an outbound chunk to the muxer for this protocol.
    pub async fn enqueue_chunk(&mut self, chunk: Payload) -> Result<(), Error> {
        self.to_plexer
            .send((self.protocol, chunk))
            .await
            .map_err(|SendError((protocol, payload))| Error::AgentEnqueue(protocol, payload))
    }

    /// Pull the next inbound chunk for this protocol from the demuxer.
    pub async fn dequeue_chunk(&mut self) -> Result<Payload, Error> {
        self.from_plexer.recv().await.ok_or(Error::AgentDequeue)
    }
}

/// Handle to the spawned muxer and demuxer tasks of a running [`Plexer`].
pub struct RunningPlexer {
    demuxer: JoinHandle<Result<(), Error>>,
    muxer: JoinHandle<Result<(), Error>>,
    stop_muxer: tokio::sync::mpsc::Sender<()>,
}

impl RunningPlexer {
    /// Abort the muxer and demuxer tasks and wait until both are gone, so the
    /// bearer is closed on return. Anything still queued is discarded.
    pub(crate) async fn abort_and_join(self) {
        self.demuxer.abort();
        self.muxer.abort();

        let _ = self.demuxer.await;
        let _ = self.muxer.await;
    }

    /// Abort the muxer and demuxer tasks.
    pub async fn abort(self) {
        self.demuxer.abort();
        self.muxer.abort();
    }

    /// Write the messages already queued and close the bearer, letting the OS
    /// deliver them without waiting for the peer. Returns once both tasks are
    /// gone; if cancelled, the muxer still finishes on its own.
    pub(crate) async fn close(self) {
        let Self {
            demuxer,
            muxer,
            stop_muxer,
        } = self;

        // The bearer closes only once the muxer drops the write half too.
        demuxer.abort();
        let _ = stop_muxer.try_send(());

        let _ = muxer.await;
        let _ = demuxer.await;
    }
}

/// Pairs a [`Demuxer`] and a [`Muxer`] sharing a single bearer.
pub struct Plexer {
    demuxer: Demuxer,
    muxer: Muxer,
}

impl Plexer {
    /// Build a plexer over the given bearer.
    pub fn new(bearer: Bearer) -> Self {
        let (r, w) = bearer.into_split();

        Self {
            demuxer: Demuxer::new(r),
            muxer: Muxer::new(w),
        }
    }

    /// Open a client-side agent channel on the given protocol id.
    pub fn subscribe_client(&mut self, protocol: Protocol) -> AgentChannel {
        let to_plexer = self.muxer.clone_sender();
        let from_plexer = self.demuxer.subscribe(protocol ^ 0x8000);
        AgentChannel::for_client(protocol, to_plexer, from_plexer)
    }

    /// Open a server-side agent channel on the given protocol id.
    pub fn subscribe_server(&mut self, protocol: Protocol) -> AgentChannel {
        let to_plexer = self.muxer.clone_sender();
        let from_plexer = self.demuxer.subscribe(protocol);
        AgentChannel::for_server(protocol ^ 0x8000, to_plexer, from_plexer)
    }

    /// Spawn the muxer and demuxer loops on the current Tokio runtime and
    /// return a handle for aborting them.
    pub fn spawn(self) -> RunningPlexer {
        let mut demuxer = self.demuxer;
        let mut muxer = self.muxer;

        let (stop_muxer, stop) = tokio::sync::mpsc::channel(1);

        let demuxer = tokio::spawn(async move { demuxer.run().await });
        let muxer = tokio::spawn(async move { muxer.run_until(stop).await });

        RunningPlexer {
            demuxer,
            muxer,
            stop_muxer,
        }
    }
}

/// Protocol value that defines max segment length
pub const MAX_SEGMENT_PAYLOAD_LENGTH: usize = 65535;

fn size_limit_error(limit: usize, seen: usize) -> Error {
    // TODO(v2-breaking): replace with a dedicated size-limit error variant
    Error::Decoding(format!(
        "message of at least {seen} bytes exceeds the size limit of {limit} bytes"
    ))
}

/// A channel abstraction to hide the complexity of partial payloads
pub struct ChannelBuffer {
    channel: AgentChannel,
    temp: Vec<u8>,
    scanner: scanner::Scanner,
    limits_enabled: bool,
    /// Whether the last segment received was shorter than a full one.
    last_segment_short: bool,
    /// End of the first top-level item of the current message, once scanned.
    first_end: Option<usize>,
    /// Buffer length at the last decode of the current message that needed
    /// more bytes.
    decoded_len: usize,
    /// Bytes decoded by those attempts, which caps further attempts.
    decode_work: usize,
}

impl ChannelBuffer {
    /// Wrap a raw [`AgentChannel`] in a message-aware buffer.
    pub fn new(channel: AgentChannel) -> Self {
        Self {
            channel,
            temp: Vec::new(),
            scanner: scanner::Scanner::default(),
            limits_enabled: true,
            last_segment_short: false,
            first_end: None,
            decoded_len: 0,
            decode_work: 0,
        }
    }

    /// Turn the size limit passed to [`Self::recv_msg`] on or off. A buffer
    /// whose limits are off ignores that limit and receives messages of any
    /// size; this is how an agent restores unlimited receiving.
    pub(crate) fn set_size_limits(&mut self, enabled: bool) {
        self.limits_enabled = enabled;
    }

    /// Enqueues a msg as a sequence payload chunks
    pub async fn send_msg_chunks<M>(&mut self, msg: &M) -> Result<(), Error>
    where
        M: Fragment,
    {
        let mut payload = Vec::new();
        minicbor::encode(msg, &mut payload).map_err(|err| Error::Encoding(err.to_string()))?;

        let chunks = payload.chunks(MAX_SEGMENT_PAYLOAD_LENGTH);

        for chunk in chunks {
            self.channel.enqueue_chunk(Vec::from(chunk)).await?;
        }

        Ok(())
    }

    /// Reads from the channel until a complete message is found
    ///
    /// Buffers a message of any size. Use [`Self::recv_full_msg_with_limit`]
    /// when the peer is not trusted.
    ///
    /// # Message shapes
    ///
    /// A message is expected to be exactly one CBOR item, as every
    /// mini-protocol message is. Such a message is decoded once its last byte
    /// arrives, however it is split into segments; if its decoder stops inside
    /// the item, the rest of the item is taken with the message.
    ///
    /// Other shapes, a message spanning several top-level items or one whose
    /// arrays declare more items than they hold, are received on a best-effort
    /// basis that depends on how the message is split into segments. Decoding
    /// such a message before it ends is rationed, to keep the work linear in
    /// the bytes received. It decodes when it arrives:
    ///
    /// - in one segment (shorter than [`MAX_SEGMENT_PAYLOAD_LENGTH`] for arrays
    ///   that declare more items than they hold), or
    /// - as full segments followed by one shorter segment, which is how
    ///   [`Self::send_msg_chunks`] splits a message.
    ///
    /// It may wait for bytes that never come when its last segment is a full
    /// one, or when it arrives in several shorter segments. If its decoder
    /// stops inside a later top-level item, the receive fails with
    /// [`Error::Decoding`].
    pub async fn recv_full_msg<M>(&mut self) -> Result<M, Error>
    where
        M: Fragment,
    {
        self.recv_msg(None).await
    }

    /// Reads from the channel until a complete message of at most `max_size`
    /// bytes is found.
    ///
    /// Fails with [`Error::Decoding`] once the message is larger than
    /// `max_size`, checking after each segment, so at most one segment beyond
    /// the limit is buffered. The error reads `message of at least <seen>
    /// bytes exceeds the size limit of <limit> bytes`. Message shapes are
    /// received as described for [`Self::recv_full_msg`].
    pub async fn recv_full_msg_with_limit<M>(&mut self, max_size: usize) -> Result<M, Error>
    where
        M: Fragment,
    {
        self.recv_msg(Some(max_size)).await
    }

    /// Receive a message, of at most `limit` bytes when one is given and this
    /// buffer's limits are enabled (see [`Self::set_size_limits`]).
    pub(crate) async fn recv_msg<M>(&mut self, limit: Option<usize>) -> Result<M, Error>
    where
        M: Fragment,
    {
        let limit = limit.filter(|_| self.limits_enabled);

        trace!(len = self.temp.len(), "waiting for full message");

        loop {
            if let Some(msg) = self.try_take_msg::<M>(limit)? {
                debug!("decoding done");
                return Ok(msg);
            }

            // Every buffered byte belongs to the incomplete message.
            if let Some(limit) = limit
                && self.temp.len() > limit
            {
                return Err(size_limit_error(limit, self.temp.len()));
            }

            trace!("not enough data");

            let chunk = self.channel.dequeue_chunk().await?;
            self.last_segment_short = chunk.len() < MAX_SEGMENT_PAYLOAD_LENGTH;
            self.temp.extend(chunk);
        }
    }

    /// Decode the message at the start of the buffer once its bytes are all
    /// there, scanning only the bytes not scanned before.
    fn try_take_msg<M>(&mut self, limit: Option<usize>) -> Result<Option<M>, Error>
    where
        M: Fragment,
    {
        loop {
            match self.scanner.scan(&self.temp) {
                Ok(Some(end)) => {
                    if let Some(limit) = limit
                        && end > limit
                    {
                        return Err(size_limit_error(limit, end));
                    }

                    // The first item is always decoded; a message type made of
                    // several top-level items needs a later one.
                    let first = self.first_end.is_none();
                    self.first_end.get_or_insert(end);

                    if (first || self.retry_due())
                        && let Some(msg) = self.take_decoded(limit, false)?
                    {
                        return Ok(Some(msg));
                    }
                }
                Ok(None) if self.retry_due() && self.scanner.at_boundary(&self.temp) => {
                    return self.take_decoded(limit, true);
                }
                Ok(None) => return Ok(None),
                Err(scanner::ScanError::TooDeep) => {
                    return Err(Error::Decoding(format!(
                        "CBOR nesting exceeds the limit of {} levels",
                        scanner::MAX_NESTING
                    )));
                }
                // The decoder reports the error, or reads the bytes another way.
                Err(scanner::ScanError::Malformed) => {
                    return match self.take_decoded(limit, false)? {
                        Some(msg) => Ok(Some(msg)),
                        None => Err(Error::Decoding("malformed CBOR".into())),
                    };
                }
            }
        }
    }

    /// Whether to decode again a message that has needed more bytes, or that
    /// the scanner has not seen end (some encoders declare more items than
    /// they write). Only after new bytes ending a short segment, and only
    /// while the failed attempts have decoded no more than the buffer holds,
    /// so the decoding done for a message stays within a few times its bytes.
    // TODO(v2-breaking): return an error when decoding reads past the first
    // item of a message, instead of waiting for a retry.
    fn retry_due(&self) -> bool {
        self.last_segment_short
            && self.temp.len() > self.decoded_len
            && self.decode_work <= self.temp.len()
    }

    /// Note a decode that needs more bytes, against the message's budget.
    fn needs_more<M>(&mut self) -> Result<Option<M>, Error> {
        self.decoded_len = self.temp.len();
        self.decode_work += self.temp.len();
        Ok(None)
    }

    /// Decode a message from the start of the buffer and take its bytes.
    /// Returns `None` when the decoder needs more bytes, or, with `early`,
    /// when it stops inside an item the scanner has not seen end. A message
    /// that stops inside its first item takes the rest of that item, so no
    /// byte is ever scanned twice.
    fn take_decoded<M>(&mut self, limit: Option<usize>, early: bool) -> Result<Option<M>, Error>
    where
        M: Fragment,
    {
        let mut decoder = minicbor::Decoder::new(&self.temp);

        let msg = match decoder.decode() {
            Ok(msg) => msg,
            Err(err) if err.is_end_of_input() => return self.needs_more(),
            Err(err) => {
                error!(?err);
                trace!("{}", hex::encode(&self.temp));
                return Err(Error::Decoding(err.to_string()));
            }
        };

        let pos = decoder.position();

        if let Some(limit) = limit
            && pos > limit
        {
            return Err(size_limit_error(limit, pos));
        }

        let take = match self.first_end {
            _ if pos >= self.scanner.position() => pos,
            Some(first) if pos <= first => first,
            None if early => return self.needs_more(),
            _ => {
                return Err(Error::Decoding(format!(
                    "message of {pos} bytes ends inside a CBOR item of at least {} bytes",
                    self.scanner.position()
                )));
            }
        };

        self.temp.drain(..take);
        self.scanner = scanner::Scanner::default();
        self.first_end = None;
        self.decoded_len = 0;
        self.decode_work = 0;

        Ok(Some(msg))
    }

    /// Discard the buffer and return the underlying raw channel.
    pub fn unwrap(self) -> AgentChannel {
        self.channel
    }
}

impl From<AgentChannel> for ChannelBuffer {
    fn from(channel: AgentChannel) -> Self {
        ChannelBuffer::new(channel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pallas_codec::minicbor;

    #[tokio::test]
    async fn multiple_messages_in_same_payload() {
        let mut input = Vec::new();
        let in_part1 = (1u8, 2u8, 3u8);
        let in_part2 = (6u8, 5u8, 4u8);

        minicbor::encode(in_part1, &mut input).unwrap();
        minicbor::encode(in_part2, &mut input).unwrap();

        let (to_plexer, _) = tokio::sync::mpsc::channel(100);
        let (into_plexer, from_plexer) = tokio::sync::mpsc::channel(100);

        let channel = AgentChannel::for_client(0, to_plexer, from_plexer);

        into_plexer.send(input).await.unwrap();

        let mut buf = ChannelBuffer::new(channel);

        let out_part1 = buf.recv_full_msg::<(u8, u8, u8)>().await.unwrap();
        let out_part2 = buf.recv_full_msg::<(u8, u8, u8)>().await.unwrap();

        assert_eq!(in_part1, out_part1);
        assert_eq!(in_part2, out_part2);
    }

    #[tokio::test]
    async fn fragmented_message_in_multiple_payloads() {
        let mut input = Vec::new();
        let msg = (11u8, 12u8, 13u8, 14u8, 15u8, 16u8, 17u8);
        minicbor::encode(msg, &mut input).unwrap();

        let (to_plexer, _) = tokio::sync::mpsc::channel(100);
        let (into_plexer, from_plexer) = tokio::sync::mpsc::channel(100);

        let channel = AgentChannel::for_client(0, to_plexer, from_plexer);

        while !input.is_empty() {
            let chunk = Vec::from(input.drain(0..2).as_slice());
            into_plexer.send(chunk).await.unwrap();
        }

        let mut buf = ChannelBuffer::new(channel);

        let out_msg = buf
            .recv_full_msg::<(u8, u8, u8, u8, u8, u8, u8)>()
            .await
            .unwrap();

        assert_eq!(msg, out_msg);
    }

    /// A buffer over a channel, with the sender that feeds it segments.
    fn fed_buffer() -> (ChannelBuffer, tokio::sync::mpsc::Sender<Payload>) {
        let (to_plexer, _) = tokio::sync::mpsc::channel(100);
        let (into_plexer, from_plexer) = tokio::sync::mpsc::channel(100);
        let channel = AgentChannel::for_client(0, to_plexer, from_plexer);
        (ChannelBuffer::new(channel), into_plexer)
    }

    /// Feed `bytes` as segments of at most `segment` bytes, from a task.
    fn feed(into: tokio::sync::mpsc::Sender<Payload>, bytes: Vec<u8>, segment: usize) {
        tokio::spawn(async move {
            for chunk in bytes.chunks(segment) {
                if into.send(chunk.to_vec()).await.is_err() {
                    break;
                }
            }
        });
    }

    /// Await a receive that must succeed promptly rather than wait for more.
    async fn within_a_second<M>(recv: impl std::future::Future<Output = Result<M, Error>>) -> M {
        tokio::time::timeout(std::time::Duration::from_secs(1), recv)
            .await
            .expect("the message is received without waiting for more")
            .unwrap()
    }

    /// The `seen` figure of a size-limit error.
    fn seen_in(err: &Error) -> usize {
        let text = format!("{err:?}");
        let (_, rest) = text
            .split_once("message of at least ")
            .unwrap_or_else(|| panic!("not a size-limit error: {text}"));
        rest.split(' ').next().unwrap().parse().unwrap()
    }

    /// A CBOR byte string of exactly `len` encoded bytes, for `len` >= 259.
    fn byte_string_of(len: usize) -> Vec<u8> {
        let payload = len - 3;
        let mut bytes = vec![0x59];
        bytes.extend((payload as u16).to_be_bytes());
        bytes.extend(std::iter::repeat_n(7u8, payload));
        bytes
    }

    #[tokio::test]
    async fn limit_accepts_a_message_of_exactly_its_size() {
        let (mut buf, into) = fed_buffer();
        let mut input = byte_string_of(1000);
        input.extend(byte_string_of(1001));
        feed(into, input, MAX_SEGMENT_PAYLOAD_LENGTH);

        let msg: minicbor::bytes::ByteVec = buf.recv_full_msg_with_limit(1000).await.unwrap();
        assert_eq!(msg.len(), 997);

        let err = buf
            .recv_full_msg_with_limit::<minicbor::bytes::ByteVec>(1000)
            .await
            .unwrap_err();
        assert_eq!(seen_in(&err), 1001);
    }

    #[tokio::test]
    async fn limit_stops_a_never_ending_message_within_one_segment() {
        const LIMIT: usize = 1000;
        const SEGMENT: usize = 100;

        // An indefinite array that is never closed.
        let mut input = vec![0x9f];
        input.extend([0u8; 20 * LIMIT]);

        let (mut buf, into) = fed_buffer();
        feed(into, input, SEGMENT);

        let err = buf
            .recv_full_msg_with_limit::<Vec<u64>>(LIMIT)
            .await
            .unwrap_err();
        let seen = seen_in(&err);
        assert!(seen > LIMIT && seen <= LIMIT + SEGMENT, "seen {seen}");
    }

    /// Feed each of `segments` as one segment, from a task.
    fn feed_segments(into: tokio::sync::mpsc::Sender<Payload>, segments: Vec<Vec<u8>>) {
        tokio::spawn(async move {
            for segment in segments {
                if into.send(segment).await.is_err() {
                    break;
                }
            }
        });
    }

    #[tokio::test]
    async fn message_stopping_inside_its_item_takes_the_rest_of_it() {
        use crate::miniprotocols::keepalive::Message;

        // Keep-alive messages nested as the unread third item of the one
        // before, then a plain one, filling exactly one full segment.
        const LEVELS: usize = 13_106;
        let mut input = [0x83, 0x00, 0x19, 0x00, 0x01].repeat(LEVELS - 1);
        input.extend([0x82, 0x00, 0x19, 0x00, 0x01]);
        input.extend([0x82, 0x00, 0x19, 0x00, 0x07]);
        assert_eq!(input.len(), MAX_SEGMENT_PAYLOAD_LENGTH);

        for limit in [None, Some(65_535)] {
            let (mut buf, into) = fed_buffer();
            feed_segments(into, vec![input.clone()]);

            let first = within_a_second(buf.recv_msg::<Message>(limit)).await;
            assert!(matches!(first, Message::KeepAlive(1)), "{first:?}");
            let next = within_a_second(buf.recv_msg::<Message>(limit)).await;
            assert!(matches!(next, Message::KeepAlive(7)), "{next:?}");
        }
    }

    #[tokio::test]
    async fn message_stopping_inside_an_unfinished_item_waits_for_it() {
        use crate::miniprotocols::handshake::{Message, n2n::VersionData};

        // A proposal whose last version data has a field the decoder does not
        // read, split before that field, then a second proposal.
        let entry = [0x11, 0x85, 0x1a, 0x2d, 0x96, 0x4a, 0x09, 0xf4, 0x01, 0xf4];
        let mut head = vec![0x82, 0x00, 0xa1];
        head.extend(entry);
        let mut second = head.clone();
        second.push(0xf4);

        let (mut buf, into) = fed_buffer();
        feed_segments(into, vec![head, vec![0xf4], second]);

        for _ in 0..2 {
            let msg = within_a_second(buf.recv_full_msg::<Message<VersionData>>()).await;
            assert!(matches!(msg, Message::Propose(table) if table.values.contains_key(&17)));
        }
    }

    #[tokio::test]
    async fn message_declaring_more_items_than_it_holds_still_decodes() {
        use crate::miniprotocols::localmsgnotification::Message;

        // An array of three that holds two items, as older encoders wrote
        // this reply.
        let (mut buf, into) = fed_buffer();
        feed(
            into,
            vec![0x83, 0x02, 0x9f, 0xff],
            MAX_SEGMENT_PAYLOAD_LENGTH,
        );

        let msg = within_a_second(buf.recv_full_msg::<Message>()).await;
        assert!(matches!(msg, Message::ReplyMessagesBlocking(msgs) if msgs.is_empty()));
    }

    /// `[2, bytes]` under an array head that declares three items.
    #[derive(Debug)]
    struct CountShort(Vec<u8>);

    impl<'b> minicbor::Decode<'b, ()> for CountShort {
        fn decode(
            d: &mut minicbor::Decoder<'b>,
            _ctx: &mut (),
        ) -> Result<Self, minicbor::decode::Error> {
            d.array()?;
            d.u16()?;
            Ok(CountShort(d.bytes()?.to_vec()))
        }
    }

    impl minicbor::Encode<()> for CountShort {
        fn encode<W: minicbor::encode::Write>(
            &self,
            e: &mut minicbor::Encoder<W>,
            _ctx: &mut (),
        ) -> Result<(), minicbor::encode::Error<W::Error>> {
            e.array(3)?.u16(2)?.bytes(&self.0)?;
            Ok(())
        }
    }

    #[tokio::test]
    async fn count_short_message_over_several_segments_still_decodes() {
        let body = vec![9u8; 100_000];
        let mut input = vec![];
        minicbor::encode(CountShort(body.clone()), &mut input).unwrap();

        let (mut buf, into) = fed_buffer();
        feed(into, input, MAX_SEGMENT_PAYLOAD_LENGTH);

        let msg = within_a_second(buf.recv_full_msg::<CountShort>()).await;
        assert_eq!(msg.0, body);
    }

    /// `[2, n]` followed by `n` top-level `u8` items that belong to it.
    #[derive(Debug)]
    struct SeveralItems(Vec<u8>);

    impl<'b> minicbor::Decode<'b, ()> for SeveralItems {
        fn decode(
            d: &mut minicbor::Decoder<'b>,
            _ctx: &mut (),
        ) -> Result<Self, minicbor::decode::Error> {
            d.array()?;
            d.u16()?;
            let n = d.u32()?;
            (0..n).map(|_| d.u8()).collect::<Result<_, _>>().map(Self)
        }
    }

    impl minicbor::Encode<()> for SeveralItems {
        fn encode<W: minicbor::encode::Write>(
            &self,
            e: &mut minicbor::Encoder<W>,
            _ctx: &mut (),
        ) -> Result<(), minicbor::encode::Error<W::Error>> {
            e.array(2)?.u16(2)?.u32(self.0.len() as u32)?;
            for item in &self.0 {
                e.u8(*item)?;
            }
            Ok(())
        }
    }

    #[tokio::test]
    async fn message_of_several_items_over_full_segments_decodes() {
        const LEN: usize = 400_000;
        let mut input = vec![0x82, 0x02, 0x1a];
        input.extend(((LEN - 7) as u32).to_be_bytes());
        input.resize(LEN, 0);

        let (mut buf, into) = fed_buffer();
        feed(into, input, MAX_SEGMENT_PAYLOAD_LENGTH);

        let msg = within_a_second(buf.recv_full_msg::<SeveralItems>()).await;
        assert_eq!(msg.0.len(), LEN - 7);
    }

    #[tokio::test]
    async fn consecutive_count_short_messages_each_decode() {
        use crate::miniprotocols::localmsgnotification::Message;

        let (mut buf, into) = fed_buffer();
        feed_segments(into, vec![vec![0x83, 0x02, 0x9f, 0xff]; 2]);

        for _ in 0..2 {
            let msg = within_a_second(buf.recv_full_msg::<Message>()).await;
            assert!(matches!(msg, Message::ReplyMessagesBlocking(msgs) if msgs.is_empty()));
        }
    }

    /// `[2, [* u8]]` under an array head that declares three items.
    #[derive(Debug)]
    struct CountShortList(Vec<u8>);

    impl<'b> minicbor::Decode<'b, ()> for CountShortList {
        fn decode(
            d: &mut minicbor::Decoder<'b>,
            _ctx: &mut (),
        ) -> Result<Self, minicbor::decode::Error> {
            d.array()?;
            d.u16()?;
            Ok(CountShortList(d.decode()?))
        }
    }

    impl minicbor::Encode<()> for CountShortList {
        fn encode<W: minicbor::encode::Write>(
            &self,
            e: &mut minicbor::Encoder<W>,
            _ctx: &mut (),
        ) -> Result<(), minicbor::encode::Error<W::Error>> {
            e.array(3)?.u16(2)?.encode(&self.0)?;
            Ok(())
        }
    }

    #[tokio::test]
    async fn count_short_message_after_one_decoded_early_still_decodes() {
        // Each first message needs one early decode before its last segment;
        // the message after it is shorter than that decode.
        for first in [
            vec![vec![0x83, 0x02, 0x9f], vec![0xff]],
            vec![vec![0x83, 0x02, 0x9f, 0x01, 0x02], vec![0xff]],
        ] {
            let mut segments = first;
            segments.push(vec![0x83, 0x02, 0x80]);

            let (mut buf, into) = fed_buffer();
            feed_segments(into, segments);

            within_a_second(buf.recv_full_msg::<CountShortList>()).await;
            let next = within_a_second(buf.recv_full_msg::<CountShortList>()).await;
            assert!(next.0.is_empty());
        }
    }

    /// Receive one message from `segments`, sent one per segment, after which
    /// the sender closes: a receive that would wait for more bytes fails with
    /// [`Error::AgentDequeue`] once it has buffered all of them.
    async fn receive_from<M: Fragment>(segments: Vec<Vec<u8>>) -> Result<M, Error> {
        let total: usize = segments.iter().map(Vec::len).sum();
        let (mut buf, into) = fed_buffer();
        feed_segments(into, segments);

        let received = buf.recv_full_msg::<M>().await;
        if received.is_err() {
            assert_eq!(buf.temp.len(), total, "every byte is buffered");
        }
        received
    }

    fn assert_waits<M: std::fmt::Debug>(received: Result<M, Error>, case: &str) {
        assert!(
            matches!(received, Err(Error::AgentDequeue)),
            "{case}: {received:?}"
        );
    }

    fn segments_of(bytes: &[u8], segment: usize) -> Vec<Vec<u8>> {
        bytes.chunks(segment).map(<[u8]>::to_vec).collect()
    }

    #[tokio::test]
    async fn one_item_message_decodes_however_it_is_split() {
        type Msg = (u16, Vec<u32>, minicbor::bytes::ByteVec);
        let msg: Msg = (2, vec![1, 1_000, 100_000], vec![7u8; 300].into());
        let mut input = vec![];
        minicbor::encode(&msg, &mut input).unwrap();

        let mut splits = vec![segments_of(&input, 1), segments_of(&input, 16)];
        splits.extend((1..input.len()).map(|at| vec![input[..at].to_vec(), input[at..].to_vec()]));

        for segments in splits {
            assert_eq!(receive_from::<Msg>(segments).await.unwrap(), msg);
        }

        // Also when its last segment is a full one.
        let len = 2 * MAX_SEGMENT_PAYLOAD_LENGTH;
        let mut input = vec![0x5a];
        input.extend(((len - 5) as u32).to_be_bytes());
        input.resize(len, 7);
        let segments = segments_of(&input, MAX_SEGMENT_PAYLOAD_LENGTH);
        let msg = receive_from::<minicbor::bytes::ByteVec>(segments).await;
        assert_eq!(msg.unwrap().len(), len - 5);
    }

    /// A [`SeveralItems`] message of exactly `len` bytes, for `len` >= 7.
    fn several_items(len: usize) -> Vec<u8> {
        let mut input = vec![0x82, 0x02, 0x1a];
        input.extend(((len - 7) as u32).to_be_bytes());
        input.resize(len, 0);
        input
    }

    #[tokio::test]
    async fn message_of_several_items_decodes_in_one_segment() {
        for len in [64, MAX_SEGMENT_PAYLOAD_LENGTH] {
            let msg = receive_from::<SeveralItems>(vec![several_items(len)]).await;
            assert_eq!(msg.unwrap().0.len(), len - 7);
        }
    }

    #[tokio::test]
    async fn message_of_several_items_may_wait_on_other_splits() {
        let full = MAX_SEGMENT_PAYLOAD_LENGTH;
        for (len, segment) in [(2 * full, full), (64, 16), (50_000, 12_288)] {
            let segments = segments_of(&several_items(len), segment);
            let received = receive_from::<SeveralItems>(segments).await;
            assert_waits(received, &format!("{len} bytes in {segment}-byte segments"));
        }

        // Decoded where the rationed decodes reach its end: after decodes at
        // 16 and 32 bytes, the one at 48 is still due.
        let segments = segments_of(&several_items(48), 16);
        let msg = receive_from::<SeveralItems>(segments).await;
        assert_eq!(msg.unwrap().0.len(), 41);
    }

    #[tokio::test]
    async fn message_declaring_more_items_than_it_holds_may_wait_on_other_splits() {
        // In one full segment.
        let mut input = vec![];
        minicbor::encode(
            CountShort(vec![9; MAX_SEGMENT_PAYLOAD_LENGTH - 5]),
            &mut input,
        )
        .unwrap();
        assert_eq!(input.len(), MAX_SEGMENT_PAYLOAD_LENGTH);
        assert_waits(
            receive_from::<CountShort>(vec![input]).await,
            "one full segment",
        );

        // In several short segments.
        for segments in [
            vec![vec![0x83, 0x02, 0x9f], vec![0x01], vec![0x02, 0xff]],
            vec![vec![0x83, 0x02], vec![0x9f], vec![0xff]],
            vec![vec![0x83], vec![0x02], vec![0x9f], vec![0xff]],
        ] {
            let case = format!("{segments:02x?}");
            assert_waits(receive_from::<CountShortList>(segments).await, &case);
        }

        // Decoded where one early decode is enough.
        let segments = vec![vec![0x83, 0x02, 0x9f], vec![0xff]];
        let msg = receive_from::<CountShortList>(segments).await;
        assert!(msg.unwrap().0.is_empty());
    }

    #[tokio::test]
    async fn limit_counts_the_bytes_the_decoder_reads() {
        // The scanner never sees this item end; the decoder reads all of it.
        let mut input = vec![];
        minicbor::encode(CountShort(vec![9; 1_095]), &mut input).unwrap();
        assert_eq!(input.len(), 1_100);

        let (mut buf, into) = fed_buffer();
        feed_segments(into, vec![input]);
        let err = buf
            .recv_full_msg_with_limit::<CountShort>(1_000)
            .await
            .unwrap_err();
        assert_eq!(seen_in(&err), 1_100);
    }

    #[tokio::test]
    async fn message_the_scanner_finds_malformed_still_decodes() {
        // A break code inside a definite array; the decoder stops before it.
        let segments = vec![vec![0x83, 0x02, 0x40, 0xff]];
        let msg = receive_from::<CountShort>(segments).await;
        assert!(msg.unwrap().0.is_empty());
    }

    #[tokio::test]
    async fn message_stopping_inside_a_later_item_is_refused() {
        use crate::miniprotocols::txsubmission::{EraTxBody, EraTxId, Message};

        // The decoder reads into the second top-level item and stops inside it.
        let segments = vec![
            vec![0x82, 0x01, 0x81, 0x80, 0x84, 0x00],
            vec![0x40, 0x05, 0x07],
        ];
        let err = receive_from::<Message<EraTxId, EraTxBody>>(segments)
            .await
            .unwrap_err();
        assert!(
            format!("{err:?}").contains("ends inside a CBOR item"),
            "{err:?}"
        );
    }

    #[tokio::test]
    async fn stopped_muxer_writes_the_queue_and_lets_the_os_deliver_it() {
        let listener = tcp::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (peer, accepted) =
            tokio::join!(tcp::TcpStream::connect(addr), Bearer::accept_tcp(&listener));
        let mut peer = peer.unwrap();
        let (read, write) = accepted.unwrap().0.into_split();
        let BearerReadHalf::Tcp(read) = read else {
            unreachable!()
        };
        let socket = socket2::SockRef::from(read.as_ref());
        assert_eq!(socket.linger().unwrap(), Some(std::time::Duration::ZERO));

        let mut muxer = Muxer::new(write);
        muxer
            .clone_sender()
            .send((0x8000, vec![1, 2, 3]))
            .await
            .unwrap();
        let (stop_tx, stop) = tokio::sync::mpsc::channel(1);
        stop_tx.send(()).await.unwrap();
        muxer.run_until(stop).await.unwrap();

        assert_eq!(socket.linger().unwrap(), None);

        let mut segment = [0u8; HEADER_LEN + 3];
        peer.read_exact(&mut segment).await.unwrap();
        assert_eq!(&segment[HEADER_LEN..], &[1, 2, 3]);
    }
}
