use pallas_codec::Fragment;
use std::marker::PhantomData;
use thiserror::Error;
use tracing::debug;

use crate::miniprotocols::Point;
use crate::multiplexer;

use super::{BlockContent, HeaderContent, IntersectResponse, Message, State, Tip};

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("attempted to receive message while agency is ours")]
    AgencyIsOurs,

    #[error("attempted to send message while agency is theirs")]
    AgencyIsTheirs,

    #[error("inbound message is not valid for current state")]
    InvalidInbound,

    #[error("outbound message is not valid for current state")]
    InvalidOutbound,

    #[error("no intersection point found")]
    IntersectionNotFound,

    #[error("error while sending or receiving data through the channel")]
    Plexer(multiplexer::Error),
}

#[derive(Debug)]
pub enum NextResponse<CONTENT> {
    RollForward(CONTENT, Tip),
    RollBackward(Point, Tip),
    Await,
}

pub struct Client<O>(State, multiplexer::ChannelBuffer, PhantomData<O>)
where
    Message<O>: Fragment;

impl<O> Client<O>
where
    Message<O>: Fragment,
{
    /// Constructs a new ChainSync `Client` instance.
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

    /// Returns the current state of the client.
    pub fn state(&self) -> &State {
        &self.0
    }

    /// Checks if the client is done.
    pub fn is_done(&self) -> bool {
        self.0 == State::Done
    }

    /// Checks if the client has agency.
    pub fn has_agency(&self) -> bool {
        match self.state() {
            State::Idle => true,
            State::CanAwait => false,
            State::MustReply => false,
            State::Intersect => false,
            State::Done => false,
        }
    }

    fn assert_agency_is_ours(&self) -> Result<(), ClientError> {
        if !self.has_agency() {
            Err(ClientError::AgencyIsTheirs)
        } else {
            Ok(())
        }
    }

    fn assert_agency_is_theirs(&self) -> Result<(), ClientError> {
        if self.has_agency() {
            Err(ClientError::AgencyIsOurs)
        } else {
            Ok(())
        }
    }

    fn assert_outbound_state(&self, msg: &Message<O>) -> Result<(), ClientError> {
        match (&self.0, msg) {
            (State::Idle, Message::RequestNext) => Ok(()),
            (State::Idle, Message::FindIntersect(_)) => Ok(()),
            (State::Idle, Message::Done) => Ok(()),
            _ => Err(ClientError::InvalidOutbound),
        }
    }

    fn assert_inbound_state(&self, msg: &Message<O>) -> Result<(), ClientError> {
        match (&self.0, msg) {
            (State::CanAwait, Message::RollForward(_, _)) => Ok(()),
            (State::CanAwait, Message::RollBackward(_, _)) => Ok(()),
            (State::CanAwait, Message::AwaitReply) => Ok(()),
            (State::MustReply, Message::RollForward(_, _)) => Ok(()),
            (State::MustReply, Message::RollBackward(_, _)) => Ok(()),
            (State::Intersect, Message::IntersectFound(_, _)) => Ok(()),
            (State::Intersect, Message::IntersectNotFound(_)) => Ok(()),
            _ => Err(ClientError::InvalidInbound),
        }
    }

    /// Sends a message to the server
    ///
    /// # Arguments
    ///
    /// * `msg` - A reference to the `Message` to be sent.
    ///
    /// # Errors
    ///
    /// Returns an error if the agency is not ours or if the outbound state is
    /// invalid.
    pub async fn send_message(&mut self, msg: &Message<O>) -> Result<(), ClientError> {
        self.assert_agency_is_ours()?;
        self.assert_outbound_state(msg)?;

        self.1
            .send_msg_chunks(msg)
            .await
            .map_err(ClientError::Plexer)?;

        Ok(())
    }

    /// Receives the next message from the server.
    ///
    /// # Errors
    ///
    /// Returns an error if the agency is not theirs or if the inbound state is
    /// invalid.
    pub async fn recv_message(&mut self) -> Result<Message<O>, ClientError> {
        self.assert_agency_is_theirs()?;

        let msg = self.1.recv_full_msg().await.map_err(ClientError::Plexer)?;

        self.assert_inbound_state(&msg)?;

        Ok(msg)
    }

    /// Sends a FindIntersect message to the server.
    ///
    /// # Arguments
    ///
    /// * `points` - A vector of `Point` instances representing the points of
    ///   intersection.
    ///
    /// # Errors
    ///
    /// Returns an error if the message cannot be sent or if it's not valid for
    /// the current state of the client.
    pub async fn send_find_intersect(&mut self, points: Vec<Point>) -> Result<(), ClientError> {
        let msg = Message::FindIntersect(points);
        self.send_message(&msg).await?;
        self.0 = State::Intersect;

        debug!("send find intersect");

        Ok(())
    }

    /// Receives an IntersectResponse message from the server.
    ///
    /// # Errors
    ///
    /// Returns an error if the inbound message is invalid.
    pub async fn recv_intersect_response(&mut self) -> Result<IntersectResponse, ClientError> {
        debug!("waiting for intersect response");

        match self.recv_message().await? {
            Message::IntersectFound(point, tip) => {
                self.0 = State::Idle;
                Ok((Some(point), tip))
            }
            Message::IntersectNotFound(tip) => {
                self.0 = State::Idle;
                Ok((None, tip))
            }
            _ => Err(ClientError::InvalidInbound),
        }
    }

    /// Finds the intersection point between the client's and server's chains.
    ///
    /// # Arguments
    ///
    /// * `points` - A vector of `Point` instances representing the points of
    ///   intersection.
    ///
    /// # Errors
    ///
    /// Returns an error if the intersection point cannot be found or if there
    /// is a communication error.
    pub async fn find_intersect(
        &mut self,
        points: Vec<Point>,
    ) -> Result<IntersectResponse, ClientError> {
        self.send_find_intersect(points).await?;
        self.recv_intersect_response().await
    }

    pub async fn send_request_next(&mut self) -> Result<(), ClientError> {
        let msg = Message::RequestNext;
        self.send_message(&msg).await?;
        self.0 = State::CanAwait;

        Ok(())
    }

    /// Receives a response while the client is in the CanAwait state.
    ///
    /// # Errors
    ///
    /// Returns an error if the inbound message is invalid.
    pub async fn recv_while_can_await(&mut self) -> Result<NextResponse<O>, ClientError> {
        match self.recv_message().await? {
            Message::AwaitReply => {
                self.0 = State::MustReply;
                Ok(NextResponse::Await)
            }
            Message::RollForward(a, b) => {
                self.0 = State::Idle;
                Ok(NextResponse::RollForward(a, b))
            }
            Message::RollBackward(a, b) => {
                self.0 = State::Idle;
                Ok(NextResponse::RollBackward(a, b))
            }
            _ => Err(ClientError::InvalidInbound),
        }
    }

    /// Receives a response while the client is in the MustReply state.
    ///
    /// # Errors
    ///
    /// Returns an error if the inbound message is invalid.
    pub async fn recv_while_must_reply(&mut self) -> Result<NextResponse<O>, ClientError> {
        match self.recv_message().await? {
            Message::RollForward(a, b) => {
                self.0 = State::Idle;
                Ok(NextResponse::RollForward(a, b))
            }
            Message::RollBackward(a, b) => {
                self.0 = State::Idle;
                Ok(NextResponse::RollBackward(a, b))
            }
            _ => Err(ClientError::InvalidInbound),
        }
    }

    /// Sends a RequestNext message to the server.
    ///
    /// # Errors
    ///
    /// Returns an error if the message cannot be sent or if the state is not
    /// idle.
    pub async fn request_next(&mut self) -> Result<NextResponse<O>, ClientError> {
        debug!("requesting next block");

        self.send_request_next().await?;

        self.recv_while_can_await().await
    }

    /// Attempt to intersect the chain at its origin (genesis block)
    ///
    /// # Errors
    ///
    /// Returns an error if the intersection point cannot be found or if there
    /// is a communication error.
    pub async fn intersect_origin(&mut self) -> Result<Point, ClientError> {
        debug!("intersecting origin");

        let (point, _) = self.find_intersect(vec![Point::Origin]).await?;

        point.ok_or(ClientError::IntersectionNotFound)
    }

    /// Attempts to intersect the chain at the latest known tip
    ///
    /// # Errors
    ///
    /// Returns an error if the intersection point cannot be found or if there
    /// is a communication error.
    pub async fn intersect_tip(&mut self) -> Result<Point, ClientError> {
        let (_, Tip(point, _)) = self.find_intersect(vec![Point::Origin]).await?;

        debug!(?point, "found tip value");

        let (point, _) = self.find_intersect(vec![point]).await?;

        point.ok_or(ClientError::IntersectionNotFound)
    }

    pub async fn send_done(&mut self) -> Result<(), ClientError> {
        let msg = Message::Done;
        self.send_message(&msg).await?;
        self.0 = State::Done;

        Ok(())
    }
}

pub type N2NClient = Client<HeaderContent>;

pub type N2CClient = Client<BlockContent>;
