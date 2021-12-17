use byteorder::{ByteOrder, NetworkEndian, WriteBytesExt};
use log::{debug, log_enabled, trace};
use std::io::{Read, Write};
#[cfg(target_family = "unix")]
use std::os::unix::net::UnixStream;
use std::{net::TcpStream, time::Instant};

use crate::{Bearer, Payload};

fn write_segment(
    writer: &mut impl Write,
    clock: Instant,
    protocol_id: u16,
    payload: &[u8],
) -> Result<(), std::io::Error> {
    let mut msg = Vec::new();
    msg.write_u32::<NetworkEndian>(clock.elapsed().as_micros() as u32)?;
    msg.write_u16::<NetworkEndian>(protocol_id)?;
    msg.write_u16::<NetworkEndian>(payload.len() as u16)?;

    if log_enabled!(log::Level::Trace) {
        trace!(
            "sending segment, header {:?}, payload length: {}",
            hex::encode(&msg),
            payload.len()
        );
    }

    msg.write_all(payload)?;

    writer.write_all(&msg)?;
    writer.flush()
}

fn read_segment(reader: &mut impl Read) -> Result<(u16, u32, Payload), std::io::Error> {
    let mut header = [0u8; 8];

    reader.read_exact(&mut header)?;

    if log_enabled!(log::Level::Trace) {
        trace!("read segment header: {:?}", hex::encode(&header));
    }

    let length = NetworkEndian::read_u16(&header[6..]) as usize;
    let id = NetworkEndian::read_u16(&header[4..6]) as usize ^ 0x8000;
    let ts = NetworkEndian::read_u32(&header[0..4]);

    debug!(
        "parsed inbound msg, protocol id: {}, ts: {}, payload length: {}",
        id, ts, length
    );

    let mut payload = vec![0u8; length];
    reader.read_exact(&mut payload)?;

    if log_enabled!(log::Level::Trace) {
        trace!("read segment payload: {:?}", hex::encode(&payload));
    }

    Ok((id as u16, ts, payload))
}

impl Bearer for TcpStream {
    fn clone(&self) -> Self {
        self.try_clone().unwrap()
    }

    fn read_segment(&mut self) -> Result<(u16, u32, Payload), std::io::Error> {
        read_segment(self)
    }

    fn write_segment(
        &mut self,
        clock: Instant,
        protocol_id: u16,
        partial_payload: &[u8],
    ) -> Result<(), std::io::Error> {
        write_segment(self, clock, protocol_id, partial_payload)
    }
}

#[cfg(target_family = "unix")]
impl Bearer for UnixStream {
    fn clone(&self) -> Self {
        self.try_clone().unwrap()
    }

    fn read_segment(&mut self) -> Result<(u16, u32, Payload), std::io::Error> {
        read_segment(self)
    }

    fn write_segment(
        &mut self,
        clock: Instant,
        protocol_id: u16,
        partial_payload: &[u8],
    ) -> Result<(), std::io::Error> {
        write_segment(self, clock, protocol_id, partial_payload)
    }
}
