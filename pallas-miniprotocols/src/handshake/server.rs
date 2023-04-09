use std::marker::PhantomData;

use pallas_codec::Fragment;
use pallas_multiplexer::agents::{Channel, ChannelBuffer};

use super::{Error, Message, RefuseReason, State, VersionNumber, VersionTable};

pub struct Server<H, D>(State, ChannelBuffer<H>, PhantomData<D>)
where
    H: Channel;

impl<H, D> Server<H, D>
where
    H: Channel,
    D: std::fmt::Debug + Clone,
    Message<D>: Fragment,
{
    pub fn new(channel: H) -> Self {
        Self(State::Propose, ChannelBuffer::new(channel), PhantomData {})
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
        self.1
            .send_msg_chunks(msg)
            .await
            .map_err(Error::ChannelError)?;

        Ok(())
    }

    pub async fn recv_message(&mut self) -> Result<Message<D>, Error> {
        self.assert_agency_is_theirs()?;
        let msg = self.1.recv_full_msg().await.map_err(Error::ChannelError)?;
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

    pub fn unwrap(self) -> H {
        self.1.unwrap()
    }
}

pub type N2NServer<H> = Server<H, super::n2n::VersionData>;

pub type N2CServer<H> = Server<H, super::n2c::VersionData>;
