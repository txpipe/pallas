use super::Error;
use pallas_codec::minicbor::{Decode, Encode, Encoder, decode, encode};

/// Protocol channel number for node-to-node Keep-alive
pub const CHANNEL_ID: u16 = 8;

/// An opaque cookie value echoed back in keepalive responses.
pub type Cookie = u16;

/// Sub-state of the client side of the keepalive protocol.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ClientState {
    /// No keepalive in progress.
    Empty,
    /// A keepalive response was received with this cookie.
    Response(Cookie),
}

/// A keepalive mini-protocol message.
#[derive(Debug, Clone)]
pub enum Message {
    /// Client sends a keepalive request with a cookie.
    KeepAlive(Cookie),
    /// Server responds with the same cookie.
    ResponseKeepAlive(Cookie),
    /// The protocol is done.
    Done,
}

/// State machine for the keepalive mini-protocol.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum State {
    /// Agency belongs to the client.
    Client(ClientState),
    /// Agency belongs to the server; contains the pending cookie.
    Server(Cookie),
    /// The protocol has terminated.
    Done,
}

impl Default for State {
    fn default() -> Self {
        Self::Client(ClientState::Empty)
    }
}

impl State {
    /// Applies a message to the current state, returning the new state.
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
