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

use pallas_codec::scan::Scanner;

use crate::{Channel, Message, Payload, protocol as proto};

const HEADER_LEN: usize = 8;

/// A timestamp value in microseconds, included in each segment header.
pub type Timestamp = u32;

/// The header of a multiplexed segment on the wire.
#[derive(Debug)]
pub struct Header {
    /// The mini-protocol channel identifier.
    pub protocol: Channel,
    /// Timestamp in microseconds since the connection was established.
    pub timestamp: Timestamp,
    /// Length of the segment payload in bytes.
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

/// A single multiplexed segment consisting of a header and payload.
pub struct Segment {
    /// The segment header containing channel, timestamp, and length.
    pub header: Header,
    /// The raw payload bytes of the segment.
    pub payload: Payload,
}

/// A network bearer (transport) that can be split into read/write halves.
///
/// Supports TCP, Unix domain sockets, and Windows named pipes.
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

    /// Connects to a remote peer via TCP.
    pub async fn connect_tcp(addr: impl tcp::ToSocketAddrs) -> Result<Self, tokio::io::Error> {
        let stream = tcp::TcpStream::connect(addr).await?;
        Self::configure_tcp(&stream)?;
        // Aggressive linger avoids TIME_WAIT accumulation when connecting to many nodes
        socket2::SockRef::from(&stream).set_linger(Some(std::time::Duration::from_secs(0)))?;
        Ok(Self::Tcp(stream))
    }

    /// Connects to a remote peer via TCP with a timeout.
    pub async fn connect_tcp_timeout(
        addr: impl tcp::ToSocketAddrs,
        timeout: std::time::Duration,
    ) -> IOResult<Self> {
        select! {
            result = Self::connect_tcp(addr) => result,
            _ = tokio::time::sleep(timeout) => Err(tokio::io::Error::new(tokio::io::ErrorKind::TimedOut, "connect timeout")),
        }
    }

    /// Accepts an incoming TCP connection from a listener.
    pub async fn accept_tcp(listener: &tcp::TcpListener) -> IOResult<(Self, std::net::SocketAddr)> {
        let (stream, addr) = listener.accept().await?;
        Self::configure_tcp(&stream)?;
        Ok((Self::Tcp(stream), addr))
    }

    /// Connects to a remote peer via a Unix domain socket.
    #[cfg(unix)]
    pub async fn connect_unix(path: impl AsRef<std::path::Path>) -> IOResult<Self> {
        let stream = unix::UnixStream::connect(path).await?;
        Ok(Self::Unix(stream))
    }

    /// Accepts an incoming connection on a Unix domain socket listener.
    #[cfg(unix)]
    pub async fn accept_unix(
        listener: &unix::UnixListener,
    ) -> IOResult<(Self, unix::unix::SocketAddr)> {
        let (stream, addr) = listener.accept().await?;
        Ok((Self::Unix(stream), addr))
    }

    /// Connects to a Windows named pipe.
    #[cfg(windows)]
    pub fn connect_named_pipe(pipe_name: impl AsRef<std::ffi::OsStr>) -> IOResult<Self> {
        let client = tokio::net::windows::named_pipe::ClientOptions::new().open(&pipe_name)?;
        Ok(Self::NamedPipe(client))
    }

    /// Splits the bearer into independent read and write halves.
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

/// The read half of a split [`Bearer`].
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

    /// Reads a single segment from the bearer, returning its channel and payload.
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
    ///
    /// Buffers a partial message of any size and decodes it again from its
    /// first byte on every segment. [`Self::read_messages`] reassembles in
    /// time linear in the bytes received and bounds the size of a message.
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

    /// Reads one segment and returns the messages it completes.
    ///
    /// Each message is expected to be one CBOR item. Every inbound byte is
    /// scanned once to find where its message ends, and the message is decoded
    /// once, when that item is whole. Bytes of the item that the decoder leaves
    /// unread are discarded with it.
    ///
    /// Fails with [`std::io::ErrorKind::InvalidData`], leaving nothing buffered
    /// for the channel, when the bytes are not well-formed CBOR, when a whole
    /// item does not decode as a message, or, while `buffers` has its size
    /// limits on, when a message is larger than the `MESSAGE_SIZE_LIMIT` of its
    /// protocol module (65,535 bytes on other channels). The limit is checked
    /// after each segment, so at most one segment beyond it is buffered. Drop
    /// the connection on any error.
    pub async fn read_messages<M>(&mut self, buffers: &mut ChannelBuffers) -> IOResult<Vec<M>>
    where
        M: Message,
    {
        let (raw_channel, chunk) = self.read_segment().await?;
        let channel = raw_channel & !crate::protocol::PROTOCOL_SERVER;

        buffers.absorb(channel, chunk)
    }
}

/// The largest inbound message accepted on `channel`, in bytes.
fn size_limit(channel: Channel) -> usize {
    match channel {
        proto::handshake::CHANNEL_ID => proto::handshake::MESSAGE_SIZE_LIMIT,
        proto::chainsync::CHANNEL_ID => proto::chainsync::MESSAGE_SIZE_LIMIT,
        proto::blockfetch::CHANNEL_ID => proto::blockfetch::MESSAGE_SIZE_LIMIT,
        proto::txsubmission::CHANNEL_ID => proto::txsubmission::MESSAGE_SIZE_LIMIT,
        proto::keepalive::CHANNEL_ID => proto::keepalive::MESSAGE_SIZE_LIMIT,
        proto::peersharing::CHANNEL_ID => proto::peersharing::MESSAGE_SIZE_LIMIT,
        proto::leiosnotify::CHANNEL_ID => proto::leiosnotify::MESSAGE_SIZE_LIMIT,
        proto::leiosfetch::CHANNEL_ID => proto::leiosfetch::MESSAGE_SIZE_LIMIT,
        _ => crate::MAX_SEGMENT_PAYLOAD_LENGTH,
    }
}

fn invalid_data(msg: String) -> tokio::io::Error {
    tokio::io::Error::new(tokio::io::ErrorKind::InvalidData, msg)
}

/// The bytes received on one channel past its last complete message.
#[derive(Debug, Default)]
struct PartialMessage {
    bytes: Payload,
    scanner: Scanner,
}

/// Partial inbound messages of one connection, per channel, for
/// [`BearerReadHalf::read_messages`].
///
/// Size limits are on unless turned off with [`Self::disable_size_limits`].
#[derive(Debug)]
pub struct ChannelBuffers {
    channels: HashMap<Channel, PartialMessage>,
    limits_enabled: bool,
}

impl Default for ChannelBuffers {
    fn default() -> Self {
        Self::new()
    }
}

impl ChannelBuffers {
    /// Empty buffers, with size limits on.
    pub fn new() -> Self {
        Self {
            channels: HashMap::new(),
            limits_enabled: true,
        }
    }

    /// Accept inbound messages of any size. Malformed messages still fail.
    pub fn disable_size_limits(&mut self) {
        self.limits_enabled = false;
    }

    fn absorb<M: Message>(&mut self, channel: Channel, chunk: Payload) -> IOResult<Vec<M>> {
        let limit = self.limits_enabled.then(|| size_limit(channel));
        let partial = self.channels.entry(channel).or_default();
        partial.bytes.extend(chunk);

        let result = Self::take_messages(partial, channel, limit);

        if result.is_err() {
            self.channels.remove(&channel);
        }

        result
    }

    /// Decode every message the buffered bytes complete, then drop their bytes.
    fn take_messages<M: Message>(
        partial: &mut PartialMessage,
        channel: Channel,
        limit: Option<usize>,
    ) -> IOResult<Vec<M>> {
        let mut msgs = Vec::new();
        let mut start = 0;

        loop {
            let end = match partial.scanner.scan(&partial.bytes) {
                Ok(Some(end)) => end,
                Ok(None) => break,
                Err(err) => return Err(invalid_data(err.to_string())),
            };

            if let Some(limit) = limit
                && end - start > limit
            {
                return Err(size_limit_error(limit, end - start));
            }

            let mut item = partial.bytes[start..end].to_vec();

            match M::from_payload(channel, &mut item) {
                Some(msg) => msgs.push(msg),
                // An implementation that consumes the item without a message
                // skips it, as for a channel it does not handle.
                None if item.is_empty() => (),
                None => {
                    return Err(invalid_data(format!(
                        "CBOR item of {} bytes is not a valid message",
                        end - start
                    )));
                }
            }

            if !item.is_empty() {
                tracing::debug!(len = item.len(), "discarding bytes left in a decoded item");
            }

            start = end;
        }

        partial.bytes.drain(..start);
        partial.scanner.remove_prefix(start);

        if let Some(limit) = limit
            && partial.bytes.len() > limit
        {
            return Err(size_limit_error(limit, partial.bytes.len()));
        }

        Ok(msgs)
    }
}

fn size_limit_error(limit: usize, seen: usize) -> tokio::io::Error {
    invalid_data(format!(
        "message of at least {seen} bytes exceeds the size limit of {limit} bytes"
    ))
}

/// The write half of a split [`Bearer`].
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

    /// Writes a single segment (header + payload) to the bearer.
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

    /// Encodes and writes a complete message, splitting it into segments as needed.
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

    /// Shuts down the write half of the connection.
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

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, Ipv6Addr};

    use pallas_codec::minicbor;

    use super::*;
    use crate::behavior::AnyMessage;
    use crate::protocol::{
        AnyCbor, MAINNET_MAGIC, Point, blockfetch, chainsync, handshake, keepalive, leiosfetch,
        leiosnotify, peersharing, txsubmission,
    };

    fn point() -> Point {
        Point::new(42, vec![0xab; 32])
    }

    fn cbor_bytes(len: usize) -> Vec<u8> {
        let mut out = vec![];
        minicbor::Encoder::new(&mut out)
            .bytes(&vec![7u8; len])
            .unwrap();
        out
    }

    fn header(variant: u8) -> chainsync::HeaderContent {
        chainsync::HeaderContent {
            variant,
            byron_prefix: (variant == 0).then_some((1, 2)),
            cbor: cbor_bytes(40),
        }
    }

    /// One message of every variant of every protocol.
    fn samples() -> Vec<AnyMessage> {
        use AnyMessage as A;
        let table = handshake::n2n::VersionTable::v11_and_above(MAINNET_MAGIC);
        let data = handshake::n2n::VersionData::new(MAINNET_MAGIC, false, Some(1), Some(false));
        let tip = chainsync::Tip(point(), 7);
        let tx_id = txsubmission::EraTxId(6, vec![1; 32]);

        vec![
            A::Handshake(handshake::Message::Propose(table.clone())),
            A::Handshake(handshake::Message::Accept(14, data)),
            A::Handshake(handshake::Message::Refuse(
                handshake::RefuseReason::VersionMismatch(vec![13, 14]),
            )),
            A::Handshake(handshake::Message::Refuse(
                handshake::RefuseReason::HandshakeDecodeError(14, "bad".into()),
            )),
            A::Handshake(handshake::Message::Refuse(
                handshake::RefuseReason::Refused(14, "no".into()),
            )),
            A::Handshake(handshake::Message::QueryReply(table)),
            A::KeepAlive(keepalive::Message::KeepAlive(9)),
            A::KeepAlive(keepalive::Message::ResponseKeepAlive(9)),
            A::KeepAlive(keepalive::Message::Done),
            A::ChainSync(chainsync::Message::RequestNext),
            A::ChainSync(chainsync::Message::AwaitReply),
            A::ChainSync(chainsync::Message::RollForward(header(0), tip.clone())),
            A::ChainSync(chainsync::Message::RollForward(header(6), tip.clone())),
            A::ChainSync(chainsync::Message::RollBackward(point(), tip.clone())),
            A::ChainSync(chainsync::Message::FindIntersect(vec![
                Point::Origin,
                point(),
            ])),
            A::ChainSync(chainsync::Message::IntersectFound(point(), tip.clone())),
            A::ChainSync(chainsync::Message::IntersectNotFound(tip)),
            A::ChainSync(chainsync::Message::Done),
            A::PeerSharing(peersharing::Message::ShareRequest(10)),
            A::PeerSharing(peersharing::Message::SharePeers(vec![
                peersharing::PeerAddress::V4(Ipv4Addr::new(10, 0, 0, 1), 3001),
                peersharing::PeerAddress::V6(Ipv6Addr::LOCALHOST, 3001),
            ])),
            A::PeerSharing(peersharing::Message::Done),
            A::BlockFetch(blockfetch::Message::RequestRange((Point::Origin, point()))),
            A::BlockFetch(blockfetch::Message::ClientDone),
            A::BlockFetch(blockfetch::Message::StartBatch),
            A::BlockFetch(blockfetch::Message::NoBlocks),
            A::BlockFetch(blockfetch::Message::Block(cbor_bytes(100))),
            A::BlockFetch(blockfetch::Message::BatchDone),
            A::TxSubmission(txsubmission::Message::Init),
            A::TxSubmission(txsubmission::Message::RequestTxIds(true, 1, 2)),
            A::TxSubmission(txsubmission::Message::ReplyTxIds(vec![
                txsubmission::TxIdAndSize(tx_id.clone(), 100),
            ])),
            A::TxSubmission(txsubmission::Message::RequestTxs(vec![tx_id])),
            A::TxSubmission(txsubmission::Message::ReplyTxs(vec![
                txsubmission::EraTxBody(6, cbor_bytes(30)),
            ])),
            A::TxSubmission(txsubmission::Message::Done),
            A::LeiosNotify(leiosnotify::Message::Done),
            A::LeiosNotify(leiosnotify::Message::Quit),
            A::LeiosNotify(leiosnotify::Message::Canceled),
            A::LeiosNotify(leiosnotify::Message::RequestNext),
            A::LeiosNotify(leiosnotify::Message::BlockAnnouncement(
                AnyCbor::from_raw_bytes(cbor_bytes(20)),
            )),
            A::LeiosNotify(leiosnotify::Message::BlockOffer(point(), 3665)),
            A::LeiosNotify(leiosnotify::Message::BlockTxsOffer(point())),
            A::LeiosNotify(leiosnotify::Message::Votes(vec![AnyCbor::from_raw_bytes(
                cbor_bytes(48),
            )])),
            A::LeiosFetch(leiosfetch::Message::Done),
            A::LeiosFetch(leiosfetch::Message::BlockRequest(point())),
            A::LeiosFetch(leiosfetch::Message::Block(AnyCbor::from_raw_bytes(
                cbor_bytes(64),
            ))),
            A::LeiosFetch(leiosfetch::Message::BlockTxsRequest(
                point(),
                leiosfetch::Bitmaps::all(70),
            )),
            A::LeiosFetch(leiosfetch::Message::BlockTxs {
                txs: vec![AnyCbor::from_raw_bytes(cbor_bytes(30))],
            }),
        ]
    }

    /// Feed `chunks` as consecutive segments on `channel`, collecting messages.
    fn feed(
        buffers: &mut ChannelBuffers,
        channel: Channel,
        chunks: impl IntoIterator<Item = Vec<u8>>,
    ) -> IOResult<Vec<AnyMessage>> {
        let mut out = vec![];
        for chunk in chunks {
            out.extend(buffers.absorb::<AnyMessage>(channel, chunk)?);
        }
        Ok(out)
    }

    fn payloads(msgs: &[AnyMessage]) -> Vec<Vec<u8>> {
        msgs.iter().map(|m| m.payload()).collect()
    }

    fn assert_size_limit_error(err: tokio::io::Error, limit: usize) {
        assert_eq!(err.kind(), tokio::io::ErrorKind::InvalidData);
        assert!(
            err.to_string()
                .ends_with(&format!("exceeds the size limit of {limit} bytes")),
            "{err}"
        );
    }

    #[test]
    fn every_message_is_one_cbor_item_and_survives_every_split() {
        for msg in samples() {
            let bytes = msg.payload();
            let channel = msg.channel();

            let mut scanner = Scanner::default();
            assert_eq!(scanner.scan(&bytes), Ok(Some(bytes.len())), "{msg:?}");

            for split in 0..=bytes.len() {
                let mut buffers = ChannelBuffers::new();
                let chunks = [bytes[..split].to_vec(), bytes[split..].to_vec()];
                let got = feed(&mut buffers, channel, chunks).unwrap();
                assert_eq!(payloads(&got), vec![bytes.clone()], "{msg:?} split {split}");
                assert!(buffers.channels[&channel].bytes.is_empty());
            }
        }
    }

    #[test]
    fn several_messages_in_one_segment_are_delivered_in_order() {
        let msgs = [
            AnyMessage::ChainSync(chainsync::Message::RequestNext),
            AnyMessage::ChainSync(chainsync::Message::FindIntersect(vec![point()])),
            AnyMessage::ChainSync(chainsync::Message::Done),
        ];
        let expected = payloads(&msgs);
        let mut segment = expected.concat();
        // The start of a fourth message waits for the next segment.
        let fourth = AnyMessage::ChainSync(chainsync::Message::AwaitReply).payload();
        segment.extend(&fourth[..fourth.len() - 1]);

        let mut buffers = ChannelBuffers::new();
        let got = feed(&mut buffers, chainsync::CHANNEL_ID, [segment]).unwrap();
        assert_eq!(payloads(&got), expected);

        let rest = vec![fourth[fourth.len() - 1]];
        let got = feed(&mut buffers, chainsync::CHANNEL_ID, [rest]).unwrap();
        assert_eq!(payloads(&got), vec![fourth]);
    }

    #[test]
    fn channels_reassemble_independently() {
        let a = AnyMessage::ChainSync(chainsync::Message::FindIntersect(vec![point()])).payload();
        let b = AnyMessage::BlockFetch(blockfetch::Message::Block(cbor_bytes(500))).payload();

        let mut buffers = ChannelBuffers::new();
        assert!(feed(&mut buffers, 2, [a[..5].to_vec()]).unwrap().is_empty());
        assert!(
            feed(&mut buffers, 3, [b[..300].to_vec()])
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            payloads(&feed(&mut buffers, 2, [a[5..].to_vec()]).unwrap()),
            [a]
        );
        assert_eq!(
            payloads(&feed(&mut buffers, 3, [b[300..].to_vec()]).unwrap()),
            [b]
        );
    }

    /// A message of `make(n)` whose encoding is as close to `target` bytes as
    /// the padding allows without passing it.
    fn sized(target: usize, make: impl Fn(usize) -> AnyMessage) -> Vec<u8> {
        let overhead = make(0).payload().len();
        let mut n = target.saturating_sub(overhead);
        loop {
            let bytes = make(n).payload();
            if bytes.len() <= target {
                assert!(target - bytes.len() < 16, "padding missed {target}");
                return bytes;
            }
            n -= 1;
        }
    }

    /// For each channel that can carry a message near its limit: one just
    /// under it is delivered across segment splits and next to another message,
    /// one just over it is refused, and the opt-out accepts it.
    #[test]
    fn messages_up_to_each_channel_limit_are_delivered() {
        type Make = Box<dyn Fn(usize) -> AnyMessage>;
        let cases: Vec<(Channel, usize, Make)> = vec![
            (
                handshake::CHANNEL_ID,
                handshake::MESSAGE_SIZE_LIMIT,
                Box::new(|n| {
                    AnyMessage::Handshake(handshake::Message::Refuse(
                        handshake::RefuseReason::Refused(14, "x".repeat(n)),
                    ))
                }),
            ),
            (
                chainsync::CHANNEL_ID,
                chainsync::MESSAGE_SIZE_LIMIT,
                Box::new(|n| {
                    let mut header = header(6);
                    header.cbor = cbor_bytes(n);
                    AnyMessage::ChainSync(chainsync::Message::RollForward(
                        header,
                        chainsync::Tip(point(), 7),
                    ))
                }),
            ),
            (
                blockfetch::CHANNEL_ID,
                blockfetch::MESSAGE_SIZE_LIMIT,
                Box::new(|n| AnyMessage::BlockFetch(blockfetch::Message::Block(vec![1; n]))),
            ),
            (
                txsubmission::CHANNEL_ID,
                txsubmission::MESSAGE_SIZE_LIMIT,
                Box::new(|n| {
                    AnyMessage::TxSubmission(txsubmission::Message::ReplyTxs(vec![
                        txsubmission::EraTxBody(6, vec![1; n]),
                    ]))
                }),
            ),
            (
                leiosnotify::CHANNEL_ID,
                leiosnotify::MESSAGE_SIZE_LIMIT,
                Box::new(|n| {
                    AnyMessage::LeiosNotify(leiosnotify::Message::BlockAnnouncement(
                        AnyCbor::from_raw_bytes(cbor_bytes(n)),
                    ))
                }),
            ),
            (
                leiosfetch::CHANNEL_ID,
                leiosfetch::MESSAGE_SIZE_LIMIT,
                Box::new(|n| {
                    AnyMessage::LeiosFetch(leiosfetch::Message::BlockTxs {
                        txs: vec![AnyCbor::from_raw_bytes(cbor_bytes(n))],
                    })
                }),
            ),
        ];

        for (channel, limit, make) in cases {
            assert_eq!(size_limit(channel), limit);
            let under = sized(limit, &make);
            let over = sized(limit + 40, &make);
            assert!(over.len() > limit);

            // Full segments, as a sender splits a message.
            let segments = under
                .chunks(crate::MAX_SEGMENT_PAYLOAD_LENGTH)
                .map(Vec::from);
            let got = feed(&mut ChannelBuffers::new(), channel, segments).unwrap();
            assert_eq!(
                payloads(&got),
                std::slice::from_ref(&under),
                "channel {channel}"
            );

            // Uneven segments, the last carrying a second message after it.
            let tail = make(0).payload();
            let mut both = under.clone();
            both.extend(&tail);
            let segments = both.chunks(7_777).map(Vec::from);
            let got = feed(&mut ChannelBuffers::new(), channel, segments).unwrap();
            assert_eq!(payloads(&got), [under, tail], "channel {channel}");

            let segments = over
                .chunks(crate::MAX_SEGMENT_PAYLOAD_LENGTH)
                .map(Vec::from);
            let mut buffers = ChannelBuffers::new();
            let err = feed(&mut buffers, channel, segments.clone()).unwrap_err();
            assert_size_limit_error(err, limit);
            assert!(!buffers.channels.contains_key(&channel));

            let mut buffers = ChannelBuffers::new();
            buffers.disable_size_limits();
            let got = feed(&mut buffers, channel, segments).unwrap();
            assert_eq!(payloads(&got), [over], "channel {channel}");
        }

        // Keep-alive and peer-sharing messages cannot grow near their limits;
        // a message that never ends checks those limits below.
    }

    #[test]
    fn a_message_that_never_ends_is_refused_past_the_limit() {
        // A map that declares more entries than will ever arrive.
        let mut start = vec![0x82, 0x00, 0xbb];
        start.extend([0xff; 8]);

        for channel in [
            handshake::CHANNEL_ID,
            chainsync::CHANNEL_ID,
            keepalive::CHANNEL_ID,
            peersharing::CHANNEL_ID,
            999,
        ] {
            let limit = size_limit(channel);
            let mut buffers = ChannelBuffers::new();
            let mut sent = 0;

            let mut segments =
                std::iter::once(start.clone()).chain(std::iter::repeat(vec![0x00; 1000]));

            let err = loop {
                let segment = segments.next().unwrap();
                sent += segment.len();
                match buffers.absorb::<AnyMessage>(channel, segment) {
                    Ok(msgs) => assert!(msgs.is_empty()),
                    Err(err) => break err,
                }
            };

            assert_size_limit_error(err, limit);
            assert!(
                sent > limit && sent <= limit + 1000,
                "channel {channel}: {sent}"
            );
            assert!(!buffers.channels.contains_key(&channel));
        }
    }

    #[test]
    fn a_complete_item_over_the_limit_in_one_segment_is_refused() {
        let msg = AnyMessage::Handshake(handshake::Message::Refuse(
            handshake::RefuseReason::Refused(14, "x".repeat(10_000)),
        ));
        let mut buffers = ChannelBuffers::new();
        let err = feed(&mut buffers, handshake::CHANNEL_ID, [msg.payload()]).unwrap_err();
        assert_size_limit_error(err, handshake::MESSAGE_SIZE_LIMIT);
    }

    #[test]
    fn malformed_bytes_are_refused_and_dropped() {
        // Not CBOR: a break with no open container.
        let mut buffers = ChannelBuffers::new();
        let err = feed(&mut buffers, chainsync::CHANNEL_ID, [vec![0xff, 0x00]]).unwrap_err();
        assert_eq!(err.kind(), tokio::io::ErrorKind::InvalidData);
        assert!(!buffers.channels.contains_key(&chainsync::CHANNEL_ID));

        // Well-formed CBOR that is no chain-sync message, after a valid one.
        let mut segment = AnyMessage::ChainSync(chainsync::Message::RequestNext).payload();
        segment.extend(minicbor::to_vec((99u8, "nope")).unwrap());
        let mut buffers = ChannelBuffers::new();
        let err = feed(&mut buffers, chainsync::CHANNEL_ID, [segment]).unwrap_err();
        assert_eq!(err.kind(), tokio::io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("is not a valid message"), "{err}");
        assert!(!buffers.channels.contains_key(&chainsync::CHANNEL_ID));

        // Nesting deeper than the scanner follows.
        let deep = vec![0x81; pallas_codec::scan::MAX_NESTING + 1];
        let mut buffers = ChannelBuffers::new();
        buffers.disable_size_limits();
        let err = feed(&mut buffers, blockfetch::CHANNEL_ID, [deep]).unwrap_err();
        assert_eq!(err.kind(), tokio::io::ErrorKind::InvalidData);
    }

    #[test]
    fn unsupported_channels_are_skipped() {
        let item = minicbor::to_vec((1u8, 2u8)).unwrap();
        let mut buffers = ChannelBuffers::new();
        let got = feed(&mut buffers, 999, [item.clone(), item]).unwrap();
        assert!(got.is_empty());
        assert!(buffers.channels[&999].bytes.is_empty());
    }

    #[test]
    fn reassembly_is_linear_in_the_bytes_received() {
        // A message that never ends, sent in 16-byte segments with limits off.
        let mut buffers = ChannelBuffers::new();
        buffers.disable_size_limits();

        let mut start = vec![0x82, 0x00, 0xbb];
        start.extend([0xff; 8]);
        buffers
            .absorb::<AnyMessage>(handshake::CHANNEL_ID, start)
            .unwrap();

        let started = std::time::Instant::now();
        for _ in 0..(1 << 20) / 16 {
            let msgs = buffers
                .absorb::<AnyMessage>(handshake::CHANNEL_ID, vec![0x00; 16])
                .unwrap();
            assert!(msgs.is_empty());
        }
        // Decoding the whole buffer on every segment takes minutes here.
        assert!(started.elapsed() < std::time::Duration::from_secs(5));
    }
}
