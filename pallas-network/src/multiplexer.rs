//! A multiplexer of several mini-protocols through a single bearer

use byteorder::{ByteOrder, NetworkEndian};
use pallas_codec::{minicbor, Fragment};
use std::net::SocketAddr;
use std::path::Path;
use thiserror::Error;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream, ToSocketAddrs};
use tokio::select;
use tokio::sync::mpsc::error::SendError;
use tokio::time::Instant;
use tracing::{debug, error, trace};

#[cfg(not(target_os = "windows"))]
use tokio::net::{UnixListener, UnixStream};

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

#[cfg(target_os = "windows")]
pub enum Bearer {
    Tcp(TcpStream),
}

#[cfg(not(target_os = "windows"))]
pub enum Bearer {
    Tcp(TcpStream),
    Unix(UnixStream),
}

const BUFFER_LEN: usize = 1024 * 10;

impl Bearer {
    pub async fn connect_tcp(addr: impl ToSocketAddrs) -> Result<Self, tokio::io::Error> {
        let stream = TcpStream::connect(addr).await?;
        Ok(Self::Tcp(stream))
    }

    pub async fn accept_tcp(listener: &TcpListener) -> tokio::io::Result<(Self, SocketAddr)> {
        let (stream, addr) = listener.accept().await?;
        Ok((Self::Tcp(stream), addr))
    }

    #[cfg(not(target_os = "windows"))]
    pub async fn accept_unix(
        listener: &UnixListener,
    ) -> tokio::io::Result<(Self, tokio::net::unix::SocketAddr)> {
        let (stream, addr) = listener.accept().await?;
        Ok((Self::Unix(stream), addr))
    }

    #[cfg(not(target_os = "windows"))]
    pub async fn connect_unix(path: impl AsRef<Path>) -> Result<Self, tokio::io::Error> {
        let stream = UnixStream::connect(path).await?;
        Ok(Self::Unix(stream))
    }

    pub async fn readable(&self) -> tokio::io::Result<()> {
        match self {
            Bearer::Tcp(x) => x.readable().await,
            #[cfg(not(target_os = "windows"))]
            Bearer::Unix(x) => x.readable().await,
        }
    }

    fn try_read(&mut self, buf: &mut [u8]) -> tokio::io::Result<usize> {
        match self {
            Bearer::Tcp(x) => x.try_read(buf),
            #[cfg(not(target_os = "windows"))]
            Bearer::Unix(x) => x.try_read(buf),
        }
    }

    async fn write_all(&mut self, buf: &[u8]) -> tokio::io::Result<()> {
        match self {
            Bearer::Tcp(x) => x.write_all(buf).await,
            #[cfg(not(target_os = "windows"))]
            Bearer::Unix(x) => x.write_all(buf).await,
        }
    }

    async fn flush(&mut self) -> tokio::io::Result<()> {
        match self {
            Bearer::Tcp(x) => x.flush().await,
            #[cfg(not(target_os = "windows"))]
            Bearer::Unix(x) => x.flush().await,
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
}

pub struct SegmentBuffer(Bearer, Vec<u8>);

impl SegmentBuffer {
    pub fn new(bearer: Bearer) -> Self {
        Self(bearer, Vec::with_capacity(BUFFER_LEN))
    }

    /// Cancel-safe loop that reads from bearer until certain len
    async fn cancellable_read(&mut self, required: usize) -> Result<(), Error> {
        loop {
            self.0.readable().await.map_err(Error::BearerIo)?;
            trace!("bearer is readable");

            let remaining = required - self.1.len();
            let mut buf = vec![0u8; remaining];

            match self.0.try_read(&mut buf) {
                Ok(0) => {
                    error!("empty bearer");
                    break Err(Error::EmptyBearer);
                }
                Ok(n) => {
                    trace!(n, "found data on bearer");
                    self.1.extend_from_slice(&buf[0..n]);

                    if self.1.len() >= required {
                        break Ok(());
                    }
                }
                Err(ref e) if e.kind() == tokio::io::ErrorKind::WouldBlock => {
                    trace!("reading from bearer would block");
                    continue;
                }
                Err(err) => {
                    error!(?err, "beaerer IO error");
                    break Err(Error::BearerIo(err));
                }
            }
        }
    }

    /// Peek the available data in search for a frame header
    async fn peek_header(&mut self) -> Result<Header, Error> {
        trace!("waiting for header buf");
        self.cancellable_read(HEADER_LEN).await?;

        trace!("found enough data for header");
        let header = &self.1[..HEADER_LEN];

        Ok(Header::from(header))
    }

    // Cancel-safe read of a full segment from the bearer
    pub async fn read_segment(&mut self) -> Result<(Protocol, Payload), Error> {
        let header = self.peek_header().await?;

        trace!("waiting for full segment buf");
        let segment_size = HEADER_LEN + header.payload_len as usize;

        self.cancellable_read(segment_size).await?;

        trace!("draining segment buffer");
        let segment = self.1.drain(..segment_size);
        let payload = segment.skip(HEADER_LEN).collect();

        Ok((header.protocol, payload))
    }

    pub async fn write_segment(
        &mut self,
        protocol: u16,
        clock: &Instant,
        payload: &[u8],
    ) -> Result<(), std::io::Error> {
        let header = Header {
            protocol,
            timestamp: clock.elapsed().as_micros() as u32,
            payload_len: payload.len() as u16,
        };

        let buf: [u8; 8] = header.into();
        self.0.write_all(&buf).await?;
        self.0.write_all(payload).await?;

        self.0.flush().await?;

        Ok(())
    }
}

pub struct AgentChannel {
    enqueue_protocol: Protocol,
    dequeue_protocol: Protocol,
    to_plexer: tokio::sync::mpsc::Sender<(Protocol, Payload)>,
    from_plexer: tokio::sync::broadcast::Receiver<(Protocol, Payload)>,
}

impl AgentChannel {
    fn for_client(protocol: Protocol, ingress: &Ingress, egress: &Egress) -> Self {
        Self {
            enqueue_protocol: protocol,
            dequeue_protocol: protocol ^ 0x8000,
            to_plexer: ingress.0.clone(),
            from_plexer: egress.0.subscribe(),
        }
    }

    fn for_server(protocol: Protocol, ingress: &Ingress, egress: &Egress) -> Self {
        Self {
            enqueue_protocol: protocol ^ 0x8000,
            dequeue_protocol: protocol,
            to_plexer: ingress.0.clone(),
            from_plexer: egress.0.subscribe(),
        }
    }

    pub async fn enqueue_chunk(&mut self, chunk: Payload) -> Result<(), Error> {
        self.to_plexer
            .send((self.enqueue_protocol, chunk))
            .await
            .map_err(|SendError((protocol, payload))| Error::AgentEnqueue(protocol, payload))
    }

    pub async fn dequeue_chunk(&mut self) -> Result<Payload, Error> {
        loop {
            let (protocol, payload) = self
                .from_plexer
                .recv()
                .await
                .map_err(|_| Error::AgentDequeue)?;

            if protocol == self.dequeue_protocol {
                trace!(protocol, "message for our protocol");
                break Ok(payload);
            }
        }
    }
}

type Ingress = (
    tokio::sync::mpsc::Sender<(Protocol, Payload)>,
    tokio::sync::mpsc::Receiver<(Protocol, Payload)>,
);

type Egress = (
    tokio::sync::broadcast::Sender<(Protocol, Payload)>,
    tokio::sync::broadcast::Receiver<(Protocol, Payload)>,
);

pub struct Plexer {
    clock: Instant,
    bearer: SegmentBuffer,
    ingress: Ingress,
    egress: Egress,
}

impl Plexer {
    pub fn new(bearer: Bearer) -> Self {
        Self {
            clock: Instant::now(),
            bearer: SegmentBuffer::new(bearer),
            ingress: tokio::sync::mpsc::channel(100), // TODO: define buffer
            egress: tokio::sync::broadcast::channel(100),
        }
    }

    async fn mux(&mut self, msg: (Protocol, Payload)) -> Result<(), Error> {
        self.bearer
            .write_segment(msg.0, &self.clock, &msg.1)
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

    async fn demux(&mut self, protocol: Protocol, payload: Payload) -> Result<(), Error> {
        if tracing::event_enabled!(tracing::Level::TRACE) {
            trace!(protocol, data = hex::encode(&payload), "read from bearer");
        }

        self.egress
            .0
            .send((protocol, payload))
            .map_err(|err| Error::PlexerDemux(err.0 .0, err.0 .1))?;

        Ok(())
    }

    pub fn subscribe_client(&mut self, protocol: Protocol) -> AgentChannel {
        AgentChannel::for_client(protocol, &self.ingress, &self.egress)
    }

    pub fn subscribe_server(&mut self, protocol: Protocol) -> AgentChannel {
        AgentChannel::for_server(protocol, &self.ingress, &self.egress)
    }

    pub async fn run(&mut self) -> Result<(), Error> {
        loop {
            trace!("selecting");
            select! {
                res = self.bearer.read_segment() => {
                    let x = res?;
                    trace!("demux selected");
                    self.demux(x.0, x.1).await?
                },
                Some(x) = self.ingress.1.recv() => {
                    trace!("mux selected");
                    self.mux(x).await?
                },
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(5)) => {
                    trace!("idle plexer");
                }
                else => {
                    error!("something else happened");
                }
            }
        }
    }
}

/// Protocol value that defines max segment length
pub const MAX_SEGMENT_PAYLOAD_LENGTH: usize = 65535;

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

        let ingress = tokio::sync::mpsc::channel(100);
        let egress = tokio::sync::broadcast::channel(100);

        let channel = AgentChannel::for_client(0, &ingress, &egress);

        egress.0.send((0x8000, input)).unwrap();

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

        let ingress = tokio::sync::mpsc::channel(100);
        let egress = tokio::sync::broadcast::channel(100);

        let channel = AgentChannel::for_client(0, &ingress, &egress);

        while !input.is_empty() {
            let chunk = Vec::from(input.drain(0..2).as_slice());
            egress.0.send((0x8000, chunk)).unwrap();
        }

        let mut buf = ChannelBuffer::new(channel);

        let out_msg = buf
            .recv_full_msg::<(u8, u8, u8, u8, u8, u8, u8)>()
            .await
            .unwrap();

        assert_eq!(msg, out_msg);
    }
}
