use std::marker::PhantomData;

use pallas_codec::Fragment;

use super::{Error, Message, RefuseReason, State, VersionNumber, VersionTable};
use crate::multiplexer;

pub struct Server<D>(State, multiplexer::ChannelBuffer, PhantomData<D>);

impl<D> Server<D>
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
        matches!(self.state(), State::Confirm)
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
            (State::Confirm, Message::Accept(..)) => Ok(()),
            (State::Confirm, Message::Refuse(_)) => Ok(()),
            _ => Err(Error::InvalidOutbound),
        }
    }

    fn assert_inbound_state(&self, msg: &Message<D>) -> Result<(), Error> {
        match (&self.0, msg) {
            (State::Propose, Message::Propose(..)) => Ok(()),
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

    pub async fn receive_proposed_versions(&mut self) -> Result<VersionTable<D>, Error> {
        match self.recv_message().await? {
            Message::Propose(v) => {
                self.0 = State::Confirm;
                Ok(v)
            }
            _ => Err(Error::InvalidOutbound),
        }
    }

    pub async fn accept_version(
        &mut self,
        version: VersionNumber,
        extra_params: D,
    ) -> Result<(), Error> {
        let message = Message::Accept(version, extra_params);
        self.send_message(&message).await?;
        self.0 = State::Done;

        Ok(())
    }

    pub async fn refuse(&mut self, reason: RefuseReason) -> Result<(), Error> {
        let message = Message::Refuse(reason);
        self.send_message(&message).await?;
        self.0 = State::Done;

        Ok(())
    }

    pub fn unwrap(self) -> multiplexer::AgentChannel {
        self.1.unwrap()
    }
}

pub type N2NServer = Server<super::n2n::VersionData>;

pub type N2CServer = Server<super::n2c::VersionData>;
