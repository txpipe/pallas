use pallas_codec::Fragment;
use std::marker::PhantomData;
use thiserror::Error;
use tracing::debug;

use crate::miniprotocols::Point;
use crate::multiplexer;

use super::{BlockContent, HeaderContent, Message, State, Tip};

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("attempted to receive message while agency is ours")]
    AgencyIsOurs,

    #[error("attempted to send message while agency is theirs")]
    AgencyIsTheirs,

    #[error("inbound message is not valid for current state")]
    InvalidInbound,

    #[error("outbound message is not valid for current state")]
    InvalidOutbound,

    #[error("error while sending or receiving data through the channel")]
    Plexer(multiplexer::Error),
}

#[derive(Debug)]
pub enum ClientRequest {
    Intersect(Vec<Point>),
    RequestNext,
}

pub struct Server<O>(State, multiplexer::ChannelBuffer, PhantomData<O>)
where
    Message<O>: Fragment;

impl<O> Server<O>
where
    Message<O>: Fragment,
{
    /// Constructs a new ChainSync `Server` instance.
    ///
    /// # Arguments
    ///
    /// * `channel` - An instance of `multiplexer::AgentChannel` to be used for
    ///   communication.
    pub fn new(channel: multiplexer::AgentChannel) -> Self {
        Self(
            State::Idle,
            multiplexer::ChannelBuffer::new(channel),
            PhantomData {},
        )
    }

    /// Returns the current state of the server.
    pub fn state(&self) -> &State {
        &self.0
    }

    /// Checks if the server state is done.
    pub fn is_done(&self) -> bool {
        self.0 == State::Done
    }

    /// Checks if the server has agency.
    pub fn has_agency(&self) -> bool {
        match self.state() {
            State::Idle => false,
            State::CanAwait => true,
            State::MustReply => true,
            State::Intersect => true,
            State::Done => false,
        }
    }

    fn assert_agency_is_ours(&self) -> Result<(), ServerError> {
        if !self.has_agency() {
            Err(ServerError::AgencyIsTheirs)
        } else {
            Ok(())
        }
    }

    fn assert_agency_is_theirs(&self) -> Result<(), ServerError> {
        if self.has_agency() {
            Err(ServerError::AgencyIsOurs)
        } else {
            Ok(())
        }
    }

    fn assert_outbound_state(&self, msg: &Message<O>) -> Result<(), ServerError> {
        match (&self.0, msg) {
            (State::CanAwait, Message::RollForward(_, _)) => Ok(()),
            (State::CanAwait, Message::RollBackward(_, _)) => Ok(()),
            (State::CanAwait, Message::AwaitReply) => Ok(()),
            (State::MustReply, Message::RollForward(_, _)) => Ok(()),
            (State::MustReply, Message::RollBackward(_, _)) => Ok(()),
            (State::Intersect, Message::IntersectFound(_, _)) => Ok(()),
            (State::Intersect, Message::IntersectNotFound(_)) => Ok(()),
            _ => Err(ServerError::InvalidOutbound),
        }
    }

    fn assert_inbound_state(&self, msg: &Message<O>) -> Result<(), ServerError> {
        match (&self.0, msg) {
            (State::Idle, Message::RequestNext) => Ok(()),
            (State::Idle, Message::FindIntersect(_)) => Ok(()),
            (State::Idle, Message::Done) => Ok(()),
            _ => Err(ServerError::InvalidInbound),
        }
    }

    /// Sends a message to the client
    ///
    /// # Arguments
    ///
    /// * `msg` - A reference to the `Message` to be sent.
    ///
    /// # Errors
    ///
    /// Returns an error if the agency is not ours or if the outbound state is
    /// invalid.
    pub async fn send_message(&mut self, msg: &Message<O>) -> Result<(), ServerError> {
        self.assert_agency_is_ours()?;
        self.assert_outbound_state(msg)?;

        self.1
            .send_msg_chunks(msg)
            .await
            .map_err(ServerError::Plexer)?;

        Ok(())
    }

    /// Receives the next message from the client.
    ///
    /// # Errors
    ///
    /// Returns an error if the agency is not theirs or if the inbound state is
    /// invalid.
    async fn recv_message(&mut self) -> Result<Message<O>, ServerError> {
        self.assert_agency_is_theirs()?;

        let msg = self.1.recv_full_msg().await.map_err(ServerError::Plexer)?;

        self.assert_inbound_state(&msg)?;

        Ok(msg)
    }

    /// Receive a message from the client when the protocol state is Idle.
    ///
    /// # Errors
    ///
    /// Returns an error if the agency is not theirs or if the inbound message
    /// is invalid for Idle protocol state.
    pub async fn recv_while_idle(&mut self) -> Result<Option<ClientRequest>, ServerError> {
        match self.recv_message().await? {
            Message::FindIntersect(points) => {
                self.0 = State::Intersect;
                Ok(Some(ClientRequest::Intersect(points)))
            }
            Message::RequestNext => {
                self.0 = State::CanAwait;
                Ok(Some(ClientRequest::RequestNext))
            }
            Message::Done => {
                self.0 = State::Done;

                Ok(None)
            }
            _ => Err(ServerError::InvalidInbound),
        }
    }

    /// Sends an IntersectNotFound message to the client.
    ///
    /// # Arguments
    ///
    /// * `tip` - the most recent point of the server's chain.
    ///
    /// # Errors
    ///
    /// Returns an error if the message cannot be sent or if it's not valid for
    /// the current state of the server.
    pub async fn send_intersect_not_found(&mut self, tip: Tip) -> Result<(), ServerError> {
        debug!("send intersect not found");

        let msg = Message::IntersectNotFound(tip);
        self.send_message(&msg).await?;
        self.0 = State::Idle;

        Ok(())
    }

    /// Sends an IntersectFound message to the client.
    ///
    /// # Arguments
    ///
    /// * `point` - the first point in the client's provided list of intersect
    ///   points that was found in the servers's current chain.
    /// * `tip` - the most recent point of the server's chain.
    ///
    /// # Errors
    ///
    /// Returns an error if the message cannot be sent or if it's not valid for
    /// the current state of the server.
    pub async fn send_intersect_found(
        &mut self,
        point: Point,
        tip: Tip,
    ) -> Result<(), ServerError> {
        debug!("send intersect found ({point:?}");

        let msg = Message::IntersectFound(point, tip);
        self.send_message(&msg).await?;
        self.0 = State::Idle;

        Ok(())
    }

    /// Sends a RollForward message to the client.
    ///
    /// # Arguments
    ///
    /// * `content` - the data to send to the client: for example block headers
    ///   for N2N or full blocks for N2C.
    /// * `tip` - the most recent point of the server's chain.
    ///
    /// # Errors
    ///
    /// Returns an error if the message cannot be sent or if it's not valid for
    /// the current state of the server.
    pub async fn send_roll_forward(&mut self, content: O, tip: Tip) -> Result<(), ServerError> {
        debug!("send roll forward");

        let msg = Message::RollForward(content, tip);
        self.send_message(&msg).await?;
        self.0 = State::Idle;

        Ok(())
    }

    /// Sends a RollBackward message to the client.
    ///
    /// # Arguments
    ///
    /// * `point` - point at which the client should rollback their chain to.
    /// * `tip` - the most recent point of the server's chain.
    ///
    /// # Errors
    ///
    /// Returns an error if the message cannot be sent or if it's not valid for
    /// the current state of the server.
    pub async fn send_roll_backward(&mut self, point: Point, tip: Tip) -> Result<(), ServerError> {
        debug!("send roll backward {point:?}");

        let msg = Message::RollBackward(point, tip);
        self.send_message(&msg).await?;
        self.0 = State::Idle;

        Ok(())
    }

    /// Sends an AwaitReply message to the client.
    ///
    /// # Arguments
    ///
    /// * `point` - point at which the client should rollback their chain to.
    /// * `tip` - the most recent point of the server's chain.
    ///
    /// # Errors
    ///
    /// Returns an error if the message cannot be sent or if it's not valid for
    /// the current state of the server.
    pub async fn send_await_reply(&mut self) -> Result<(), ServerError> {
        debug!("send await reply");

        let msg = Message::AwaitReply;
        self.send_message(&msg).await?;
        self.0 = State::MustReply;

        Ok(())
    }
}

pub type N2NServer = Server<HeaderContent>;

pub type N2CServer = Server<BlockContent>;
