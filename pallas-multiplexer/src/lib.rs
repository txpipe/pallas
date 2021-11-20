use std::{
    collections::HashMap,
    io::{Read, Write},
    net::TcpStream,
    sync::mpsc::{self, Receiver, Sender, TryRecvError},
    thread,
    time::{Duration, Instant},
};

use byteorder::{ByteOrder, NetworkEndian, WriteBytesExt};
use log::{debug, error, log_enabled, trace, warn};

pub trait Bearer: Read + Write + Send + Sync + Sized {
    fn read_segment(&mut self) -> Result<(u16, u32, Payload), std::io::Error>;

    fn write_segment(
        &mut self,
        clock: Instant,
        protocol_id: u16,
        partial_payload: &[u8],
    ) -> Result<(), std::io::Error>;

    fn clone(&self) -> Self;
}

impl Bearer for TcpStream {
    fn write_segment(
        &mut self,
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

        msg.write(&payload[..]).unwrap();

        self.write(&msg)?;

        self.flush()
    }

    fn read_segment(&mut self) -> Result<(u16, u32, Payload), std::io::Error> {
        let mut header = [0u8; 8];

        self.read_exact(&mut header)?;

        let length = NetworkEndian::read_u16(&header[6..]) as usize;
        let mut payload = vec![0u8; length];
        self.read_exact(&mut payload)?;

        let id = NetworkEndian::read_u16(&mut header[4..6]) as usize ^ 0x8000;
        let ts = NetworkEndian::read_u32(&mut header[0..4]);

        if log_enabled!(log::Level::Trace) {
            trace!(
                "received segment, header: {:?}, payload length: {}",
                hex::encode(&header),
                payload.len()
            );
        }

        Ok((id as u16, ts, payload))
    }

    fn clone(&self) -> Self {
        self.try_clone().unwrap()
    }
}

const MAX_SEGMENT_PAYLOAD_LENGTH: usize = 65535;

pub type Payload = Vec<u8>;

#[derive(Debug)]
pub struct Error {}

fn tx_round<TBearer>(
    bearer: &mut TBearer,
    ingress: &MuxIngress,
    clock: Instant,
) -> Result<u16, std::io::Error>
where
    TBearer: Bearer,
{
    let mut writes = 0u16;

    for (id, rx) in ingress.iter() {
        match rx.try_recv() {
            Ok(payload) => {
                let chunks = payload.chunks(MAX_SEGMENT_PAYLOAD_LENGTH);

                for chunk in chunks {
                    bearer.write_segment(clock, *id, chunk)?;
                    writes += 1;
                }
            }
            Err(TryRecvError::Disconnected) => {
                //TODO: remove handle from list
                warn!("protocol handle disconnected");
            }
            Err(TryRecvError::Empty) => (),
        };
    }

    Ok(writes)
}

fn tx_loop<TBearer>(bearer: &mut TBearer, ingress: MuxIngress)
where
    TBearer: Bearer,
{
    loop {
        let clock = Instant::now();
        match tx_round(bearer, &ingress, clock) {
            Err(err) => {
                error!("{:?}", err);
                panic!();
            }
            Ok(0) => thread::sleep(Duration::from_millis(10)),
            Ok(_) => (),
        };
    }
}

fn rx_loop<TBearer>(bearer: &mut TBearer, egress: DemuxerEgress)
where
    TBearer: Bearer,
{
    let mut tx_map: HashMap<_, _> = egress.into_iter().collect();

    loop {
        match bearer.read_segment() {
            Err(err) => {
                error!("{:?}", err);
                panic!();
            }
            Ok(segment) => {
                let (id, _ts, payload) = segment;
                match tx_map.get(&id) {
                    Some(tx) => match tx.send(payload) {
                        Err(err) => {
                            error!("error sending egress tx to protocol, removing protocol from egress output. {:?}", err);
                            tx_map.remove(&id);
                        }
                        Ok(_) => {
                            debug!("successful tx to egress protocol");
                        }
                    },
                    None => warn!(
                        "received segment for protocol id not being demuxed {}",
                        id
                    ),
                }
            }
        }
    }
}

type ChannelProtocolHandle = (u16, Receiver<Payload>, Sender<Payload>);
type ChannelIngressHandle = (u16, Receiver<Payload>);
type ChannelEgressHandle = (u16, Sender<Payload>);
type MuxIngress = Vec<ChannelIngressHandle>;
type DemuxerEgress = Vec<ChannelEgressHandle>;

pub struct Multiplexer {}

impl Multiplexer {
    pub fn new<TBearer>(
        bearer: TBearer,
        protocols: &[u16],
    ) -> Result<Vec<ChannelProtocolHandle>, Error>
    where
        TBearer: Bearer + 'static,
    {
        let handles = protocols
            .iter()
            .map(|id| {
                let (demux_tx, demux_rx) = mpsc::channel::<Payload>();
                let (mux_tx, mux_rx) = mpsc::channel::<Payload>();

                let protocol_handle: ChannelProtocolHandle =
                    (*id, demux_rx, mux_tx);
                let ingress_handle: ChannelIngressHandle = (*id, mux_rx);
                let egress_handle: ChannelEgressHandle = (*id, demux_tx);

                (protocol_handle, (ingress_handle, egress_handle))
            })
            .collect::<Vec<_>>();

        let (protocol_handles, multiplex_handles): (Vec<_>, Vec<_>) =
            handles.into_iter().unzip();

        let (ingress, egress): (Vec<_>, Vec<_>) =
            multiplex_handles.into_iter().unzip();

        let mut tx_bearer = bearer.clone();
        thread::spawn(move || tx_loop(&mut tx_bearer, ingress));

        let mut rx_bearer = bearer.clone();
        thread::spawn(move || rx_loop(&mut rx_bearer, egress));

        Ok(protocol_handles)
    }
}
