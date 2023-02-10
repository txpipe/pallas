use byteorder::{ByteOrder, NetworkEndian, WriteBytesExt};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, ToSocketAddrs};
use std::{net::TcpStream, time::Instant};
use tracing::{debug, event_enabled, trace};

use crate::Payload;

#[cfg(target_family = "unix")]
use std::os::unix::net::UnixStream;
use std::time::Duration;

pub struct Segment {
    pub protocol: u16,
    pub timestamp: u32,
    pub payload: Payload,
}
impl Segment {
    pub fn new(clock: Instant, protocol: u16, payload: Payload) -> Self {
        Segment {
            timestamp: clock.elapsed().as_micros() as u32,
            protocol,
            payload,
        }
    }
}

fn write_segment(writer: &mut impl Write, segment: Segment) -> Result<(), std::io::Error> {
    let Segment {
        timestamp,
        protocol,
        payload,
    } = segment;

    let mut msg = Vec::new();
    msg.write_u32::<NetworkEndian>(timestamp)?;
    msg.write_u16::<NetworkEndian>(protocol)?;
    msg.write_u16::<NetworkEndian>(payload.len() as u16)?;
    msg.write_all(&payload)?;

    if event_enabled!(tracing::Level::TRACE) {
        trace!(
            protocol,
            length = payload.len(),
            message = hex::encode(&msg),
            "writing segment"
        );
    }

    writer.write_all(&msg)?;
    writer.flush()
}

fn read_segment(reader: &mut impl Read) -> Result<Segment, std::io::Error> {
    let mut header = [0u8; 8];

    reader.read_exact(&mut header)?;

    if event_enabled!(tracing::Level::TRACE) {
        trace!(header = hex::encode(header), "segment header read");
    }

    let length = NetworkEndian::read_u16(&header[6..]) as usize;
    let protocol = NetworkEndian::read_u16(&header[4..6]) as usize ^ 0x8000;
    let timestamp = NetworkEndian::read_u32(&header[0..4]);

    debug!(protocol, timestamp, length, "parsed inbound msg");

    let mut payload = vec![0u8; length];
    reader.read_exact(&mut payload)?;

    if event_enabled!(tracing::Level::TRACE) {
        trace!(payload = hex::encode(&payload), "segment payload read");
    }

    Ok(Segment {
        protocol: protocol as u16,
        timestamp,
        payload,
    })
}

// This snippet will be useful if we want to switch TCP streams into
// non-blocking mode, but that's not likely (if we want async, we'll probably go
// with Tokio instead of a handcrafted approach).
/*
fn read_segment_with_timeout(reader: &mut impl Read) -> Result<Option<Segment>, std::io::Error> {
    match read_segment(reader) {
        Ok(s) => Ok(Some(s)),
        Err(err) => match err.kind() {
            std::io::ErrorKind::WouldBlock => Ok(None),
            std::io::ErrorKind::TimedOut => Ok(None),
            std::io::ErrorKind::Interrupted => Ok(None),
            _ => Err(err),
        },
    }
}
 */

#[derive(Debug)]
pub enum Bearer {
    Tcp(TcpStream),

    #[cfg(target_family = "unix")]
    Unix(UnixStream),
}

impl Bearer {
    pub fn connect_tcp<A: ToSocketAddrs>(addr: A) -> Result<Self, std::io::Error> {
        let bearer = TcpStream::connect(addr)?;
        bearer.set_nodelay(true)?;

        Ok(Bearer::Tcp(bearer))
    }

    pub fn connect_tcp_timeout(
        addr: &SocketAddr,
        timeout: Duration,
    ) -> Result<Self, std::io::Error> {
        let bearer = TcpStream::connect_timeout(addr, timeout)?;
        bearer.set_nodelay(true)?;

        Ok(Bearer::Tcp(bearer))
    }

    pub fn accept_tcp(server: TcpListener) -> Result<(Self, SocketAddr), std::io::Error> {
        let (bearer, remote_addr) = server.accept().unwrap();
        bearer.set_nodelay(true)?;

        Ok((Bearer::Tcp(bearer), remote_addr))
    }

    #[cfg(target_family = "unix")]
    pub fn connect_unix<P: AsRef<std::path::Path>>(path: P) -> Result<Self, std::io::Error> {
        let bearer = UnixStream::connect(path)?;

        Ok(Bearer::Unix(bearer))
    }

    pub fn read_segment(&mut self) -> Result<Option<Segment>, std::io::Error> {
        match self {
            Bearer::Tcp(s) => {
                // std tcp streams won't be supporting timeout / async. We don't handle
                // specific timeout-related errors, these will remain unhandled and bubble up
                // to the consumer lib. The Option wrapper is here just for compatiblity with
                // other future bearers that might support timeouts
                read_segment(s).map(Some)
            }

            #[cfg(target_family = "unix")]
            Bearer::Unix(s) => read_segment(s).map(Some),
        }
    }

    pub fn write_segment(&mut self, segment: Segment) -> Result<(), std::io::Error> {
        match self {
            Bearer::Tcp(s) => write_segment(s, segment),

            #[cfg(target_family = "unix")]
            Bearer::Unix(s) => write_segment(s, segment),
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

impl Clone for Bearer {
    fn clone(&self) -> Self {
        match self {
            Bearer::Tcp(s) => Bearer::Tcp(s.try_clone().expect("error cloning tcp stream")),

            #[cfg(target_family = "unix")]
            Bearer::Unix(s) => Bearer::Unix(s.try_clone().expect("error cloning unix stream")),
        }
    }
}
