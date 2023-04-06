use std::future::ready;

use byteorder::{ByteOrder, NetworkEndian};
use gasket::error::AsWorkError;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf, ReadHalf, WriteHalf};
use tokio::net::{TcpStream, ToSocketAddrs};
use tokio::select;
use tokio::time::Instant;
use tracing::{debug, error, info, trace, warn};

use pallas_miniprotocols::handshake;

use crate::framework::*;

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
        let protocol = NetworkEndian::read_u16(&value[4..6]) ^ 0x8000;
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

use tokio::io::{AsyncReadExt, AsyncWriteExt};

struct AsyncBearer(OwnedReadHalf, OwnedWriteHalf, Instant);

impl AsyncBearer {
    async fn connect_tcp(addr: impl ToSocketAddrs) -> Result<Self, std::io::Error> {
        let stream = TcpStream::connect(addr).await?;
        let (read, write) = stream.into_split();

        Ok(Self(read, write, Instant::now()))
    }
}

impl AsyncBearer {
    async fn readable(&self) -> tokio::io::Result<()> {
        self.0.readable().await
    }

    /// Peek the available data in search for a frame header
    async fn peek_header(&mut self) -> tokio::io::Result<Option<Header>> {
        let mut buf = [0u8; HEADER_LEN];
        let len = self.0.peek(&mut buf).await?;

        if len < HEADER_LEN {
            return Ok(None);
        }

        Ok(Some(Header::from(buf.as_slice())))
    }

    async fn has_payload(&mut self, payload_len: usize) -> tokio::io::Result<bool> {
        let segment_size = HEADER_LEN + payload_len;
        let mut buf = vec![0u8; segment_size];

        let available = self.0.peek(&mut buf).await?;

        return Ok(available >= segment_size);
    }

    /// Peeks the bearer to see if a full segment is available to be read
    async fn has_segment(&mut self) -> std::io::Result<bool> {
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
    async fn read_segment(&mut self) -> tokio::io::Result<(Protocol, Payload)> {
        let mut buf = [0u8; HEADER_LEN];
        self.0.read_exact(&mut buf).await?;
        let header = Header::from(buf.as_slice());

        // TODO: assert any business invariants regarding timestamp from the other party

        let mut payload = vec![0u8; header.payload_len as usize];
        self.0.read_exact(&mut payload).await?;

        Ok((header.protocol, payload))
    }

    async fn write_segment(&mut self, protocol: u16, payload: &[u8]) -> Result<(), std::io::Error> {
        let header = Header {
            protocol,
            timestamp: self.2.elapsed().as_micros() as u32,
            payload_len: payload.len() as u16,
        };

        let buf: [u8; 8] = header.into();
        self.1.write_all(&buf).await?;

        self.1.write_all(&payload).await?;

        Ok(())
    }
}

pub struct AsyncAgentChannel(
    Protocol,
    tokio::sync::mpsc::Sender<(Protocol, Payload)>,
    tokio::sync::broadcast::Receiver<(Protocol, Payload)>,
);

impl pallas_multiplexer::agents::Channel for AsyncAgentChannel {
    async fn enqueue_chunk(
        &mut self,
        chunk: pallas_multiplexer::Payload,
    ) -> Result<(), pallas_multiplexer::agents::ChannelError> {
        let res = self.1.send((self.0, chunk)).await;

        res.map_err(|err| pallas_multiplexer::agents::ChannelError::NotConnected(Some(err.0 .1)))
    }

    async fn dequeue_chunk(
        &mut self,
    ) -> Result<pallas_multiplexer::Payload, pallas_multiplexer::agents::ChannelError> {
        loop {
            let (protocol, payload) = self
                .2
                .recv()
                .await
                .map_err(|err| pallas_multiplexer::agents::ChannelError::NotConnected(None))?;

            if protocol == self.0 {
                break Ok(payload);
            }
        }
    }
}

pub type AsyncIngress = (
    tokio::sync::mpsc::Sender<(Protocol, Payload)>,
    tokio::sync::mpsc::Receiver<(Protocol, Payload)>,
);
pub type AsyncEgress = (
    tokio::sync::broadcast::Sender<(Protocol, Payload)>,
    tokio::sync::broadcast::Receiver<(Protocol, Payload)>,
);

struct AsyncPlexer {
    bearer: AsyncBearer,
    ingress: AsyncIngress,
    egress: AsyncEgress,
}

impl AsyncPlexer {
    pub fn new(bearer: AsyncBearer) -> Self {
        Self {
            bearer,
            ingress: tokio::sync::mpsc::channel(100), // TODO: define buffer
            egress: tokio::sync::broadcast::channel(100),
        }
    }

    async fn mux(&mut self, msg: (Protocol, Payload)) -> tokio::io::Result<()> {
        self.bearer.write_segment(msg.0, &msg.1).await?;

        Ok(())
    }

    async fn demux(&mut self) -> tokio::io::Result<()> {
        let (protocol, payload) = self.bearer.read_segment().await?;

        self.egress.0.send((protocol, payload)).unwrap();

        Ok(())
    }

    pub fn subscribe(&mut self, protocol: Protocol) -> AsyncAgentChannel {
        let agent_tx = self.ingress.0.clone();
        let agent_rx = self.egress.0.subscribe();

        AsyncAgentChannel(protocol, agent_tx, agent_rx)
    }

    pub async fn run(&mut self) -> tokio::io::Result<()> {
        loop {
            select! {
                Ok(_) = self.bearer.readable() => {
                    if let Ok(true) = self.bearer.has_segment().await {
                        trace!("demux selected");
                        self.demux().await?
                    }
                },
                Some(x) = self.ingress.1.recv() => {
                    trace!("mux selected");
                    self.mux(x).await?
                },
            }
        }
    }
}

impl From<AsyncBearer> for AsyncPlexer {
    fn from(value: AsyncBearer) -> Self {
        Self::new(value)
    }
}

impl From<AsyncPlexer> for AsyncBearer {
    fn from(value: AsyncPlexer) -> Self {
        value.bearer
    }
}

async fn handshake(
    plexer: &mut AsyncPlexer,
    network_magic: u64,
) -> Result<(), gasket::error::Error> {
    info!("executing handshake");

    let channel0 = plexer.subscribe(0);
    let versions = handshake::n2n::VersionTable::v7_and_above(network_magic);
    let mut client = handshake::Client::new(channel0);

    //let p = tokio::spawn(plexer.run());
    //let output = client.handshake(versions).or_restart()?;

    let output = select! {
        x = client.handshake(versions) => x.or_restart()?,
        x = plexer.run() => {
            match x.or_restart() {
                Err(x) => return Err(x),
                _ => unreachable!(),
            };
        },
    };

    debug!("handshake output: {:?}", output);
    //p.abort();

    match output {
        handshake::Confirmation::Accepted(version, _) => {
            info!(version, "connected to upstream peer");
            Ok(())
        }
        _ => {
            error!("couldn't agree on handshake version");
            Err(gasket::error::Error::WorkPanic)
        }
    }
}

pub struct Worker {
    peer_address: String,
    network_magic: u64,
    bearer: Option<AsyncBearer>,
    mux_input: MuxInputPort,
    channel2_out: Option<DemuxOutputPort>,
    channel3_out: Option<DemuxOutputPort>,
    ops_count: gasket::metrics::Counter,
}

impl Worker {
    pub fn new(
        peer_address: String,
        network_magic: u64,
        mux_input: MuxInputPort,
        channel2_out: Option<DemuxOutputPort>,
        channel3_out: Option<DemuxOutputPort>,
    ) -> Self {
        Self {
            peer_address,
            network_magic,
            channel2_out,
            channel3_out,
            mux_input,
            bearer: None,
            ops_count: Default::default(),
        }
    }
}

pub enum WorkUnit {
    Connect,
    Mux((u16, Vec<u8>)),
    Demux,
}

impl gasket::runtime::Worker for Worker {
    type WorkUnit = WorkUnit;

    fn metrics(&self) -> gasket::metrics::Registry {
        // TODO: define networking metrics (bytes in / out, etc)
        gasket::metrics::Builder::new()
            .with_counter("ops_count", &self.ops_count)
            .build()
    }

    async fn bootstrap(&mut self) -> gasket::runtime::ScheduleResult<Self::WorkUnit> {
        Ok(gasket::runtime::WorkSchedule::Unit(WorkUnit::Connect))
    }

    async fn schedule(&mut self) -> gasket::runtime::ScheduleResult<Self::WorkUnit> {
        let bearer = self.bearer.as_mut().unwrap();
        trace!("selecting");
        select! {
            Ok(msg) = self.mux_input.recv() => { Ok(gasket::runtime::WorkSchedule::Unit(WorkUnit::Mux(msg.payload))) }
            Ok(true) = bearer.has_segment() => Ok(gasket::runtime::WorkSchedule::Unit(WorkUnit::Demux)),
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(1)) => Ok(gasket::runtime::WorkSchedule::Idle),
        }
    }

    async fn execute(&mut self, unit: &Self::WorkUnit) -> Result<(), gasket::error::Error> {
        match unit {
            WorkUnit::Connect => {
                debug!("connecting");
                let bearer = AsyncBearer::connect_tcp(&self.peer_address)
                    .await
                    .or_retry()?;

                let mut plexer = bearer.into();

                handshake(&mut plexer, self.network_magic).await?;

                self.bearer = Some(plexer.into());
            }
            WorkUnit::Mux(x) => {
                trace!("muxing");
                self.bearer
                    .as_mut()
                    .unwrap()
                    .write_segment(x.0, &x.1)
                    .await
                    .or_restart()?;
            }
            WorkUnit::Demux => {
                trace!("demuxing");

                let (protocol, payload) = self
                    .bearer
                    .as_mut()
                    .unwrap()
                    .read_segment()
                    .await
                    .or_restart()?;

                match protocol {
                    2 => {
                        if let Some(channel) = &mut self.channel2_out {
                            channel.send(payload.into()).await?;
                            trace!("sent protocol 2 msg");
                        }
                    }
                    3 => {
                        if let Some(channel) = &mut self.channel3_out {
                            channel.send(payload.into()).await?;
                            trace!("sent protocol 3 msg");
                        }
                    }
                    x => warn!("trying to demux unexpected protocol {x}"),
                }
            }
        };

        Ok(())
    }
}
