use std::net::SocketAddr;
use std::path::Path;

use byteorder::{ByteOrder, NetworkEndian};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufStream};
use tokio::net::{TcpListener, TcpStream, ToSocketAddrs, UnixStream};
use tokio::time::Instant;

const HEADER_LEN: usize = 8;

pub type Timestamp = u32;

pub type Payload = Vec<u8>;

pub type Protocol = u16;

/// A `Header` struct represents an Ouroboros segment header.
///
/// # Examples
///
/// Converting a `Header` to bytes:
///
/// ```
/// use byteorder::{BigEndian, ByteOrder};
/// use pallas_upstream::plexer::Header;
///
/// let header = Header {
///     protocol: 0x01,
///     timestamp: 1619804871,
///     payload_len: 42,
/// };
///
/// let header_bytes: [u8; 8] = header.into();
/// assert_eq!(header_bytes, [97, 75, 168, 15, 128, 1, 0, 42]);
/// ```
///
/// Converting bytes to a `Header`:
///
/// ```
/// use byteorder::{BigEndian, ByteOrder};
/// use pallas_upstream::plexer::Header;
///
/// let bytes = [97, 75, 168, 15, 128, 1, 0, 42];
/// let header: Header = (&bytes[..]).into();
///
/// assert_eq!(header.protocol, 0x01);
/// assert_eq!(header.timestamp, 1619804871);
/// assert_eq!(header.payload_len, 42);
/// ```
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
    Tcp(BufStream<TcpStream>, Instant),
    Unix(BufStream<UnixStream>, Instant),
}

impl Bearer {
    pub async fn connect_tcp(addr: impl ToSocketAddrs) -> Result<Self, tokio::io::Error> {
        let stream = TcpStream::connect(addr).await?;
        let buf = BufStream::new(stream);
        Ok(Self::Tcp(buf, Instant::now()))
    }

    pub async fn accept_tcp(listener: TcpListener) -> tokio::io::Result<(Self, SocketAddr)> {
        let (stream, addr) = listener.accept().await?;
        let buf = BufStream::new(stream);
        Ok((Self::Tcp(buf, Instant::now()), addr))
    }

    pub async fn connect_unix(path: impl AsRef<Path>) -> Result<Self, tokio::io::Error> {
        let stream = UnixStream::connect(path).await?;
        let buf = BufStream::new(stream);
        Ok(Self::Unix(buf, Instant::now()))
    }

    fn clock(&self) -> &Instant {
        match self {
            Bearer::Tcp(_, x) => x,
            Bearer::Unix(_, x) => x,
        }
    }

    pub async fn readable(&self) -> tokio::io::Result<()> {
        match self {
            Bearer::Tcp(x, _) => x.get_ref().readable().await,
            Bearer::Unix(x, _) => x.get_ref().readable().await,
        }
    }

    async fn fill_buf(&mut self) -> tokio::io::Result<&[u8]> {
        match self {
            Bearer::Tcp(x, _) => x.fill_buf().await,
            Bearer::Unix(x, _) => x.fill_buf().await,
        }
    }

    async fn read_exact(&mut self, buf: &mut [u8]) -> tokio::io::Result<usize> {
        match self {
            Bearer::Tcp(x, _) => x.read_exact(buf).await,
            Bearer::Unix(x, _) => x.read_exact(buf).await,
        }
    }

    async fn write_all(&mut self, buf: &[u8]) -> tokio::io::Result<()> {
        match self {
            Bearer::Tcp(x, _) => {
                println!("writing: {}", hex::encode(buf));
                x.write_all(buf).await
            }
            Bearer::Unix(x, _) => x.write_all(buf).await,
        }
    }

    async fn flush(&mut self) -> tokio::io::Result<()> {
        match self {
            Bearer::Tcp(x, _) => x.flush().await,
            Bearer::Unix(x, _) => x.flush().await,
        }
    }

    /// Peek the available data in search for a frame header
    async fn peek_header(&mut self) -> tokio::io::Result<Option<Header>> {
        let temp = self.fill_buf().await?;

        if temp.is_empty() {
            panic!("unexpected eof");
        }

        if temp.len() < HEADER_LEN {
            println!("len: {}, {}", temp.len(), hex::encode(temp));
            return Ok(None);
        }

        let header = &temp[..HEADER_LEN];

        Ok(Some(Header::from(header)))
    }

    async fn has_payload(&mut self, payload_len: usize) -> tokio::io::Result<bool> {
        let segment_size = HEADER_LEN + payload_len;

        let temp = self.fill_buf().await?;

        if temp.is_empty() {
            panic!("unexpected eof");
        }

        return Ok(temp.len() >= segment_size);
    }

    /// Peeks the bearer to see if a full segment is available to be read
    pub async fn has_segment(&mut self) -> std::io::Result<bool> {
        let header = match self.peek_header().await? {
            Some(x) => x,
            None => return Ok(false),
        };

        self.has_payload(header.payload_len as usize).await
    }

    /// Reads a full segment from the bearer while consuming the bytes
    ///
    /// This function is NOT "cancel safe", meaning that it shouldn't be used
    /// inside the context of a select!. Only call this function once you're
    /// sure that you can await until all the required bytes are available.
    pub async fn read_segment(&mut self) -> tokio::io::Result<(Protocol, Payload)> {
        let mut buf = [0u8; HEADER_LEN];
        self.read_exact(&mut buf).await?;
        println!("read header: {}", hex::encode(buf));

        let header = Header::from(buf.as_slice());

        // TODO: assert any business invariants regarding timestamp from the other party

        let mut payload = vec![0u8; header.payload_len as usize];
        self.read_exact(&mut payload).await?;

        Ok((header.protocol, payload))
    }

    pub async fn write_segment(
        &mut self,
        protocol: u16,
        payload: &[u8],
    ) -> Result<(), std::io::Error> {
        let header = Header {
            protocol,
            timestamp: self.clock().elapsed().as_micros() as u32,
            payload_len: payload.len() as u16,
        };

        let buf: [u8; 8] = header.into();
        self.write_all(&buf).await?;
        self.write_all(&payload).await?;

        self.flush().await?;

        Ok(())
    }
}
