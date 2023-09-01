use std::marker::PhantomData;

use pallas_codec::Fragment;
use tracing::{debug, warn};

use super::{Error, Message, RefuseReason, State, VersionNumber, VersionTable};
use crate::multiplexer;

pub struct Server<D>(State, multiplexer::ChannelBuffer, PhantomData<D>);

impl<D> Server<D>
where
    D: std::fmt::Debug + Clone + std::cmp::PartialEq,
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

    /// Perform a handshake with the client
    ///
    /// Performs a full handshake with the client, where `versions` are the
    /// acceptable versions supported by the server.
    pub async fn handshake(
        &mut self,
        versions: VersionTable<D>,
    ) -> Result<Option<(VersionNumber, D)>, Error> {
        // receive proposed versions
        let client_versions = self
            .receive_proposed_versions()
            .await?
            .values
            .into_iter()
            .collect::<Vec<(u64, D)>>();

        // find highest intersect with our version table (TODO: improve)
        let mut versions = versions.values.into_iter().collect::<Vec<(u64, D)>>();

        versions.sort_by(|a, b| b.0.cmp(&a.0));

        for (ver_num, ver_data) in versions.clone() {
            for (client_ver_num, client_ver_data) in client_versions.clone() {
                if ver_num == client_ver_num {
                    if ver_data == client_ver_data {
                        // found a version number and extra data match
                        debug!("accepting hs with ({}, {:?})", ver_num, ver_data);

                        self.accept_version(ver_num, ver_data.clone()).await?;

                        return Ok(Some((ver_num, ver_data)));
                    } else {
                        warn!(
                            "rejecting hs as params not acceptable - server: {:?}, client: {:?}",
                            ver_data, client_ver_data
                        );

                        // found version number match but extra data not acceptable
                        self.refuse(RefuseReason::Refused(
                            ver_num,
                            "Proposed extra params don't match".into(),
                        ))
                        .await?;

                        return Ok(None);
                    }
                }
            }
        }

        warn!(
            "rejecting hs as no version intersect found - server: {:?}, client: {:?}",
            versions, client_versions
        );

        // failed to find a version number intersection
        self.refuse(RefuseReason::VersionMismatch(
            versions.into_iter().map(|(num, _)| num).collect(),
        ))
        .await?;

        Ok(None)
    }

    pub fn unwrap(self) -> multiplexer::AgentChannel {
        self.1.unwrap()
    }
}

pub type N2NServer = Server<super::n2n::VersionData>;

pub type N2CServer = Server<super::n2c::VersionData>;
