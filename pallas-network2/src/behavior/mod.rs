//! Opinionated standard behavior for Cardano networks

use pallas_codec::{Fragment, minicbor};

use crate::{Channel, Message, Payload, protocol as proto};

pub mod initiator;
pub mod responder;

// Re-export initiator types for backward compatibility
pub use initiator::*;

/// A unified message type that wraps all supported mini-protocol messages.
#[derive(Debug, Clone)]
pub enum AnyMessage {
    /// A handshake protocol message.
    Handshake(proto::handshake::Message<proto::handshake::n2n::VersionData>),
    /// A keepalive protocol message.
    KeepAlive(proto::keepalive::Message),
    /// A chain-sync protocol message.
    ChainSync(proto::chainsync::Message<proto::chainsync::HeaderContent>),
    /// A peer-sharing protocol message.
    PeerSharing(proto::peersharing::Message),
    /// A block-fetch protocol message.
    BlockFetch(proto::blockfetch::Message),
    /// A tx-submission protocol message.
    TxSubmission(proto::txsubmission::Message),
}

fn try_decode_msg<T: Fragment>(buffer: &mut Vec<u8>) -> Option<T> {
    let mut decoder = minicbor::Decoder::new(buffer);
    let maybe_msg: Result<T, _> = decoder.decode();

    match maybe_msg {
        Ok(msg) => {
            let new_pos = decoder.position();
            buffer.drain(0..new_pos);
            Some(msg)
        }
        Err(err) if err.is_end_of_input() => None,
        Err(err) => {
            tracing::error!(?err);
            None
        }
    }
}

impl Message for AnyMessage {
    fn channel(&self) -> u16 {
        match self {
            AnyMessage::Handshake(_) => proto::handshake::CHANNEL_ID,
            AnyMessage::KeepAlive(_) => proto::keepalive::CHANNEL_ID,
            AnyMessage::ChainSync(_) => proto::chainsync::CHANNEL_ID,
            AnyMessage::PeerSharing(_) => proto::peersharing::CHANNEL_ID,
            AnyMessage::BlockFetch(_) => proto::blockfetch::CHANNEL_ID,
            AnyMessage::TxSubmission(_) => proto::txsubmission::CHANNEL_ID,
        }
    }

    fn payload(&self) -> Vec<u8> {
        match self {
            AnyMessage::Handshake(msg) => pallas_codec::minicbor::to_vec(msg).unwrap(),
            AnyMessage::KeepAlive(msg) => pallas_codec::minicbor::to_vec(msg).unwrap(),
            AnyMessage::ChainSync(msg) => pallas_codec::minicbor::to_vec(msg).unwrap(),
            AnyMessage::PeerSharing(msg) => pallas_codec::minicbor::to_vec(msg).unwrap(),
            AnyMessage::BlockFetch(msg) => pallas_codec::minicbor::to_vec(msg).unwrap(),
            AnyMessage::TxSubmission(msg) => pallas_codec::minicbor::to_vec(msg).unwrap(),
        }
    }

    fn from_payload(channel: Channel, payload: &mut Payload) -> Option<Self> {
        match channel {
            proto::handshake::CHANNEL_ID => try_decode_msg(payload).map(AnyMessage::Handshake),
            proto::keepalive::CHANNEL_ID => try_decode_msg(payload).map(AnyMessage::KeepAlive),
            proto::chainsync::CHANNEL_ID => try_decode_msg(payload).map(AnyMessage::ChainSync),
            proto::peersharing::CHANNEL_ID => try_decode_msg(payload).map(AnyMessage::PeerSharing),
            proto::blockfetch::CHANNEL_ID => try_decode_msg(payload).map(AnyMessage::BlockFetch),
            proto::txsubmission::CHANNEL_ID => {
                try_decode_msg(payload).map(AnyMessage::TxSubmission)
            }
            x => {
                tracing::warn!(channel = x, "unsupported channel, skipping payload");
                payload.clear();
                None
            }
        }
    }

    fn into_payload(self) -> (Channel, Payload) {
        let channel = self.channel();
        let payload = self.payload();

        (channel, payload)
    }
}

/// Timestamp of the last observed activity for a peer.
pub type LastSeen = chrono::DateTime<chrono::Utc>;

/// The high-level connection state of a peer.
#[derive(PartialEq, Debug, Default)]
pub enum ConnectionState {
    /// Peer was discovered but no connection attempt has been made.
    #[default]
    New,
    /// A connection attempt is in progress.
    Connecting,
    /// TCP connection established, handshake not yet complete.
    Connected,
    /// Handshake completed successfully; mini-protocols are active.
    Initialized,
    /// The peer has been disconnected.
    Disconnected,
    /// An error occurred on this peer's connection.
    Errored,
}

/// A range of blocks defined by start and end points.
pub type BlockRange = (proto::Point, proto::Point);

/// The accepted version number and data from a successful N2N handshake.
pub type AcceptedVersion = (u64, proto::handshake::n2n::VersionData);
