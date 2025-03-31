use pallas_codec::Fragment;
use std::fmt::Debug;
use tracing::{debug, warn};

use super::{DoneState, Message, RefuseReason, State, VersionNumber, VersionTable};
use crate::{miniprotocols::Error, multiplexer};

pub struct Server<D>(State<D>)
where
    D: Debug + Clone;

impl<D> Default for Server<D>
where
    D: Debug + Clone,
    Message<D>: Fragment,
{
    fn default() -> Self {
        Self(State::Propose)
    }
}

impl<D> crate::miniprotocols::Agent for Server<D>
where
    D: Debug + Clone,
    Message<D>: Fragment,
{
    type State = State<D>;
    type Message = Message<D>;

    fn new(init: Self::State) -> Self {
        Self(init)
    }

    fn is_done(&self) -> bool {
        matches!(self.state(), State::Done(..))
    }

    fn has_agency(&self) -> bool {
        matches!(self.state(), State::Confirm(..))
    }

    fn state(&self) -> &Self::State {
        &self.0
    }

    fn apply(&self, msg: &Self::Message) -> Result<Self::State, Error> {
        match self.state() {
            State::Propose => match msg {
                Message::Propose(x) => Ok(State::Confirm(x.clone())),
                _ => Err(Error::InvalidInbound),
            },
            State::Confirm(..) => match msg {
                Message::Accept(x, y) => Ok(State::Done(DoneState::Accepted(*x, y.clone()))),
                Message::Refuse(x) => Ok(State::Done(DoneState::Rejected(x.clone()))),
                Message::QueryReply(x) => Ok(State::Done(DoneState::QueryReply(x.clone()))),
                _ => Err(Error::InvalidInbound),
            },
            State::Done(..) => Err(Error::InvalidInbound),
        }
    }
}

impl<D> crate::miniprotocols::PlexerAdapter<Server<D>>
where
    D: Debug + Clone + PartialEq,
    Message<D>: Fragment,
{
    pub async fn receive_proposed_versions(&mut self) -> Result<VersionTable<D>, Error> {
        self.recv().await?;

        match self.state() {
            State::Confirm(v) => Ok(v.clone()),
            _ => Err(Error::InvalidOutbound),
        }
    }

    pub async fn accept_version(
        &mut self,
        version: VersionNumber,
        extra_params: D,
    ) -> Result<(), Error> {
        let message = Message::Accept(version, extra_params.clone());

        self.send(&message).await?;

        Ok(())
    }

    pub async fn refuse(&mut self, reason: RefuseReason) -> Result<(), Error> {
        let message = Message::Refuse(reason.clone());

        self.send(&message).await?;

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
}

pub type N2NServer = Server<super::n2n::VersionData>;

pub type N2CServer = Server<super::n2c::VersionData>;
