//! Opinionated standard behavior for Cardano networks

use pallas_codec::{Fragment, minicbor};

use crate::{Channel, Message, Payload, protocol as proto};

pub mod initiator;
pub mod responder;

// Re-export initiator types for backward compatibility
pub use initiator::*;

#[derive(Debug, Clone)]
pub enum AnyMessage {
    Handshake(proto::handshake::Message<proto::handshake::n2n::VersionData>),
    KeepAlive(proto::keepalive::Message),
    ChainSync(proto::chainsync::Message<proto::chainsync::HeaderContent>),
    PeerSharing(proto::peersharing::Message),
    BlockFetch(proto::blockfetch::Message),
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

pub type LastSeen = chrono::DateTime<chrono::Utc>;

#[derive(PartialEq, Debug, Default)]
pub enum ConnectionState {
    #[default]
    New,
    Connecting,
    Connected,
    Initialized,
    Disconnected,
    Errored,
}

pub type BlockRange = (proto::Point, proto::Point);

pub type AcceptedVersion = (u64, proto::handshake::n2n::VersionData);
