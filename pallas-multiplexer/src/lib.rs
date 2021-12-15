mod bearers;

use std::{
    collections::HashMap,
    io::{Read, Write},
    sync::mpsc::{self, Receiver, Sender, TryRecvError},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use log::{debug, error, trace, warn};

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

const MAX_SEGMENT_PAYLOAD_LENGTH: usize = 65535;

pub type Payload = Vec<u8>;

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
                trace!("protocol handle {} disconnected", id);
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
                    None => warn!("received segment for protocol id not being demuxed {}", id),
                }
            }
        }
    }
}

pub struct Channel(pub Sender<Payload>, pub Receiver<Payload>);

type ChannelProtocolHandle = (u16, Channel);
type ChannelIngressHandle = (u16, Receiver<Payload>);
type ChannelEgressHandle = (u16, Sender<Payload>);
type MuxIngress<'a> = &'a [ChannelIngressHandle];
type DemuxerEgress = Vec<ChannelEgressHandle>;

pub struct Multiplexer {
    tx_thread: JoinHandle<()>,
    rx_thread: JoinHandle<()>,
    io_handles: HashMap<u16, Channel>,
}

impl Multiplexer {
    pub fn setup<TBearer>(
        bearer: TBearer,
        protocols: &[u16],
    ) -> Result<Multiplexer, Box<dyn std::error::Error>>
    where
        TBearer: Bearer + 'static,
    {
        let handles = protocols.iter().map(|id| {
            let (demux_tx, demux_rx) = mpsc::channel::<Payload>();
            let (mux_tx, mux_rx) = mpsc::channel::<Payload>();

            let channel = Channel(mux_tx, demux_rx);

            let protocol_handle: ChannelProtocolHandle = (*id, channel);
            let ingress_handle: ChannelIngressHandle = (*id, mux_rx);
            let egress_handle: ChannelEgressHandle = (*id, demux_tx);

            (protocol_handle, (ingress_handle, egress_handle))
        });

        let (protocol_handles, multiplex_handles): (Vec<_>, Vec<_>) = handles.into_iter().unzip();

        let (ingress, egress): (Vec<_>, Vec<_>) = multiplex_handles.into_iter().unzip();

        let mut tx_bearer = bearer.clone();
        let tx_thread = thread::spawn(move || tx_loop(&mut tx_bearer, ingress.as_slice()));

        let mut rx_bearer = bearer.clone();
        let rx_thread = thread::spawn(move || rx_loop(&mut rx_bearer, egress));

        let io_handles: HashMap<u16, Channel> = protocol_handles.into_iter().collect();

        Ok(Multiplexer {
            io_handles,
            tx_thread,
            rx_thread,
        })
    }

    pub fn use_channel(&mut self, protocol_id: u16) -> Channel {
        self.io_handles.remove(&protocol_id).unwrap()
    }

    pub fn join(self) {
        self.tx_thread.join().unwrap();
        self.rx_thread.join().unwrap();
    }
}
