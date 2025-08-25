use pallas_codec::minicbor::{Decode, Encode, Encoder, decode, encode};
use std::net::{Ipv4Addr, Ipv6Addr};

use crate::protocol::Error;

/// Protocol channel number for node-to-node Peer-sharing
pub const CHANNEL_ID: u16 = 10;

pub type Port = u16;

pub type Amount = u8;

#[derive(Debug, Clone)]
pub enum Message {
    ShareRequest(Amount),
    SharePeers(Vec<PeerAddress>),
    Done,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum IdleState {
    Empty,
    Response(Vec<PeerAddress>),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum State {
    Idle(IdleState),
    Busy(Amount),
    Done,
}

impl Default for State {
    fn default() -> Self {
        Self::Idle(IdleState::Empty)
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum PeerAddress {
    V4(Ipv4Addr, Port),
    V6(Ipv6Addr, Port),
}

impl State {
    pub fn apply(&self, msg: &Message) -> Result<Self, Error> {
        match self {
            State::Idle(..) => match msg {
                Message::ShareRequest(x) => Ok(State::Busy(*x)),
                _ => Err(Error::InvalidOutbound),
            },
            State::Busy(..) => match msg {
                Message::SharePeers(x) => Ok(State::Idle(IdleState::Response(x.clone()))),
                _ => Err(Error::InvalidInbound),
            },
            State::Done => Err(Error::InvalidOutbound),
        }
    }
}

impl Encode<()> for PeerAddress {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        match self {
            PeerAddress::V4(address, port) => {
                e.array(3)?.u16(0)?;
                let word = address.to_bits();
                e.encode(word)?;
                e.encode(port)?;
            }
            PeerAddress::V6(address, port) => {
                e.array(8)?.u16(1)?;

                let bits: u128 = address.to_bits();
                let word1: u32 = (bits >> 96) as u32;
                let word2: u32 = ((bits >> 64) & 0xFFFF_FFFF) as u32;
                let word3: u32 = ((bits >> 32) & 0xFFFF_FFFF) as u32;
                let word4: u32 = (bits & 0xFFFF_FFFF) as u32;

                e.encode(word1)?;
                e.encode(word2)?;
                e.encode(word3)?;
                e.encode(word4)?;
                e.encode(port)?;
            }
        }

        Ok(())
    }
}

impl<'b> Decode<'b, ()> for PeerAddress {
    fn decode(
        d: &mut pallas_codec::minicbor::Decoder<'b>,
        _ctx: &mut (),
    ) -> Result<Self, decode::Error> {
        d.array()?;
        let label = d.u16()?;

        match label {
            0 => {
                let ip: u32 = d.decode()?;
                let address = Ipv4Addr::from(ip);
                let port = d.decode()?;
                Ok(PeerAddress::V4(address, port))
            }
            1 => {
                let word1: u32 = d.decode()?;
                let word2: u32 = d.decode()?;
                let word3: u32 = d.decode()?;
                let word4: u32 = d.decode()?;
                let bits: u128 = ((word1 as u128) << 96)
                    | ((word2 as u128) << 64)
                    | ((word3 as u128) << 32)
                    | (word4 as u128);

                let address = Ipv6Addr::from_bits(bits);
                let port: u16 = d.decode()?;
                Ok(PeerAddress::V6(address, port))
            }
            _ => Err(decode::Error::message("can't decode PeerAddress")),
        }
    }
}

impl Encode<()> for Message {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        match self {
            Message::ShareRequest(amount) => {
                e.array(2)?.u16(0)?;
                e.encode(amount)?;
            }
            Message::SharePeers(addresses) => {
                e.array(2)?.u16(1)?;
                e.begin_array()?;
                for address in addresses {
                    e.encode(address)?;
                }
                e.end()?;
            }
            Message::Done => {
                e.array(1)?.u16(2)?;
            }
        }

        Ok(())
    }
}

impl<'b> Decode<'b, ()> for Message {
    fn decode(
        d: &mut pallas_codec::minicbor::Decoder<'b>,
        _ctx: &mut (),
    ) -> Result<Self, decode::Error> {
        d.array()?;
        let label = d.u16()?;

        match label {
            0 => {
                let amount = d.decode()?;
                Ok(Message::ShareRequest(amount))
            }
            1 => {
                let addresses = d.decode()?;
                Ok(Message::SharePeers(addresses))
            }
            2 => Ok(Message::Done),
            _ => Err(decode::Error::message("can't decode Message")),
        }
    }
}
