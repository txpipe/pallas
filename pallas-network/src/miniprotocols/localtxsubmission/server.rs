use std::marker::PhantomData;

use pallas_codec::Fragment;

use crate::miniprotocols::localtxsubmission::{EraTx, Message, RejectReason, State};
use crate::multiplexer;

use super::{Error, Response};

/// Cardano specific instantiation of LocalTxSubmission server.
pub type Server = GenericServer<EraTx, RejectReason>;

/// A generic Ouroboros server for submitting a generic transaction
/// from a client, which possibly results in a generic rejection.
pub struct GenericServer<Tx, Reject> {
    state: State,
    muxer: multiplexer::ChannelBuffer,
    pd_tx: PhantomData<Tx>,
    pd_reject: PhantomData<Reject>,
}

impl<Tx, Reject> GenericServer<Tx, Reject>
where
    Message<Tx, Reject>: Fragment,
{
    /// Constructs a new LocalTxSubmission `Server` instance.
    ///
    /// # Arguments
    /// * `channel` - An instance of `multiplexer::AgentChannel` to be used for
    ///   communication.
    pub fn new(channel: multiplexer::AgentChannel) -> Self {
        Self {
            state: State::Idle,
            muxer: multiplexer::ChannelBuffer::new(channel),
            pd_tx: Default::default(),
            pd_reject: Default::default(),
        }
    }

    /// Returns the current state of the server.
    pub fn state(&self) -> &State {
        &self.state
    }

    /// Checks if the server has agency.
    fn has_agency(&self) -> bool {
        match self.state() {
            State::Idle => false,
            State::Busy | State::Done => true,
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

    /// As a server in a specific state, am I allowed to send this message?
    fn assert_outbound_state(&self, msg: &Message<Tx, Reject>) -> Result<(), Error> {
        match (&self.state, msg) {
            (State::Busy, Message::AcceptTx | Message::RejectTx(_)) => Ok(()),
            _ => Err(Error::InvalidInbound),
        }
    }

    /// As a server in a specific state, am I allowed to receive this message?
    fn assert_inbound_state(&self, msg: &Message<Tx, Reject>) -> Result<(), Error> {
        match (&self.state, msg) {
            (State::Idle, Message::SubmitTx(_) | Message::Done) => Ok(()),
            _ => Err(Error::InvalidOutbound),
        }
    }

    /// Sends a message to the server
    ///
    /// # Arguments
    ///
    /// * `msg` - A reference to the `Message` to be sent.
    ///
    /// # Errors
    /// Returns an error if the agency is not ours or if the outbound state is
    /// invalid.
    async fn send_message(&mut self, msg: &Message<Tx, Reject>) -> Result<(), Error> {
        self.assert_agency_is_ours()?;
        self.assert_outbound_state(msg)?;

        self.muxer
            .send_msg_chunks(msg)
            .await
            .map_err(Error::ChannelError)?;

        Ok(())
    }

    /// Receives the next message from the server.
    ///
    /// # Errors
    /// Returns an error if the agency is not theirs or if the inbound state is
    /// invalid.
    async fn recv_message(&mut self) -> Result<Message<Tx, Reject>, Error> {
        self.assert_agency_is_theirs()?;

        let msg = self
            .muxer
            .recv_full_msg()
            .await
            .map_err(Error::ChannelError)?;

        self.assert_inbound_state(&msg)?;

        Ok(msg)
    }

    /// Sends SubmitTx response to the client.
    pub async fn send_submit_tx_response(
        &mut self,
        response: Response<Reject>,
    ) -> Result<(), Error> {
        match response {
            Response::Accepted => {
                let msg = Message::AcceptTx;
                self.send_message(&msg).await?;
                self.state = State::Idle;

                Ok(())
            }
            Response::Rejected(reject) => {
                let msg = Message::RejectTx::<Tx, Reject>(reject);
                self.send_message(&msg).await?;
                self.state = State::Idle;

                Ok(())
            }
        }
    }

    /// Receives next request from the client.
    ///
    /// # Errors
    /// Returns an error if the inbound message is invalid.
    pub async fn recv_next_request(&mut self) -> Result<Request<Tx>, Error> {
        match self.recv_message().await? {
            Message::SubmitTx(tx) => {
                self.state = State::Busy;

                Ok(Request::Submit(tx))
            }
            Message::Done => {
                self.state = State::Done;

                Ok(Request::Done)
            }
            _ => Err(Error::InvalidInbound),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Request<Tx> {
    Submit(Tx),
    Done,
}
