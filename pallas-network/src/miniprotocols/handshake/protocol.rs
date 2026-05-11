use itertools::Itertools;
use pallas_codec::minicbor::{Decode, Decoder, Encode, Encoder, decode, encode};
use std::{collections::HashMap, fmt::Debug};
use thiserror::*;

use crate::multiplexer;

/// Errors produced by the handshake protocol.
#[derive(Error, Debug)]
pub enum Error {
    /// Tried to receive while we hold agency.
    #[error("attempted to receive message while agency is ours")]
    AgencyIsOurs,

    /// Tried to send while the peer holds agency.
    #[error("attempted to send message while agency is theirs")]
    AgencyIsTheirs,

    /// Inbound message is not valid for the current state.
    #[error("inbound message is not valid for current state")]
    InvalidInbound,

    /// Outbound message is not valid for the current state.
    #[error("outbound message is not valid for current state")]
    InvalidOutbound,

    /// Underlying multiplexer error.
    #[error("error while sending or receiving data through the channel")]
    Plexer(multiplexer::Error),
}

/// Map of protocol version numbers to their version-specific payload.
#[derive(Debug, Clone)]
pub struct VersionTable<T>
where
    T: Debug + Clone,
{
    /// Underlying version → version-data map.
    pub values: HashMap<u64, T>,
}

impl<T> Encode<()> for VersionTable<T>
where
    T: Debug + Clone + Encode<()>,
{
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        e.map(self.values.len() as u64)?;

        for key in self.values.keys().sorted() {
            e.u64(*key)?;
            e.encode(&self.values[key])?;
        }

        Ok(())
    }
}

impl<'b, T> Decode<'b, ()> for VersionTable<T>
where
    T: Debug + Clone + Decode<'b, ()>,
{
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        let len = d.map()?.ok_or(decode::Error::message(
            "expected def-length map for versiontable",
        ))?;
        let mut values = HashMap::new();

        for _ in 0..len {
            let key = d.u64()?;
            let value = d.decode()?;
            values.insert(key, value);
        }
        Ok(VersionTable { values })
    }
}

/// Network identifier exchanged at handshake (mainnet, testnet, …).
pub type NetworkMagic = u64;

/// Numeric protocol version, where higher means more recent.
pub type VersionNumber = u64;

/// Handshake protocol message.
#[derive(Debug)]
pub enum Message<D>
where
    D: Debug + Clone,
{
    /// Initiator → responder: offered versions and their payloads.
    Propose(VersionTable<D>),
    /// Responder → initiator: accepted version plus the agreed payload.
    Accept(VersionNumber, D),
    /// Responder → initiator: handshake refused for a stated reason.
    Refuse(RefuseReason),
    /// Responder → initiator: query-mode response listing supported versions.
    QueryReply(VersionTable<D>),
}

impl<D> Encode<()> for Message<D>
where
    D: Debug + Clone,
    D: Encode<()>,
    VersionTable<D>: Encode<()>,
{
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        match self {
            Message::Propose(version_table) => {
                e.array(2)?.u16(0)?;
                e.encode(version_table)?;
            }
            Message::Accept(version_number, version_data) => {
                e.array(3)?.u16(1)?;
                e.u64(*version_number)?;
                e.encode(version_data)?;
            }
            Message::Refuse(reason) => {
                e.array(2)?.u16(2)?;
                e.encode(reason)?;
            }
            Message::QueryReply(version_table) => {
                e.array(2)?.u16(3)?;
                e.encode(version_table)?;
            }
        };

        Ok(())
    }
}

impl<'b, D> Decode<'b, ()> for Message<D>
where
    D: Decode<'b, ()> + Debug + Clone,
    VersionTable<D>: Decode<'b, ()>,
{
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        d.array()?;

        match d.u16()? {
            0 => {
                let version_table = d.decode()?;
                Ok(Message::Propose(version_table))
            }
            1 => {
                let version_number = d.u64()?;
                let version_data = d.decode()?;
                Ok(Message::Accept(version_number, version_data))
            }
            2 => {
                let reason: RefuseReason = d.decode()?;
                Ok(Message::Refuse(reason))
            }
            3 => {
                let version_table = d.decode()?;
                Ok(Message::QueryReply(version_table))
            }
            _ => Err(decode::Error::message(
                "unknown variant for handshake message",
            )),
        }
    }
}

/// Handshake state-machine state.
#[derive(Debug, PartialEq, Eq)]
pub enum State {
    /// Initiator side: waiting to send `Propose`.
    Propose,
    /// Responder side: waiting to send `Accept`/`Refuse`/`QueryReply`.
    Confirm,
    /// Protocol terminated.
    Done,
}

/// Reason a responder gave for refusing the handshake.
#[derive(Debug)]
pub enum RefuseReason {
    /// None of the offered versions overlap with the responder's set.
    VersionMismatch(Vec<VersionNumber>),
    /// The version-data payload for the chosen version failed to decode.
    HandshakeDecodeError(VersionNumber, String),
    /// The responder refused the chosen version with an explanatory message.
    Refused(VersionNumber, String),
}

impl Encode<()> for RefuseReason {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        match self {
            RefuseReason::VersionMismatch(versions) => {
                e.array(2)?;
                e.u16(0)?;
                e.array(versions.len() as u64)?;
                for v in versions.iter() {
                    e.u64(*v)?;
                }

                Ok(())
            }
            RefuseReason::HandshakeDecodeError(version, msg) => {
                e.array(3)?;
                e.u16(1)?;
                e.u64(*version)?;
                e.str(msg)?;

                Ok(())
            }
            RefuseReason::Refused(version, msg) => {
                e.array(3)?;
                e.u16(2)?;
                e.u64(*version)?;
                e.str(msg)?;

                Ok(())
            }
        }
    }
}

impl<'b> Decode<'b, ()> for RefuseReason {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        d.array()?;

        match d.u16()? {
            0 => {
                let versions = d.array_iter::<u64>()?;
                let versions = versions.try_collect()?;
                Ok(RefuseReason::VersionMismatch(versions))
            }
            1 => {
                let version = d.u64()?;
                let msg = d.str()?;

                Ok(RefuseReason::HandshakeDecodeError(version, msg.to_string()))
            }
            2 => {
                let version = d.u64()?;
                let msg = d.str()?;

                Ok(RefuseReason::Refused(version, msg.to_string()))
            }
            _ => Err(decode::Error::message("unknown variant for refusereason")),
        }
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "blueprint")]
    #[test]
    fn message_roundtrip() {
        use super::Message;
        use pallas_codec::minicbor;
        use pallas_codec::utils;

        macro_rules! include_test_msg {
            ($path:literal) => {
                include_str!(concat!(
                    "../../../../cardano-blueprint/src/network/node-to-node/handshake/test-data/",
                    $path
                ))
            };
        }

        let test_messages = [
            include_test_msg!("test-0"),
            include_test_msg!("test-1"),
            include_test_msg!("test-2"),
            include_test_msg!("test-3"),
            include_test_msg!("test-4"),
        ];

        for (idx, message_str) in test_messages.iter().enumerate() {
            println!("Decoding test message {}", idx + 1);
            let bytes =
                hex::decode(message_str).unwrap_or_else(|_| panic!("bad message file {idx}"));

            let message: Message<utils::AnyCbor> = minicbor::decode(&bytes[..])
                .unwrap_or_else(|e| panic!("error decoding cbor for file {idx}: {e:?}"));
            println!("Decoded message: {:#?}", message);

            let bytes2 = minicbor::to_vec(message)
                .unwrap_or_else(|e| panic!("error encoding cbor for file {idx}: {e:?}"));

            assert!(
                bytes.eq(&bytes2),
                "re-encoded bytes didn't match original file {idx}"
            );
        }
    }
}
