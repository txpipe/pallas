use pallas_codec::Fragment;
use std::fmt::Debug;
use tracing::debug;

use super::{DoneState, Message, State, VersionTable};
use crate::miniprotocols::Error;

pub struct Client<D>(State<D>)
where
    D: Debug + Clone;

impl<D> Default for Client<D>
where
    D: Debug + Clone,
    Message<D>: Fragment,
{
    fn default() -> Self {
        Client(State::<D>::default())
    }
}

impl<D> crate::miniprotocols::Agent for Client<D>
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
        match self.state() {
            State::Propose => true,
            State::Confirm(..) => false,
            State::Done(..) => true,
        }
    }

    fn state(&self) -> &Self::State {
        &self.0
    }

    fn apply(&self, msg: &Self::Message) -> Result<Self::State, Error> {
        match self.state() {
            State::Propose => match msg {
                Message::Propose(x) => Ok(State::Confirm(x.clone())),
                _ => Err(Error::InvalidOutbound),
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

impl<D> crate::miniprotocols::PlexerAdapter<Client<D>>
where
    D: Debug + Clone,
    Message<D>: Fragment,
{
    pub async fn send_propose(&mut self, versions: VersionTable<D>) -> Result<(), Error> {
        let msg = Message::Propose(versions);
        self.send(&msg).await?;

        debug!("version proposed");

        Ok(())
    }

    pub async fn recv_while_confirm(&mut self) -> Result<DoneState<D>, Error> {
        self.recv().await?;

        debug!("version confirmed");

        match self.state() {
            State::Done(x) => Ok(x.clone()),
            _ => Err(Error::InvalidInbound),
        }
    }

    pub async fn handshake(&mut self, versions: VersionTable<D>) -> Result<DoneState<D>, Error> {
        self.send_propose(versions).await?;
        self.recv_while_confirm().await
    }
}

pub type N2NClient = Client<super::n2n::VersionData>;

pub type N2CClient = Client<super::n2c::VersionData>;
