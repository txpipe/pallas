use std::net::{Ipv4Addr, Ipv6Addr};

use super::protocol::*;
use pallas_codec::minicbor::{decode, encode, Decode, Encode, Encoder};

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
            PeerAddress::V6(address, flow_info, scope_id, port) => {
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
                e.encode(flow_info)?;
                e.encode(scope_id)?;
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
                let flow_info: u32 = d.decode()?;
                let scope_id: u32 = d.decode()?;
                let port: u32 = d.decode()?;
                Ok(PeerAddress::V6(address, flow_info, scope_id, port))
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
