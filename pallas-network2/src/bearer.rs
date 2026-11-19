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

use pallas_codec::scan::{ScanError, Scanner};

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
    /// unread are discarded with it. A decoder that consumes an item without
    /// returning a message skips it. It is then offered the rest of the
    /// segment's whole items in one call, and each item that call leaves is
    /// decoded alone.
    ///
    /// Fails with [`std::io::ErrorKind::InvalidData`], leaving nothing buffered
    /// for the channel, when a whole item does not decode as a message, when
    /// the call after a skipped item stops inside an item without returning a
    /// message, when CBOR nests deeper than the scanner allows, when bytes that
    /// are not well-formed CBOR follow an item skipped earlier in the same
    /// segment or do not decode as a message either, or, while `buffers` has
    /// its size limits on, when a message is larger than the
    /// `MESSAGE_SIZE_LIMIT` of its protocol module. Channels without one share
    /// a limit of 65,535 buffered bytes. Limits are checked after each segment,
    /// so at most one segment beyond them is buffered. Drop the connection on
    /// any error.
    pub async fn read_messages<M>(&mut self, buffers: &mut ChannelBuffers) -> IOResult<Vec<M>>
    where
        M: Message,
    {
        let (raw_channel, chunk) = self.read_segment().await?;
        let channel = raw_channel & !crate::protocol::PROTOCOL_SERVER;

        buffers.absorb(channel, chunk)
    }
}

/// The largest inbound message accepted on `channel`, in bytes, or `None` for
/// a channel with no limit of its own.
fn size_limit(channel: Channel) -> Option<usize> {
    let limit = match channel {
        proto::handshake::CHANNEL_ID => proto::handshake::MESSAGE_SIZE_LIMIT,
        proto::chainsync::CHANNEL_ID => proto::chainsync::MESSAGE_SIZE_LIMIT,
        proto::blockfetch::CHANNEL_ID => proto::blockfetch::MESSAGE_SIZE_LIMIT,
        proto::txsubmission::CHANNEL_ID => proto::txsubmission::MESSAGE_SIZE_LIMIT,
        proto::keepalive::CHANNEL_ID => proto::keepalive::MESSAGE_SIZE_LIMIT,
        proto::peersharing::CHANNEL_ID => proto::peersharing::MESSAGE_SIZE_LIMIT,
        proto::leiosnotify::CHANNEL_ID => proto::leiosnotify::MESSAGE_SIZE_LIMIT,
        proto::leiosfetch::CHANNEL_ID => proto::leiosfetch::MESSAGE_SIZE_LIMIT,
        _ => return None,
    };

    Some(limit)
}

/// Bytes that the channels with no limit of their own may buffer together.
const SHARED_SIZE_LIMIT: usize = crate::MAX_SEGMENT_PAYLOAD_LENGTH;

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
/// A channel keeps buffered state only while it holds part of a message.
#[derive(Debug)]
pub struct ChannelBuffers {
    channels: HashMap<Channel, PartialMessage>,
    /// Bytes buffered on the channels with no limit of their own.
    shared: usize,
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
            shared: 0,
            limits_enabled: true,
        }
    }

    /// Accept inbound messages of any size. Malformed messages still fail.
    pub fn disable_size_limits(&mut self) {
        self.limits_enabled = false;
    }

    fn absorb<M: Message>(&mut self, channel: Channel, chunk: Payload) -> IOResult<Vec<M>> {
        let own_limit = size_limit(channel);
        let limit = self
            .limits_enabled
            .then(|| own_limit.unwrap_or(SHARED_SIZE_LIMIT));

        let partial = self.channels.entry(channel).or_default();
        let held = partial.bytes.len();
        partial.bytes.extend(chunk);

        let result = Self::take_messages(partial, channel, limit);
        let held_now = match result {
            Ok(_) => partial.bytes.len(),
            Err(_) => 0,
        };

        if held_now == 0 {
            self.channels.remove(&channel);
        }

        if own_limit.is_none() {
            self.shared = self.shared + held_now - held;

            if self.limits_enabled && self.shared > SHARED_SIZE_LIMIT {
                let seen = self.shared;
                self.shared -= held_now;
                self.channels.remove(&channel);

                return Err(invalid_data(format!(
                    "{seen} bytes buffered on channels with no size limit of their own exceed \
                     their shared limit of {SHARED_SIZE_LIMIT} bytes"
                )));
            }
        }

        result
    }

    /// Decode every message the buffered bytes complete, then drop their bytes.
    fn take_messages<M: Message>(
        partial: &mut PartialMessage,
        channel: Channel,
        limit: Option<usize>,
    ) -> IOResult<Vec<M>> {
        let check = |len: usize| match limit {
            Some(limit) if len > limit => Err(size_limit_error(limit, len)),
            _ => Ok(()),
        };

        let mut msgs = Vec::new();
        // The first byte not yet taken, and the first byte the scanner sees.
        let mut start = 0;
        let mut origin = 0;
        // Where the whole items after one the decoder skipped begin, and where
        // each of them ends.
        let mut skipped_from = None;
        let mut ends = Vec::new();

        loop {
            let end = match partial.scanner.scan(&partial.bytes[origin..]) {
                Ok(Some(end)) => origin + end,
                Ok(None) => break,
                // Some encoders declare more array items than they write, and
                // their decoders ignore the count. Such a message ends at the
                // byte the scanner refuses; scanning restarts after it.
                Err(ScanError::Malformed) if skipped_from.is_none() => {
                    let end = origin + partial.scanner.position() + 1;
                    check(end - start)?;

                    let mut item = partial.bytes[start..end].to_vec();
                    let Some(msg) = M::from_payload(channel, &mut item) else {
                        return Err(invalid_data(ScanError::Malformed.to_string()));
                    };

                    msgs.push(msg);
                    start = end;
                    origin = end;
                    partial.scanner = Scanner::default();
                    continue;
                }
                Err(err) => return Err(invalid_data(err.to_string())),
            };

            check(end - start)?;

            if skipped_from.is_some() {
                ends.push(end);
            } else {
                let mut item = partial.bytes[start..end].to_vec();

                match decode_item(channel, &mut item)? {
                    Some(msg) => msgs.push(msg),
                    None => skipped_from = Some(end),
                }
            }

            start = end;
        }

        // A decoder that consumes an item without a message skips it, as for
        // a channel it does not handle. It is offered the whole items after it
        // in one payload, which such a decoder clears in one call.
        if let Some(from) = skipped_from
            && from < start
        {
            let mut item = partial.bytes[from..start].to_vec();
            let before = item.len();
            let msg = M::from_payload(channel, &mut item);

            // The rest of the item the call stopped in is discarded, as when it
            // is decoded alone. A call that returns no message must end an item.
            let used = start - item.len().min(before);
            let next = ends.partition_point(|&end| end < used);

            if msg.is_none() && ends[next] > used {
                return Err(invalid_data(format!(
                    "CBOR item of {} bytes after a skipped one is not a valid message",
                    ends[next] - ends[..next].last().map_or(from, |&end| end)
                )));
            }

            msgs.extend(msg);

            // Each item the call left is decoded alone, so the work stays
            // linear whatever the decoder consumes per call.
            for pair in ends[next..].windows(2) {
                item.clear();
                item.extend_from_slice(&partial.bytes[pair[0]..pair[1]]);
                msgs.extend(decode_item(channel, &mut item)?);
            }
        }

        partial.scanner.remove_prefix(start - origin);

        // The bytes left all arrived in this segment, after the last message
        // it completed, so moving them out of the old buffer stays linear.
        if start > 0 {
            partial.bytes = partial.bytes.split_off(start);
            partial.scanner.shrink_to_fit();
        }

        check(partial.bytes.len())?;

        Ok(msgs)
    }
}

/// Decode one whole CBOR item: a message, `None` when the decoder consumes the
/// item without one, or an error when it returns none and leaves bytes of it.
/// Bytes of the item the decoder leaves unread are discarded.
fn decode_item<M: Message>(channel: Channel, item: &mut Payload) -> IOResult<Option<M>> {
    let len = item.len();

    match M::from_payload(channel, item) {
        Some(msg) => {
            if !item.is_empty() {
                tracing::debug!(len = item.len(), "discarding bytes left in a decoded item");
            }

            Ok(Some(msg))
        }
        None if item.is_empty() => Ok(None),
        None => Err(invalid_data(format!(
            "CBOR item of {len} bytes is not a valid message"
        ))),
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
                assert!(buffers.channels.is_empty());
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
                peersharing::CHANNEL_ID,
                peersharing::MESSAGE_SIZE_LIMIT,
                Box::new(|n| {
                    let peer = peersharing::PeerAddress::V4(Ipv4Addr::new(10, 0, 0, 1), 3001);
                    AnyMessage::PeerSharing(peersharing::Message::SharePeers(vec![peer; n / 10]))
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
            assert_eq!(size_limit(channel), Some(limit));
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

        // Keep-alive messages cannot grow near their limit; a message that
        // never ends checks it below.
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
            let limit = size_limit(channel).unwrap_or(SHARED_SIZE_LIMIT);
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
        assert!(buffers.channels.is_empty());
    }

    /// Feed a message that never ends to `channel` in 16-byte segments, up to
    /// `len` bytes. Returns the time taken and how often the buffer moved.
    fn never_ending(
        buffers: &mut ChannelBuffers,
        channel: Channel,
        len: usize,
    ) -> (std::time::Duration, usize) {
        let mut start = vec![0x82, 0x00, 0xbb];
        start.extend([0xff; 8]);
        buffers.absorb::<AnyMessage>(channel, start).unwrap();

        let mut moves = 0;
        let mut at = buffers.channels[&channel].bytes.as_ptr();
        let started = std::time::Instant::now();

        for _ in 0..len / 16 {
            let msgs = buffers
                .absorb::<AnyMessage>(channel, vec![0x00; 16])
                .unwrap();
            assert!(msgs.is_empty());

            let now = buffers.channels[&channel].bytes.as_ptr();
            moves += usize::from(now != at);
            at = now;
        }

        (started.elapsed(), moves)
    }

    #[test]
    fn reassembly_is_linear_in_the_bytes_received() {
        let mut buffers = ChannelBuffers::new();
        buffers.disable_size_limits();
        let (elapsed, _) = never_ending(&mut buffers, handshake::CHANNEL_ID, 1 << 20);
        // Decoding the whole buffer on every segment takes minutes here.
        assert!(elapsed < std::time::Duration::from_secs(5), "{elapsed:?}");

        // With limits on, on the channel with the largest one. Copying the
        // whole buffer on every segment moves it every time, and copies
        // hundreds of gigabytes.
        let mut buffers = ChannelBuffers::new();
        let (elapsed, moves) = never_ending(&mut buffers, leiosfetch::CHANNEL_ID, 4 << 20);
        assert!(elapsed < std::time::Duration::from_secs(5), "{elapsed:?}");
        assert!(moves < 64, "the buffer moved {moves} times");
    }

    #[test]
    fn limits_match_the_network_specification() {
        for (channel, limit) in [
            (handshake::CHANNEL_ID, 5_760),
            (chainsync::CHANNEL_ID, 65_535),
            (blockfetch::CHANNEL_ID, 2_500_000),
            (txsubmission::CHANNEL_ID, 2_500_000),
            (keepalive::CHANNEL_ID, 65_535),
            (peersharing::CHANNEL_ID, 5_760),
            (leiosnotify::CHANNEL_ID, 65_535),
            (leiosfetch::CHANNEL_ID, 16_777_216),
        ] {
            assert_eq!(size_limit(channel), Some(limit), "channel {channel}");
        }

        assert_eq!(size_limit(999), None);
        assert_eq!(SHARED_SIZE_LIMIT, 65_535);
    }

    #[test]
    fn channels_without_a_limit_share_one() {
        // The start of a message that never ends, on one channel after another.
        let mut segment = vec![0x82, 0x00, 0xbb];
        segment.extend([0xff; 8]);
        segment.resize(1_000, 0x00);

        let mut buffers = ChannelBuffers::new();
        let mut channels = 1_000..;
        let err = loop {
            let channel = channels.next().unwrap();
            match buffers.absorb::<AnyMessage>(channel, segment.clone()) {
                Ok(msgs) => assert!(msgs.is_empty()),
                Err(err) => break err,
            }
        };

        assert_eq!(err.kind(), tokio::io::ErrorKind::InvalidData);
        assert!(
            err.to_string()
                .ends_with("exceed their shared limit of 65535 bytes"),
            "{err}"
        );
        assert_eq!(buffers.channels.len(), 65);
        assert_eq!(buffers.shared, 65_000);

        // Channels with their own limit are not counted against it.
        let mut partial = AnyMessage::BlockFetch(blockfetch::Message::Block(vec![1; 500_000]))
            .payload()
            .chunks(crate::MAX_SEGMENT_PAYLOAD_LENGTH)
            .map(Vec::from)
            .collect::<Vec<_>>();
        partial.pop();
        assert!(
            feed(&mut buffers, blockfetch::CHANNEL_ID, partial)
                .unwrap()
                .is_empty()
        );
        assert_eq!(buffers.shared, 65_000);

        // The opt-out lifts the shared limit too.
        let mut buffers = ChannelBuffers::new();
        buffers.disable_size_limits();
        for channel in 1_000..1_100 {
            assert!(
                feed(&mut buffers, channel, [segment.clone()])
                    .unwrap()
                    .is_empty()
            );
        }
        assert_eq!(buffers.channels.len(), 100);
    }

    #[test]
    fn whole_items_on_skipped_channels_leave_nothing_buffered() {
        // Nesting that keeps a deep scanner stack until the item ends.
        let mut deep = vec![0x81; 60_000];
        deep.push(0x00);

        let mut buffers = ChannelBuffers::new();
        for channel in 1_000..1_100 {
            assert!(
                feed(&mut buffers, channel, [deep.clone()])
                    .unwrap()
                    .is_empty()
            );
        }
        assert!(buffers.channels.is_empty());
        assert_eq!(buffers.shared, 0);

        // The start of the next item is all a channel keeps.
        let mut segment = deep.clone();
        segment.push(0x82);
        assert!(feed(&mut buffers, 1_000, [segment]).unwrap().is_empty());
        let partial = &buffers.channels[&1_000];
        assert_eq!(partial.bytes, [0x82]);
        assert!(partial.bytes.capacity() < 64);
        assert!(partial.scanner.capacity() < 64);
        assert_eq!(buffers.shared, 1);
    }

    thread_local! {
        static SKIPPER_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    }

    /// A decoder that skips every payload, counting the calls.
    #[derive(Debug, Clone)]
    struct Skipper;

    impl Message for Skipper {
        fn channel(&self) -> Channel {
            999
        }

        fn payload(&self) -> Payload {
            vec![]
        }

        fn from_payload(_: Channel, payload: &mut Payload) -> Option<Self> {
            SKIPPER_CALLS.with(|calls| calls.set(calls.get() + 1));
            payload.clear();
            None
        }

        fn into_payload(self) -> (Channel, Payload) {
            (999, vec![])
        }
    }

    #[test]
    fn a_skipped_channel_is_decoded_at_most_twice_per_segment() {
        let calls = || SKIPPER_CALLS.with(|calls| calls.replace(0));
        let mut buffers = ChannelBuffers::new();

        let segment = vec![0x00; crate::MAX_SEGMENT_PAYLOAD_LENGTH];
        assert!(buffers.absorb::<Skipper>(999, segment).unwrap().is_empty());
        assert_eq!(calls(), 2);
        assert!(buffers.channels.is_empty());

        // Whole items are skipped, and the start of the next one waits.
        let mut segment = vec![0x00; 100];
        segment.extend([0x82, 0x00]);
        assert!(buffers.absorb::<Skipper>(999, segment).unwrap().is_empty());
        assert_eq!(calls(), 2);
        assert_eq!(buffers.channels[&999].bytes, [0x82, 0x00]);
        assert_eq!(buffers.shared, 2);

        assert!(
            buffers
                .absorb::<Skipper>(999, vec![0x00])
                .unwrap()
                .is_empty()
        );
        assert_eq!(calls(), 1);
        assert!(buffers.channels.is_empty());
        assert_eq!(buffers.shared, 0);
    }

    thread_local! {
        static PICKY_OFFERED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    }

    /// A decoder of small integers: it skips 0, decodes 1 to 8 and the
    /// two-byte item `0x18 n`, and refuses anything else without consuming
    /// it. It waits without consuming while `0x18 n` is incomplete. It reads
    /// only the head of `0x82 n ..`, skips only the head of `0x81 ..`, and
    /// decodes 16 without consuming it. It skips up to three bytes from `0x21`, and
    /// on `0x22` it appends ten bytes and returns no message. It counts the
    /// bytes it is offered.
    #[derive(Debug, Clone, PartialEq)]
    struct Picky(u8);

    impl Message for Picky {
        fn channel(&self) -> Channel {
            999
        }

        fn payload(&self) -> Payload {
            vec![self.0]
        }

        fn from_payload(_: Channel, payload: &mut Payload) -> Option<Self> {
            PICKY_OFFERED.with(|offered| offered.set(offered.get() + payload.len()));

            match payload.first().copied()? {
                0 | 0x81 => {
                    payload.remove(0);
                    None
                }
                n @ 1..=8 => {
                    payload.remove(0);
                    Some(Picky(n))
                }
                0x10 => Some(Picky(0x10)),
                0x21 => {
                    payload.drain(..payload.len().min(3));
                    None
                }
                0x22 => {
                    payload.extend([0; 10]);
                    None
                }
                0x18 | 0x82 => {
                    let n = *payload.get(1)?;
                    payload.drain(..2);
                    Some(Picky(n))
                }
                _ => None,
            }
        }

        fn into_payload(self) -> (Channel, Payload) {
            (999, vec![self.0])
        }
    }

    #[test]
    fn messages_after_a_skipped_item_are_all_decoded() {
        let feed_picky = |bytes: &[u8], split: usize| {
            let mut buffers = ChannelBuffers::new();
            let mut out = vec![];
            for chunk in [&bytes[..split], &bytes[split..]] {
                match buffers.absorb::<Picky>(999, chunk.to_vec()) {
                    Ok(msgs) => out.extend(msgs),
                    Err(err) => {
                        assert!(buffers.channels.is_empty());
                        return Err(err);
                    }
                }
            }
            assert!(buffers.channels.is_empty());
            Ok(out)
        };

        // The fifth case splits inside its final item after a skipped one.
        // After it, an item gives the same messages after a skipped one as
        // alone: the bytes the decoder leaves unread in it are discarded.
        let cases: [(&[u8], &[u8]); 11] = [
            (&[0, 1, 0, 2], &[1, 2]),
            (&[0, 0, 1], &[1]),
            (&[0, 0, 0, 3, 0, 4, 5], &[3, 4, 5]),
            (&[1, 0, 2], &[1, 2]),
            (&[0, 1, 0x18, 5], &[1, 5]),
            (&[0x82, 1, 2], &[1]),
            (&[0, 0x82, 1, 2], &[1]),
            (&[0, 0x82, 1, 0x82, 7, 0], &[1]),
            (&[0, 0, 0x82, 1, 2, 0x82, 3, 4, 5], &[1, 3, 5]),
            (&[0x10, 1], &[16, 1]),
            (&[0, 0x10, 1], &[16, 1]),
        ];

        for (bytes, expected) in cases {
            let expected: Vec<_> = expected.iter().map(|&n| Picky(n)).collect();
            for split in 0..=bytes.len() {
                assert_eq!(
                    feed_picky(bytes, split).unwrap(),
                    expected,
                    "{bytes:?} split {split}"
                );
            }
        }

        // An item the decoder refuses, or skips only in part, fails after a
        // skipped item as it does alone. That holds when the call after the
        // skipped item stops inside a later item than the first, and when the
        // decoder grows its payload instead of consuming it.
        let refused: [&[u8]; 9] = [
            &[9, 3],
            &[0, 9, 3],
            &[0, 1, 9],
            &[0x81, 0],
            &[0, 0x81, 0],
            &[0, 1, 0x81, 0],
            &[0, 0x21, 0, 0x81, 0],
            &[0x22],
            &[0, 0x22],
        ];
        for bytes in refused {
            for split in 0..=bytes.len() {
                let err = feed_picky(bytes, split).unwrap_err();
                assert_eq!(
                    err.kind(),
                    tokio::io::ErrorKind::InvalidData,
                    "{bytes:?} split {split}"
                );
            }
        }
    }

    #[test]
    fn items_after_a_skipped_one_are_decoded_in_linear_time() {
        let offered = || PICKY_OFFERED.with(|offered| offered.replace(0));
        offered();

        // Each call consumes one item, so decoding the segment's items from
        // one payload would offer about half the segment's square in bytes.
        let len = crate::MAX_SEGMENT_PAYLOAD_LENGTH;
        let mut buffers = ChannelBuffers::new();
        let msgs = buffers.absorb::<Picky>(999, vec![0x00; len]).unwrap();
        assert!(msgs.is_empty());
        assert!(buffers.channels.is_empty());
        assert!(offered() <= 3 * len);

        let mut segment = vec![0x00; len - 1];
        segment.push(0x01);
        let msgs = buffers.absorb::<Picky>(999, segment).unwrap();
        assert_eq!(msgs, [Picky(1)]);
        assert!(offered() <= 3 * len);
    }

    #[test]
    fn peer_addresses_from_released_encoders_are_accepted() {
        // Released encoders declare eight items for an IPv6 address and write six.
        let old_v6 = [0x88, 0x01, 0x00, 0x00, 0x00, 0x01, 0x19, 0x0b, 0xb9];
        let v6 = peersharing::PeerAddress::V6(Ipv6Addr::LOCALHOST, 3001);
        let v4 = peersharing::PeerAddress::V4(Ipv4Addr::new(10, 0, 0, 1), 3001);
        let share = |peers| AnyMessage::PeerSharing(peersharing::Message::SharePeers(peers));

        let v4_bytes = share(vec![v4.clone()]).payload();
        assert_eq!(v4_bytes[..3], [0x82, 0x01, 0x9f]);
        let v4_bytes = v4_bytes[3..v4_bytes.len() - 1].to_vec();

        let old = |entries: &[&[u8]]| {
            let mut bytes = vec![0x82, 0x01, 0x9f];
            bytes.extend(entries.concat());
            bytes.push(0xff);
            bytes
        };
        let mut back_to_back = old(&[&old_v6, &old_v6]);
        back_to_back.extend(old(&[&old_v6]));

        let cases = [
            (old(&[&old_v6]), vec![vec![v6.clone()]]),
            (
                old(&[&v4_bytes, &old_v6, &v4_bytes]),
                vec![vec![v4.clone(), v6.clone(), v4.clone()]],
            ),
            (old(&[&old_v6, &old_v6]), vec![vec![v6.clone(), v6.clone()]]),
            (
                back_to_back,
                vec![vec![v6.clone(), v6.clone()], vec![v6.clone()]],
            ),
        ];

        let done = AnyMessage::PeerSharing(peersharing::Message::Done).payload();

        for (bytes, lists) in cases {
            let mut expected: Vec<_> = lists.into_iter().map(|p| share(p).payload()).collect();
            expected.push(done.clone());
            let mut both = bytes.clone();
            both.extend(&done);

            for split in 0..=both.len() {
                let mut buffers = ChannelBuffers::new();
                let chunks = [both[..split].to_vec(), both[split..].to_vec()];
                let got = feed(&mut buffers, peersharing::CHANNEL_ID, chunks).unwrap();
                assert_eq!(payloads(&got), expected, "{bytes:02x?} split {split}");
                assert!(buffers.channels.is_empty());
            }
        }

        // Malformed bytes the decoder refuses still fail: an unknown address kind.
        let mut bytes = old(&[&old_v6]);
        bytes[4] = 0x05;
        let mut buffers = ChannelBuffers::new();
        let err = feed(&mut buffers, peersharing::CHANNEL_ID, [bytes]).unwrap_err();
        assert_eq!(err.kind(), tokio::io::ErrorKind::InvalidData);
    }
}
