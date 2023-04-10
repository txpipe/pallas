use std::net::SocketAddr;
use std::path::Path;

use byteorder::{ByteOrder, NetworkEndian};
use thiserror::Error;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream, ToSocketAddrs, UnixStream};
use tokio::time::Instant;
use tracing::trace;

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
    Tcp(TcpStream),
    Unix(UnixStream),
}

const BUFFER_LEN: usize = 1024 * 10;

impl Bearer {
    pub async fn connect_tcp(addr: impl ToSocketAddrs) -> Result<Self, tokio::io::Error> {
        let stream = TcpStream::connect(addr).await?;
        Ok(Self::Tcp(stream))
    }

    pub async fn accept_tcp(listener: TcpListener) -> tokio::io::Result<(Self, SocketAddr)> {
        let (stream, addr) = listener.accept().await?;
        Ok((Self::Tcp(stream), addr))
    }

    pub async fn connect_unix(path: impl AsRef<Path>) -> Result<Self, tokio::io::Error> {
        let stream = UnixStream::connect(path).await?;
        Ok(Self::Unix(stream))
    }

    pub async fn readable(&self) -> tokio::io::Result<()> {
        match self {
            Bearer::Tcp(x) => x.readable().await,
            Bearer::Unix(x) => x.readable().await,
        }
    }

    fn try_read(&mut self, buf: &mut [u8]) -> tokio::io::Result<usize> {
        match self {
            Bearer::Tcp(x) => x.try_read(buf),
            Bearer::Unix(x) => x.try_read(buf),
        }
    }

    async fn write_all(&mut self, buf: &[u8]) -> tokio::io::Result<()> {
        match self {
            Bearer::Tcp(x) => x.write_all(buf).await,
            Bearer::Unix(x) => x.write_all(buf).await,
        }
    }

    async fn flush(&mut self) -> tokio::io::Result<()> {
        match self {
            Bearer::Tcp(x) => x.flush().await,
            Bearer::Unix(x) => x.flush().await,
        }
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("no data available in bearer to complete segment")]
    NoData,

    #[error("unexpected I/O error")]
    Io(#[source] tokio::io::Error),
}

pub struct SegmentBuffer(Bearer, Vec<u8>);

impl SegmentBuffer {
    pub fn new(bearer: Bearer) -> Self {
        Self(bearer, Vec::with_capacity(BUFFER_LEN))
    }

    /// Cancel-safe loop that reads from bearer until certain len
    async fn cancellable_read(&mut self, required: usize) -> Result<(), Error> {
        loop {
            self.0.readable().await.map_err(Error::Io)?;
            trace!("bearer is readable");

            let remaining = required - self.1.len();
            let mut buf = vec![0u8; remaining];

            match self.0.try_read(&mut buf) {
                Ok(0) => break Err(Error::NoData),
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
                Err(e) => {
                    return Err(Error::Io(e));
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
        self.0.write_all(&payload).await?;

        self.0.flush().await?;

        Ok(())
    }
}
