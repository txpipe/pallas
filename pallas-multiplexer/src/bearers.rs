use byteorder::{ByteOrder, NetworkEndian, WriteBytesExt};
use log::{debug, log_enabled, trace};
use std::io::{Read, Write};
#[cfg(target_family = "unix")]
use std::os::unix::net::UnixStream;
use std::{net::TcpStream, time::Instant};

use crate::Payload;

pub struct Segment {
    pub protocol: u16,
    pub timestamp: u32,
    pub payload: Payload,
}

pub trait Bearer: Read + Write + Send + Sync + Sized {
    type Error: std::error::Error;

    fn read_segment(&mut self) -> Result<Option<Segment>, Self::Error>;

    fn write_segment(&mut self, segment: Segment) -> Result<(), Self::Error>;

    fn clone(&self) -> Self;
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
            _ => todo!(),
        },
    }
}

impl Bearer for TcpStream {
    type Error = std::io::Error;

    fn clone(&self) -> Self {
        self.try_clone().expect("error cloning tcp stream")
    }

    fn read_segment(&mut self) -> Result<Option<Segment>, std::io::Error> {
        read_segment_with_timeout(self)
    }

    fn write_segment(&mut self, segment: Segment) -> Result<(), std::io::Error> {
        write_segment(self, segment)
    }
}

#[cfg(target_family = "unix")]
impl Bearer for UnixStream {
    type Error = std::io::Error;

    fn clone(&self) -> Self {
        self.try_clone().expect("error cloning unix stream")
    }

    fn read_segment(&mut self) -> Result<Option<Segment>, std::io::Error> {
        read_segment_with_timeout(self)
    }

    fn write_segment(&mut self, segment: Segment) -> Result<(), std::io::Error> {
        write_segment(self, segment)
    }
}
