use pallas_codec::Fragment;
use std::fmt::Debug;
use std::marker::PhantomData;
use tracing::debug;

use super::{Error, Message, RefuseReason, State, VersionNumber, VersionTable};
use crate::multiplexer;

/// Outcome of a completed handshake exchange.
#[derive(Debug)]
pub enum Confirmation<D: Debug + Clone> {
    /// Server accepted the given version and payload.
    Accepted(VersionNumber, D),
    /// Server refused the handshake for the stated reason.
    Rejected(RefuseReason),
    /// Server replied in query mode with the versions it supports.
    QueryReply(VersionTable<D>),
}

/// Handshake client agent generic over the version-data payload type.
pub struct Client<D>(State, multiplexer::ChannelBuffer, PhantomData<D>);

impl<D> Client<D>
where
    D: Debug + Clone,
    Message<D>: Fragment,
{
    /// Build a client over a freshly subscribed agent channel.
    pub fn new(channel: multiplexer::AgentChannel) -> Self {
        Self(
            State::Propose,
            multiplexer::ChannelBuffer::new(channel),
            PhantomData {},
        )
    }

    /// Current state-machine state.
    pub fn state(&self) -> &State {
        &self.0
    }

    /// True if the protocol has terminated.
    pub fn is_done(&self) -> bool {
        self.0 == State::Done
    }

    /// True if the client holds agency in the current state.
    pub fn has_agency(&self) -> bool {
        match self.state() {
            State::Propose => true,
            State::Confirm => false,
            State::Done => false,
        }
    }

    fn assert_agency_is_ours(&self) -> Result<(), Error> {
        if !self.has_agency() {
            Err(Error::AgencyIsTheirs)
        } else {
            Ok(())
        }
    }

    fn assert_agency_is_theirs(&self) -> Result<(), Error> {
        if self.has_agency() {
            Err(Error::AgencyIsOurs)
        } else {
            Ok(())
        }
    }

    fn assert_outbound_state(&self, msg: &Message<D>) -> Result<(), Error> {
        match (&self.0, msg) {
            (State::Propose, Message::Propose(_)) => Ok(()),
            _ => Err(Error::InvalidOutbound),
        }
    }

    fn assert_inbound_state(&self, msg: &Message<D>) -> Result<(), Error> {
        match (&self.0, msg) {
            (State::Confirm, Message::Accept(..)) => Ok(()),
            (State::Confirm, Message::Refuse(..)) => Ok(()),
            (State::Confirm, Message::QueryReply(..)) => Ok(()),
            _ => Err(Error::InvalidInbound),
        }
    }

    /// Low-level send.
    pub async fn send_message(&mut self, msg: &Message<D>) -> Result<(), Error> {
        self.assert_agency_is_ours()?;
        self.assert_outbound_state(msg)?;
        self.1.send_msg_chunks(msg).await.map_err(Error::Plexer)?;

        Ok(())
    }

    /// Low-level receive.
    pub async fn recv_message(&mut self) -> Result<Message<D>, Error> {
        self.assert_agency_is_theirs()?;
        let msg = self.1.recv_full_msg().await.map_err(Error::Plexer)?;
        self.assert_inbound_state(&msg)?;

        Ok(msg)
    }

    /// Send a `Propose` message with the given offered versions.
    pub async fn send_propose(&mut self, versions: VersionTable<D>) -> Result<(), Error> {
        let msg = Message::Propose(versions);
        self.send_message(&msg).await?;
        self.0 = State::Confirm;

        debug!("version proposed");

        Ok(())
    }

    /// Wait for the server's response to our proposal.
    pub async fn recv_while_confirm(&mut self) -> Result<Confirmation<D>, Error> {
        match self.recv_message().await? {
            Message::Accept(v, m) => {
                self.0 = State::Done;
                debug!("handshake accepted");

                Ok(Confirmation::Accepted(v, m))
            }
            Message::Refuse(r) => {
                self.0 = State::Done;
                debug!("handshake refused");

                Ok(Confirmation::Rejected(r))
            }
            Message::QueryReply(version_table) => {
                debug!("handshake query reply");

                Ok(Confirmation::QueryReply(version_table))
            }
            _ => Err(Error::InvalidInbound),
        }
    }

    /// Propose `versions` and wait for the server's response in one call.
    pub async fn handshake(&mut self, versions: VersionTable<D>) -> Result<Confirmation<D>, Error> {
        self.send_propose(versions).await?;
        self.recv_while_confirm().await
    }

    /// Discard the protocol wrapper and return the raw agent channel.
    pub fn unwrap(self) -> multiplexer::AgentChannel {
        self.1.unwrap()
    }
}

/// Node-to-node handshake client.
pub type N2NClient = Client<super::n2n::VersionData>;

/// Node-to-client handshake client.
pub type N2CClient = Client<super::n2c::VersionData>;
