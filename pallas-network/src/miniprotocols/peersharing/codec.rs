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
                e.encode(address)?;
                e.encode(port)?;
            }
            PeerAddress::V6(address, port) => {
                e.array(3)?.u16(1)?;
                e.encode(address)?;
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
                let address = d.decode()?;
                let port = d.decode()?;
                Ok(PeerAddress::V4(address, port))
            }
            1 => {
                let address = d.decode()?;
                let port = d.decode()?;
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
