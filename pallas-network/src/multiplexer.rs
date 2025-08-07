//! A multiplexer of several mini-protocols through a single bearer

use std::collections::HashMap;

use byteorder::{ByteOrder, NetworkEndian};
use pallas_codec::{minicbor, Fragment};
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

pub type Timestamp = u32;

pub type Payload = Vec<u8>;

pub type Protocol = u16;

#[derive(Debug)]
pub struct Header {
    pub protocol: Protocol,
    pub timestamp: Timestamp,
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

pub struct Segment {
    pub header: Header,
    pub payload: Payload,
}

pub enum Bearer {
    Tcp(tcp::TcpStream),

    #[cfg(unix)]
    Unix(unix::UnixStream),

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
        sock_ref.set_nodelay(true)?;
        sock_ref.set_linger(Some(std::time::Duration::from_secs(0)))?;

        Ok(())
    }

    pub async fn connect_tcp(addr: impl tcp::ToSocketAddrs) -> Result<Self, tokio::io::Error> {
        let stream = tcp::TcpStream::connect(addr).await?;
        Self::configure_tcp(&stream)?;
        Ok(Self::Tcp(stream))
    }

    pub async fn connect_tcp_timeout(
        addr: impl tcp::ToSocketAddrs,
        timeout: std::time::Duration,
    ) -> IOResult<Self> {
        select! {
            result = Self::connect_tcp(addr) => result,
            _ = tokio::time::sleep(timeout) => Err(tokio::io::Error::new(tokio::io::ErrorKind::TimedOut, "connect timeout")),
        }
    }

    pub async fn accept_tcp(listener: &tcp::TcpListener) -> IOResult<(Self, std::net::SocketAddr)> {
        let (stream, addr) = listener.accept().await?;
        Self::configure_tcp(&stream)?;
        Ok((Self::Tcp(stream), addr))
    }

    #[cfg(unix)]
    pub async fn connect_unix(path: impl AsRef<std::path::Path>) -> IOResult<Self> {
        let stream = unix::UnixStream::connect(path).await?;
        Ok(Self::Unix(stream))
    }

    #[cfg(unix)]
    pub async fn accept_unix(
        listener: &unix::UnixListener,
    ) -> IOResult<(Self, unix::unix::SocketAddr)> {
        let (stream, addr) = listener.accept().await?;
        Ok((Self::Unix(stream), addr))
    }

    #[cfg(windows)]
    pub fn connect_named_pipe(pipe_name: impl AsRef<std::ffi::OsStr>) -> IOResult<Self> {
        let client = tokio::net::windows::named_pipe::ClientOptions::new().open(&pipe_name)?;
        Ok(Self::NamedPipe(client))
    }

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

pub enum BearerReadHalf {
    Tcp(tcp::tcp::OwnedReadHalf),

    #[cfg(unix)]
    Unix(unix::unix::OwnedReadHalf),

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

pub enum BearerWriteHalf {
    Tcp(tcp::tcp::OwnedWriteHalf),

    #[cfg(unix)]
    Unix(unix::unix::OwnedWriteHalf),

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
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("no data available in bearer to complete segment")]
    EmptyBearer,

    #[error("bearer I/O error")]
    BearerIo(tokio::io::Error),

    #[error("failure to encode channel message")]
    Decoding(String),

    #[error("failure to decode channel message")]
    Encoding(String),

    #[error("agent failed to enqueue chunk for protocol {0}")]
    AgentEnqueue(Protocol, Payload),

    #[error("agent failed to dequeue chunk")]
    AgentDequeue,

    #[error("plexer failed to dumux chunk for protocol {0}")]
    PlexerDemux(Protocol, Payload),

    #[error("plexer failed to mux chunk")]
    PlexerMux,

    #[error("failure to abort the plexer threads")]
    AbortFailure,
}

type EgressChannel = tokio::sync::mpsc::Sender<Payload>;
type Egress = HashMap<Protocol, EgressChannel>;

const EGRESS_MSG_QUEUE_BUFFER: usize = 100;

pub struct Demuxer(BearerReadHalf, Egress);

impl Demuxer {
    pub fn new(bearer: BearerReadHalf) -> Self {
        let egress = HashMap::new();
        Self(bearer, egress)
    }

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

    pub fn subscribe(&mut self, protocol: Protocol) -> tokio::sync::mpsc::Receiver<Payload> {
        let (sender, recv) = tokio::sync::mpsc::channel(EGRESS_MSG_QUEUE_BUFFER);

        // keep track of the sender
        self.1.insert(protocol, sender);

        // return the receiver for the agent
        recv
    }

    pub async fn tick(&mut self) -> Result<(), Error> {
        let (protocol, payload) = self.read_segment().await?;
        trace!(protocol, "demux happening");
        self.demux(protocol, payload).await
    }

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

pub struct Muxer(BearerWriteHalf, Clock, Ingress);

impl Muxer {
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

    pub fn clone_sender(&self) -> tokio::sync::mpsc::Sender<(Protocol, Payload)> {
        self.2 .0.clone()
    }

    pub async fn tick(&mut self) -> Result<(), Error> {
        let msg = self.2 .1.recv().await;

        if let Some(x) = msg {
            trace!(protocol = x.0, "mux happening");
            self.mux(x).await?
        }

        Ok(())
    }

    pub async fn run(&mut self) -> Result<(), Error> {
        loop {
            if let Err(err) = self.tick().await {
                break Err(err);
            }
        }
    }
}

type ToPlexerPort = tokio::sync::mpsc::Sender<(Protocol, Payload)>;
type FromPlexerPort = tokio::sync::mpsc::Receiver<Payload>;

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

    pub async fn enqueue_chunk(&mut self, chunk: Payload) -> Result<(), Error> {
        self.to_plexer
            .send((self.protocol, chunk))
            .await
            .map_err(|SendError((protocol, payload))| Error::AgentEnqueue(protocol, payload))
    }

    pub async fn dequeue_chunk(&mut self) -> Result<Payload, Error> {
        self.from_plexer.recv().await.ok_or(Error::AgentDequeue)
    }
}

pub struct RunningPlexer {
    demuxer: JoinHandle<Result<(), Error>>,
    muxer: JoinHandle<Result<(), Error>>,
}

impl RunningPlexer {
    pub async fn abort(self) {
        self.demuxer.abort();
        self.muxer.abort();
    }
}

pub struct Plexer {
    demuxer: Demuxer,
    muxer: Muxer,
}

impl Plexer {
    pub fn new(bearer: Bearer) -> Self {
        let (r, w) = bearer.into_split();

        Self {
            demuxer: Demuxer::new(r),
            muxer: Muxer::new(w),
        }
    }

    pub fn subscribe_client(&mut self, protocol: Protocol) -> AgentChannel {
        let to_plexer = self.muxer.clone_sender();
        let from_plexer = self.demuxer.subscribe(protocol ^ 0x8000);
        AgentChannel::for_client(protocol, to_plexer, from_plexer)
    }

    pub fn subscribe_server(&mut self, protocol: Protocol) -> AgentChannel {
        let to_plexer = self.muxer.clone_sender();
        let from_plexer = self.demuxer.subscribe(protocol);
        AgentChannel::for_server(protocol ^ 0x8000, to_plexer, from_plexer)
    }

    pub fn spawn(self) -> RunningPlexer {
        let mut demuxer = self.demuxer;
        let mut muxer = self.muxer;

        let demuxer = tokio::spawn(async move { demuxer.run().await });
        let muxer = tokio::spawn(async move { muxer.run().await });

        RunningPlexer { demuxer, muxer }
    }
}

/// Protocol value that defines max segment length
#[deprecated(
    note = "Use `pallas_primitives::types::network_constant::MAX_SEGMENT_PAYLOAD_LENGTH` instead"
)]
pub const MAX_SEGMENT_PAYLOAD_LENGTH: usize =
    pallas_primitives::types::network_constant::MAX_SEGMENT_PAYLOAD_LENGTH;

fn try_decode_message<M>(buffer: &mut Vec<u8>) -> Result<Option<M>, Error>
where
    M: Fragment,
{
    let mut decoder = minicbor::Decoder::new(buffer);
    let maybe_msg = decoder.decode();

    match maybe_msg {
        Ok(msg) => {
            let pos = decoder.position();
            buffer.drain(0..pos);
            Ok(Some(msg))
        }
        Err(err) if err.is_end_of_input() => Ok(None),
        Err(err) => {
            error!(?err);
            trace!("{}", hex::encode(buffer));
            Err(Error::Decoding(err.to_string()))
        }
    }
}

/// A channel abstraction to hide the complexity of partial payloads
pub struct ChannelBuffer {
    channel: AgentChannel,
    temp: Vec<u8>,
}

impl ChannelBuffer {
    pub fn new(channel: AgentChannel) -> Self {
        Self {
            channel,
            temp: Vec::new(),
        }
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
    pub async fn recv_full_msg<M>(&mut self) -> Result<M, Error>
    where
        M: Fragment,
    {
        trace!(len = self.temp.len(), "waiting for full message");

        if !self.temp.is_empty() {
            trace!("buffer has data from previous payload");

            if let Some(msg) = try_decode_message::<M>(&mut self.temp)? {
                debug!("decoding done");
                return Ok(msg);
            }
        }

        loop {
            let chunk = self.channel.dequeue_chunk().await?;
            self.temp.extend(chunk);

            if let Some(msg) = try_decode_message::<M>(&mut self.temp)? {
                debug!("decoding done");
                return Ok(msg);
            }

            trace!("not enough data");
        }
    }

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
}
