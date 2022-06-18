use byteorder::{ByteOrder, NetworkEndian, WriteBytesExt};
use log::{debug, log_enabled, trace};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, ToSocketAddrs};
use std::{net::TcpStream, time::Instant};

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

    if log_enabled!(log::Level::Trace) {
        trace!(
            "sending segment, header {:?}, protocol id: {}, payload length: {}",
            hex::encode(&msg),
            protocol,
            payload.len()
        );
    }

    msg.write_all(&payload)?;

    writer.write_all(&msg)?;
    writer.flush()
}

fn read_segment(reader: &mut impl Read) -> Result<Segment, std::io::Error> {
    let mut header = [0u8; 8];

    reader.read_exact(&mut header)?;

    if log_enabled!(log::Level::Trace) {
        trace!("read segment header: {:?}", hex::encode(&header));
    }

    let length = NetworkEndian::read_u16(&header[6..]) as usize;
    let protocol = NetworkEndian::read_u16(&header[4..6]) as usize ^ 0x8000;
    let timestamp = NetworkEndian::read_u32(&header[0..4]);

    debug!(
        "parsed inbound msg, protocol id: {}, ts: {}, payload length: {}",
        protocol, timestamp, length
    );

    let mut payload = vec![0u8; length];
    reader.read_exact(&mut payload)?;

    if log_enabled!(log::Level::Trace) {
        trace!("read segment payload: {:?}", hex::encode(&payload));
    }

    Ok(Segment {
        protocol: protocol as u16,
        timestamp,
        payload,
    })
}

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

    pub fn accept_tcp(server: TcpListener) -> Result<Self, std::io::Error> {
        let (bearer, _) = server.accept().unwrap();
        bearer.set_nodelay(true)?;

        Ok(Bearer::Tcp(bearer))
    }

    #[cfg(target_family = "unix")]
    pub fn connect_unix<P: AsRef<std::path::Path>>(path: P) -> Result<Self, std::io::Error> {
        let bearer = UnixStream::connect(path)?;

        Ok(Bearer::Unix(bearer))
    }

    pub fn read_segment(&mut self) -> Result<Option<Segment>, std::io::Error> {
        match self {
            Bearer::Tcp(s) => read_segment_with_timeout(s),

            #[cfg(target_family = "unix")]
            Bearer::Unix(s) => read_segment_with_timeout(s),
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
