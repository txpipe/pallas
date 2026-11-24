use pallas_codec::{Fragment, minicbor};
use std::marker::PhantomData;
use thiserror::Error;
use tracing::debug;

use crate::miniprotocols::Point;
use crate::multiplexer;

use super::{BlockContent, HeaderContent, IntersectResponse, Message, State, Tip};

/// Errors produced by the chain-sync client agent.
#[derive(Error, Debug)]
pub enum ClientError {
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

    /// None of the points offered by the client are on the server's chain.
    #[error("no intersection point found")]
    IntersectionNotFound,

    /// Underlying multiplexer error.
    #[error("error while sending or receiving data through the channel")]
    Plexer(multiplexer::Error),
}

/// Outcome of a single `request_next` step.
#[derive(Debug)]
pub enum NextResponse<CONTENT> {
    /// Chain advances with new content.
    RollForward(CONTENT, Tip),
    /// Chain rolls back to the given point.
    RollBackward(Point, Tip),
    /// No update is available yet; the server is awaiting one.
    Await,
}

struct ClientMeta<O> {
    in_flight: usize,
    _content: PhantomData<O>,
}

/// Chain-sync client agent generic over the content type (`HeaderContent` for
/// node-to-node, `BlockContent` for node-to-client).
///
/// With node-to-node content, inbound messages larger than
/// [`MESSAGE_SIZE_LIMIT`](super::MESSAGE_SIZE_LIMIT) fail with [`multiplexer::Error::Decoding`], unless
/// [`Self::disable_size_limits`] is called. Other content types are received
/// without limits.
pub struct Client<O>(State, multiplexer::ChannelBuffer, ClientMeta<O>)
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
        let mut buffer = multiplexer::ChannelBuffer::new(channel);
        buffer.set_size_limits(super::is_node_to_node::<O>());
        Self(
            State::Idle,
            buffer,
            ClientMeta {
                in_flight: 0,
                _content: PhantomData,
            },
        )
    }

    /// Receive inbound messages of any size, as releases before 1.5 did.
    ///
    /// By default a message larger than the size limit of its protocol state
    /// fails with [`multiplexer::Error::Decoding`].
    pub fn disable_size_limits(&mut self) {
        self.1.set_size_limits(false);
    }

    fn size_limit(&self) -> usize {
        super::MESSAGE_SIZE_LIMIT
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

        let msg = self
            .1
            .recv_msg(Some(self.size_limit()))
            .await
            .map_err(ClientError::Plexer)?;

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

    /// Returns the number of unanswered `RequestNext` messages currently in flight.
    ///
    /// This is `0` when the client is in [`State::Idle`], [`State::Intersect`],
    /// or [`State::Done`], and `>= 1` while awaiting responses in
    /// [`State::CanAwait`] or [`State::MustReply`]. Receiving
    /// [`NextResponse::Await`] does not decrement this count because the
    /// underlying request is still awaiting a `RollForward` or `RollBackward`.
    pub fn in_flight(&self) -> usize {
        self.2.in_flight
    }

    /// Returns `true` when another pipelined `RequestNext` may be sent.
    ///
    /// Pipelining is permitted in [`State::Idle`] and [`State::CanAwait`], and
    /// disallowed in [`State::MustReply`] (server already signaled no ready
    /// block), [`State::Intersect`], and [`State::Done`].
    pub fn can_pipeline_request_next(&self) -> bool {
        matches!(self.0, State::Idle | State::CanAwait)
    }

    fn complete_one_in_flight(&mut self) {
        self.2.in_flight = self.2.in_flight.saturating_sub(1);
        self.0 = if self.2.in_flight > 0 {
            State::CanAwait
        } else {
            State::Idle
        };
    }

    /// Send a `RequestNext` message and transition to `CanAwait`.
    pub async fn send_request_next(&mut self) -> Result<(), ClientError> {
        let msg = Message::RequestNext;
        self.send_message(&msg).await?;
        self.0 = State::CanAwait;
        self.2.in_flight = 1;

        Ok(())
    }

    /// Send a `RequestNext` message, allowing pipelining while already in
    /// [`State::CanAwait`].
    ///
    /// Unlike [`Self::send_request_next`], which requires [`State::Idle`], this
    /// method may be called repeatedly while catching up so multiple
    /// `RequestNext` messages stay in flight concurrently.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError::AgencyIsTheirs`] if the client is in
    /// [`State::MustReply`], [`State::Intersect`], or [`State::Done`].
    pub async fn send_request_next_pipelined(&mut self) -> Result<(), ClientError> {
        self.send_request_next_n(1).await
    }

    /// Send `count` pipelined `RequestNext` messages coalesced into as few
    /// multiplexer segments as possible.
    ///
    /// Sending `0` is a no-op and returns `Ok(())`. For `count > 0`, the client
    /// must be in [`State::Idle`] or [`State::CanAwait`]. Keeping the total
    /// in-flight count at or below
    /// [`DEFAULT_PIPELINE_WINDOW`](super::DEFAULT_PIPELINE_WINDOW) avoids
    /// filling the multiplexer's 100-slot queues and starving sibling
    /// mini-protocols such as keep-alive.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError::AgencyIsTheirs`] if the client is in
    /// [`State::MustReply`], [`State::Intersect`], or [`State::Done`], or
    /// [`ClientError::InvalidOutbound`] if `in_flight + count` overflows.
    pub async fn send_request_next_n(&mut self, count: usize) -> Result<(), ClientError> {
        if count == 0 {
            return Ok(());
        }

        if !self.can_pipeline_request_next() {
            return Err(ClientError::AgencyIsTheirs);
        }

        let new_in_flight = self
            .2
            .in_flight
            .checked_add(count)
            .ok_or(ClientError::InvalidOutbound)?;

        let mut single = Vec::with_capacity(2);
        minicbor::encode(Message::<O>::RequestNext, &mut single)
            .map_err(|err| ClientError::Plexer(multiplexer::Error::Encoding(err.to_string())))?;

        if count == 1 {
            self.1
                .send_raw_chunks(&single)
                .await
                .map_err(ClientError::Plexer)?;
        } else {
            let total_len = single
                .len()
                .checked_mul(count)
                .ok_or(ClientError::InvalidOutbound)?;
            let mut batch = Vec::with_capacity(total_len);
            for _ in 0..count {
                batch.extend_from_slice(&single);
            }
            self.1
                .send_raw_chunks(&batch)
                .await
                .map_err(ClientError::Plexer)?;
        }

        self.0 = State::CanAwait;
        self.2.in_flight = new_in_flight;

        Ok(())
    }

    /// Receives the next response while the client has an in-flight
    /// `RequestNext` (in either [`State::CanAwait`] or [`State::MustReply`]).
    ///
    /// - On [`Message::AwaitReply`] (valid only in `CanAwait`), transitions to
    ///   [`State::MustReply`] without decrementing [`Self::in_flight`].
    /// - On [`Message::RollForward`] or [`Message::RollBackward`], decrements
    ///   [`Self::in_flight`] and transitions to [`State::CanAwait`] if further
    ///   pipelined requests remain in flight, or [`State::Idle`] once
    ///   `in_flight` reaches `0`.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError::AgencyIsOurs`] in [`State::Idle`], or
    /// [`ClientError::InvalidInbound`] in [`State::Intersect`] / [`State::Done`]
    /// or if the peer sends an unexpected message for the current state.
    pub async fn recv_next(&mut self) -> Result<NextResponse<O>, ClientError> {
        match self.0 {
            State::Idle => return Err(ClientError::AgencyIsOurs),
            State::Intersect | State::Done => return Err(ClientError::InvalidInbound),
            State::CanAwait | State::MustReply => {}
        }

        match self.recv_message().await? {
            Message::AwaitReply => {
                self.0 = State::MustReply;
                Ok(NextResponse::Await)
            }
            Message::RollForward(a, b) => {
                self.complete_one_in_flight();
                Ok(NextResponse::RollForward(a, b))
            }
            Message::RollBackward(a, b) => {
                self.complete_one_in_flight();
                Ok(NextResponse::RollBackward(a, b))
            }
            _ => Err(ClientError::InvalidInbound),
        }
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
                self.complete_one_in_flight();
                Ok(NextResponse::RollForward(a, b))
            }
            Message::RollBackward(a, b) => {
                self.complete_one_in_flight();
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
                self.complete_one_in_flight();
                Ok(NextResponse::RollForward(a, b))
            }
            Message::RollBackward(a, b) => {
                self.complete_one_in_flight();
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

    /// Either requests the next block, or waits for one to become available.
    ///
    /// # Errors
    ///
    /// Returns an error if the message cannot be sent, or if the inbound
    /// message is invalid
    pub async fn request_or_await_next(&mut self) -> Result<NextResponse<O>, ClientError> {
        if self.has_agency() {
            self.request_next().await
        } else {
            self.recv_next().await
        }
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

    /// Send a `Done` message and terminate the protocol.
    pub async fn send_done(&mut self) -> Result<(), ClientError> {
        let msg = Message::Done;
        self.send_message(&msg).await?;
        self.0 = State::Done;

        Ok(())
    }
}

/// Node-to-node chain-sync client (streams `HeaderContent`).
pub type N2NClient = Client<HeaderContent>;

/// Node-to-client chain-sync client (streams whole `BlockContent`).
pub type N2CClient = Client<BlockContent>;
