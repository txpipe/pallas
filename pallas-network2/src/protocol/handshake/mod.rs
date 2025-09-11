use std::collections::HashMap;
use std::fmt::Debug;

use pallas_codec::minicbor::{Decode, Decoder, Encode, Encoder, decode, encode};

use crate::protocol::Error;

pub mod n2c;
pub mod n2n;

/// Protocol channel number for node-to-node handshakes
pub const CHANNEL_ID: u16 = 0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionTable<T>
where
    T: Debug + Clone,
{
    pub values: HashMap<u64, T>,
}

pub type NetworkMagic = u64;

pub type VersionNumber = u64;

#[derive(Debug, Clone)]
pub enum Message<D>
where
    D: Debug + Clone,
{
    Propose(VersionTable<D>),
    Accept(VersionNumber, D),
    Refuse(RefuseReason),
    QueryReply(VersionTable<D>),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum DoneState<D>
where
    D: Debug + Clone,
{
    Accepted(VersionNumber, D),
    Rejected(RefuseReason),
    QueryReply(VersionTable<D>),
}

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub enum State<D>
where
    D: Debug + Clone,
{
    #[default]
    Propose,
    Confirm(VersionTable<D>),
    Done(DoneState<D>),
}

impl<D> State<D>
where
    D: Debug + Clone,
{
    pub fn apply(&self, msg: &Message<D>) -> Result<Self, Error> {
        match self {
            State::Propose => match msg {
                Message::Propose(x) => Ok(State::Confirm(x.clone())),
                _ => Err(Error::InvalidOutbound),
            },
            State::Confirm(..) => match msg {
                Message::Accept(x, y) => Ok(State::Done(DoneState::Accepted(*x, y.clone()))),
                Message::Refuse(x) => Ok(State::Done(DoneState::Rejected(x.clone()))),
                Message::QueryReply(x) => Ok(State::Done(DoneState::QueryReply(x.clone()))),
                _ => Err(Error::InvalidInbound),
            },
            State::Done(..) => Err(Error::InvalidInbound),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefuseReason {
    VersionMismatch(Vec<VersionNumber>),
    HandshakeDecodeError(VersionNumber, String),
    Refused(VersionNumber, String),
}

impl<T> Encode<()> for VersionTable<T>
where
    T: std::fmt::Debug + Clone + Encode<()>,
{
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        e.map(self.values.len() as u64)?;

        let mut keys = self.values.keys().collect::<Vec<_>>();
        keys.sort();

        for key in keys {
            e.u64(*key)?;
            e.encode(&self.values[key])?;
        }

        Ok(())
    }
}

impl<'b, T> Decode<'b, ()> for VersionTable<T>
where
    T: std::fmt::Debug + Clone + Decode<'b, ()>,
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

impl<D> Encode<()> for Message<D>
where
    D: std::fmt::Debug + Clone,
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
    D: Decode<'b, ()> + std::fmt::Debug + Clone,
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
                let versions: Vec<u64> = versions.collect::<Result<_, _>>()?;
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
