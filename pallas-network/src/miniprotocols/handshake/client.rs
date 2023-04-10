use pallas_codec::Fragment;
use std::marker::PhantomData;
use tracing::debug;

use super::{Error, Message, RefuseReason, State, VersionNumber, VersionTable};
use crate::multiplexer;

#[derive(Debug)]
pub enum Confirmation<D> {
    Accepted(VersionNumber, D),
    Rejected(RefuseReason),
}

pub struct Client<D>(State, multiplexer::ChannelBuffer, PhantomData<D>);

impl<D> Client<D>
where
    D: std::fmt::Debug + Clone,
    Message<D>: Fragment,
{
    pub fn new(channel: multiplexer::AgentChannel) -> Self {
        Self(
            State::Propose,
            multiplexer::ChannelBuffer::new(channel),
            PhantomData {},
        )
    }

    pub fn state(&self) -> &State {
        &self.0
    }

    pub fn is_done(&self) -> bool {
        self.0 == State::Done
    }

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
            _ => Err(Error::InvalidInbound),
        }
    }

    pub async fn send_message(&mut self, msg: &Message<D>) -> Result<(), Error> {
        self.assert_agency_is_ours()?;
        self.assert_outbound_state(msg)?;
        self.1.send_msg_chunks(msg).await.map_err(Error::Plexer)?;

        Ok(())
    }

    pub async fn recv_message(&mut self) -> Result<Message<D>, Error> {
        self.assert_agency_is_theirs()?;
        let msg = self.1.recv_full_msg().await.map_err(Error::Plexer)?;
        self.assert_inbound_state(&msg)?;

        Ok(msg)
    }

    pub async fn send_propose(&mut self, versions: VersionTable<D>) -> Result<(), Error> {
        let msg = Message::Propose(versions);
        self.send_message(&msg).await?;
        self.0 = State::Confirm;

        debug!("version proposed");

        Ok(())
    }

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
            _ => Err(Error::InvalidInbound),
        }
    }

    pub async fn handshake(&mut self, versions: VersionTable<D>) -> Result<Confirmation<D>, Error> {
        self.send_propose(versions).await?;
        self.recv_while_confirm().await
    }

    pub fn unwrap(self) -> multiplexer::AgentChannel {
        self.1.unwrap()
    }
}

pub type N2NClient = Client<super::n2n::VersionData>;

pub type N2CClient = Client<super::n2c::VersionData>;
