type IOResult<T> = tokio::io::Result<T>;

use std::collections::HashMap;

use byteorder::{ByteOrder as _, NetworkEndian};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::{net as tcp, select};

#[cfg(unix)]
use tokio::net as unix;

#[cfg(windows)]
use tokio::net::windows::named_pipe::NamedPipeClient;

#[cfg(windows)]
use tokio::io::{ReadHalf, WriteHalf};

use crate::{Channel, Message, Payload};

const HEADER_LEN: usize = 8;

pub type Timestamp = u32;

#[derive(Debug)]
pub struct Header {
    pub protocol: Channel,
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
        sock_ref.set_tcp_nodelay(true)?;

        Ok(())
    }

    pub async fn connect_tcp(addr: impl tcp::ToSocketAddrs) -> Result<Self, tokio::io::Error> {
        let stream = tcp::TcpStream::connect(addr).await?;
        Self::configure_tcp(&stream)?;
        // Aggressive linger avoids TIME_WAIT accumulation when connecting to many nodes
        socket2::SockRef::from(&stream).set_linger(Some(std::time::Duration::from_secs(0)))?;
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

    pub async fn read_segment(&mut self) -> IOResult<(Channel, Payload)> {
        tracing::trace!("waiting for segment header");

        let mut buf = vec![0u8; HEADER_LEN];
        self.read_exact(&mut buf).await?;

        let header = Header::from(buf.as_slice());

        tracing::trace!("waiting for full segment");

        let segment_size = header.payload_len as usize;
        let mut buf = vec![0u8; segment_size];

        self.read_exact(&mut buf).await?;

        Ok((header.protocol, buf))
    }

    /// Reads from the channel until a complete message is found
    pub async fn read_full_msgs<M>(
        &mut self,
        partial_chunks: &mut HashMap<Channel, Payload>,
    ) -> IOResult<Vec<M>>
    where
        M: Message,
    {
        let (raw_channel, chunk) = self.read_segment().await?;
        let channel = raw_channel & !crate::protocol::PROTOCOL_SERVER;

        let previous = partial_chunks.remove(&channel);

        let mut payload = match previous {
            Some(x) => {
                let mut payload = x;
                payload.extend(chunk);
                payload
            }
            None => chunk,
        };

        let mut msgs = Vec::new();

        while let Some(msg) = M::from_payload(channel, &mut payload) {
            msgs.push(msg);
        }

        if !payload.is_empty() {
            tracing::debug!("payload is not empty after successful decode");
            partial_chunks.insert(channel, payload);
        }

        Ok(msgs)
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

    pub async fn write_segment(
        &mut self,
        protocol: u16,
        timestamp: Timestamp,
        payload: &[u8],
    ) -> IOResult<()> {
        let header = Header {
            protocol,
            timestamp,
            payload_len: payload.len() as u16,
        };

        let buf: [u8; 8] = header.into();

        self.write_all(&buf).await?;
        self.write_all(payload).await?;

        self.flush().await?;

        Ok(())
    }

    pub async fn write_message<M>(
        &mut self,
        msg: M,
        timestamp: Timestamp,
        mode: u16,
    ) -> IOResult<()>
    where
        M: Message,
    {
        let (channel, chunks) = msg.into_chunks();
        let channel = channel | mode;

        for chunk in chunks {
            self.write_segment(channel, timestamp, &chunk).await?;
        }

        Ok(())
    }

    pub async fn shutdown(&mut self) -> IOResult<()> {
        match self {
            Self::Tcp(x) => x.shutdown().await,

            #[cfg(unix)]
            Self::Unix(x) => x.shutdown().await,

            #[cfg(windows)]
            Self::NamedPipe(x) => x.shutdown().await,
        }
    }
}
