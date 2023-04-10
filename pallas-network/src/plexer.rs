use pallas_codec::{minicbor, Fragment};
use thiserror::Error;
use tokio::sync::mpsc::error::SendError;
use tokio::{select, time::Instant};
use tracing::{debug, error, trace};

use crate::bearer::{Bearer, Payload, Protocol, SegmentBuffer};

#[derive(Error, Debug)]
pub enum Error {
    #[error("failure to encode channel message")]
    Decoding(String),

    #[error("failure to decode channel message")]
    Encoding(String),

    #[error("agent failed to enqueue chunk for protocol {0}")]
    AgentEnqueue(Protocol, Payload),

    #[error("agent failed to dequeue chunk")]
    AgentDequeue,

    #[error("plexer failed to dumux chunk for protocol {0}")]
    PlexerDemux(Protocol, Payload),

    #[error("plexer failed to mux chunk")]
    PlexerMux,

    #[error("bearer IO error")]
    Bearer(tokio::io::Error),
}

pub struct AgentChannel {
    enqueue_protocol: crate::bearer::Protocol,
    dequeue_protocol: crate::bearer::Protocol,
    to_plexer: tokio::sync::mpsc::Sender<(Protocol, Payload)>,
    from_plexer: tokio::sync::broadcast::Receiver<(Protocol, Payload)>,
}

impl AgentChannel {
    fn for_client(protocol: crate::bearer::Protocol, ingress: &Ingress, egress: &Egress) -> Self {
        Self {
            enqueue_protocol: protocol,
            dequeue_protocol: protocol ^ 0x8000,
            to_plexer: ingress.0.clone(),
            from_plexer: egress.0.subscribe(),
        }
    }

    fn for_server(protocol: crate::bearer::Protocol, ingress: &Ingress, egress: &Egress) -> Self {
        Self {
            enqueue_protocol: protocol ^ 0x8000,
            dequeue_protocol: protocol,
            to_plexer: ingress.0.clone(),
            from_plexer: egress.0.subscribe(),
        }
    }

    pub async fn enqueue_chunk(&mut self, chunk: Payload) -> Result<(), Error> {
        self.to_plexer
            .send((self.enqueue_protocol, chunk))
            .await
            .map_err(|SendError((protocol, payload))| Error::AgentEnqueue(protocol, payload))
    }

    pub async fn dequeue_chunk(&mut self) -> Result<Payload, Error> {
        loop {
            let (protocol, payload) = self
                .from_plexer
                .recv()
                .await
                .map_err(|_| Error::AgentDequeue)?;

            if protocol == self.dequeue_protocol {
                trace!(protocol, "message for our protocol");
                break Ok(payload);
            }
        }
    }
}

type Ingress = (
    tokio::sync::mpsc::Sender<(Protocol, Payload)>,
    tokio::sync::mpsc::Receiver<(Protocol, Payload)>,
);

type Egress = (
    tokio::sync::broadcast::Sender<(Protocol, Payload)>,
    tokio::sync::broadcast::Receiver<(Protocol, Payload)>,
);

pub struct Plexer {
    clock: Instant,
    bearer: SegmentBuffer,
    ingress: Ingress,
    egress: Egress,
}

impl Plexer {
    pub fn new(bearer: Bearer) -> Self {
        Self {
            clock: Instant::now(),
            bearer: SegmentBuffer::new(bearer),
            ingress: tokio::sync::mpsc::channel(100), // TODO: define buffer
            egress: tokio::sync::broadcast::channel(100),
        }
    }

    async fn mux(&mut self, msg: (Protocol, Payload)) -> tokio::io::Result<()> {
        self.bearer
            .write_segment(msg.0, &self.clock, &msg.1)
            .await?;

        if tracing::event_enabled!(tracing::Level::TRACE) {
            trace!(
                protocol = msg.0,
                data = hex::encode(&msg.1),
                "write to bearer"
            );
        }

        Ok(())
    }

    async fn demux(&mut self, protocol: Protocol, payload: Payload) -> tokio::io::Result<()> {
        if tracing::event_enabled!(tracing::Level::TRACE) {
            trace!(protocol, data = hex::encode(&payload), "read from bearer");
        }

        self.egress.0.send((protocol, payload)).unwrap();

        Ok(())
    }

    pub fn subscribe_client(&mut self, protocol: Protocol) -> AgentChannel {
        AgentChannel::for_client(protocol, &self.ingress, &self.egress)
    }

    pub fn subscribe_server(&mut self, protocol: Protocol) -> AgentChannel {
        AgentChannel::for_server(protocol, &self.ingress, &self.egress)
    }

    pub async fn run(&mut self) -> tokio::io::Result<()> {
        loop {
            trace!("selecting");
            select! {
                Ok(x) = self.bearer.read_segment() => {
                    trace!("demux selected");
                    self.demux(x.0, x.1).await?
                },
                Some(x) = self.ingress.1.recv() => {
                    trace!("mux selected");
                    self.mux(x).await?
                },
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(5)) => {
                    trace!("idle plexer");
                }
                else => {
                    error!("something else happened");
                }
            }
        }
    }
}

/// Protocol value that defines max segment length
pub const MAX_SEGMENT_PAYLOAD_LENGTH: usize = 65535;

fn try_decode_message<M>(buffer: &mut Vec<u8>) -> Result<Option<M>, Error>
where
    M: Fragment,
{
    let mut decoder = minicbor::Decoder::new(buffer);
    let maybe_msg = decoder.decode();

    match maybe_msg {
        Ok(msg) => {
            let pos = decoder.position();
            buffer.drain(0..pos);
            Ok(Some(msg))
        }
        Err(err) if err.is_end_of_input() => Ok(None),
        Err(err) => {
            error!(?err);
            trace!("{}", hex::encode(buffer));
            Err(Error::Decoding(err.to_string()))
        }
    }
}

/// A channel abstraction to hide the complexity of partial payloads
pub struct ChannelBuffer {
    channel: AgentChannel,
    temp: Vec<u8>,
}

impl ChannelBuffer {
    pub fn new(channel: AgentChannel) -> Self {
        Self {
            channel,
            temp: Vec::new(),
        }
    }

    /// Enqueues a msg as a sequence payload chunks
    pub async fn send_msg_chunks<M>(&mut self, msg: &M) -> Result<(), Error>
    where
        M: Fragment,
    {
        let mut payload = Vec::new();
        minicbor::encode(msg, &mut payload).map_err(|err| Error::Encoding(err.to_string()))?;

        let chunks = payload.chunks(MAX_SEGMENT_PAYLOAD_LENGTH);

        for chunk in chunks {
            self.channel.enqueue_chunk(Vec::from(chunk)).await?;
        }

        Ok(())
    }

    /// Reads from the channel until a complete message is found
    pub async fn recv_full_msg<M>(&mut self) -> Result<M, Error>
    where
        M: Fragment,
    {
        trace!(len = self.temp.len(), "waiting for full message");

        if !self.temp.is_empty() {
            trace!("buffer has data from previous payload");

            if let Some(msg) = try_decode_message::<M>(&mut self.temp)? {
                debug!("decoding done");
                return Ok(msg);
            }
        }

        loop {
            let chunk = self.channel.dequeue_chunk().await?;
            self.temp.extend(chunk);

            if let Some(msg) = try_decode_message::<M>(&mut self.temp)? {
                debug!("decoding done");
                return Ok(msg);
            }

            trace!("not enough data");
        }
    }

    pub fn unwrap(self) -> AgentChannel {
        self.channel
    }
}

impl From<AgentChannel> for ChannelBuffer {
    fn from(channel: AgentChannel) -> Self {
        ChannelBuffer::new(channel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pallas_codec::minicbor;

    #[tokio::test]
    async fn multiple_messages_in_same_payload() {
        let mut input = Vec::new();
        let in_part1 = (1u8, 2u8, 3u8);
        let in_part2 = (6u8, 5u8, 4u8);

        minicbor::encode(in_part1, &mut input).unwrap();
        minicbor::encode(in_part2, &mut input).unwrap();

        let ingress = tokio::sync::mpsc::channel(100);
        let egress = tokio::sync::broadcast::channel(100);

        let channel = AgentChannel::for_client(0, &ingress, &egress);

        egress.0.send((0 ^ 0x8000, input)).unwrap();

        let mut buf = ChannelBuffer::new(channel);

        let out_part1 = buf.recv_full_msg::<(u8, u8, u8)>().await.unwrap();
        let out_part2 = buf.recv_full_msg::<(u8, u8, u8)>().await.unwrap();

        assert_eq!(in_part1, out_part1);
        assert_eq!(in_part2, out_part2);
    }

    #[tokio::test]
    async fn fragmented_message_in_multiple_payloads() {
        let mut input = Vec::new();
        let msg = (11u8, 12u8, 13u8, 14u8, 15u8, 16u8, 17u8);
        minicbor::encode(msg, &mut input).unwrap();

        let ingress = tokio::sync::mpsc::channel(100);
        let egress = tokio::sync::broadcast::channel(100);

        let channel = AgentChannel::for_client(0, &ingress, &egress);

        while !input.is_empty() {
            let chunk = Vec::from(input.drain(0..2).as_slice());
            egress.0.send((0 ^ 0x8000, chunk)).unwrap();
        }

        let mut buf = ChannelBuffer::new(channel);

        let out_msg = buf
            .recv_full_msg::<(u8, u8, u8, u8, u8, u8, u8)>()
            .await
            .unwrap();

        assert_eq!(msg, out_msg);
    }
}
