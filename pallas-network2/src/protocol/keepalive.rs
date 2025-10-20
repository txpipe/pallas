use super::Error;
use pallas_codec::minicbor::{Decode, Encode, Encoder, decode, encode};

/// Protocol channel number for node-to-node Keep-alive
pub const CHANNEL_ID: u16 = 8;

pub type Cookie = u16;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ClientState {
    Empty,
    Response(Cookie),
}

#[derive(Debug, Clone)]
pub enum Message {
    KeepAlive(Cookie),
    ResponseKeepAlive(Cookie),
    Done,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum State {
    Client(ClientState),
    Server(Cookie),
    Done,
}

impl Default for State {
    fn default() -> Self {
        Self::Client(ClientState::Empty)
    }
}

impl State {
    pub fn apply(&self, msg: &Message) -> Result<Self, Error> {
        match self {
            State::Client(..) => match msg {
                Message::KeepAlive(x) => Ok(State::Server(*x)),
                _ => Err(Error::InvalidOutbound),
            },
            State::Server(_) => match msg {
                Message::ResponseKeepAlive(x) => Ok(State::Client(ClientState::Response(*x))),
                _ => Err(Error::InvalidInbound),
            },
            State::Done => Err(Error::InvalidOutbound),
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
            Message::KeepAlive(cookie) => {
                e.array(2)?.u16(0)?;
                e.encode(cookie)?;
            }
            Message::ResponseKeepAlive(cookie) => {
                e.array(2)?.u16(1)?;
                e.encode(cookie)?;
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
                let cookie = d.decode()?;
                Ok(Message::KeepAlive(cookie))
            }
            1 => {
                let cookie = d.decode()?;
                Ok(Message::ResponseKeepAlive(cookie))
            }
            2 => Ok(Message::Done),
            _ => Err(decode::Error::message("can't decode Message")),
        }
    }
}
