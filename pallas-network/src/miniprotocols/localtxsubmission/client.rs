use std::marker::PhantomData;

use thiserror::Error;
use tracing::debug;

use pallas_codec::Fragment;

use crate::miniprotocols::localtxsubmission::{EraTx, Message, State};
use crate::multiplexer;

use super::TxValidationError;

/// Cardano specific instantiation of LocalTxSubmission client.
pub type Client = GenericClient<EraTx, TxValidationError>;

/// A generic Ouroboros client for submitting a generic transaction
/// to a server, which possibly results in a generic rejection.
pub struct GenericClient<Tx, Reject> {
    state: State,
    muxer: multiplexer::ChannelBuffer,
    pd_tx: PhantomData<Tx>,
    pd_reject: PhantomData<Reject>,
}

impl<Tx, Reject> GenericClient<Tx, Reject>
where
    Message<Tx, Reject>: Fragment,
{
    /// Constructs a new LocalTxSubmission `Client` instance.
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

    /// Submits the given `tx` to the server.
    ///
    /// # Arguments
    /// * `tx` - transaction to submit.
    ///
    /// # Errors
    /// Returns an error if the agency is not ours or if the outbound state is
    /// invalid.
    pub async fn submit_tx(&mut self, tx: Tx) -> Result<Response<Reject>, Error> {
        self.send_submit_tx(tx).await?;
        self.recv_submit_tx_response().await
    }

    /// Terminates the protocol gracefully.
    ///
    /// # Errors
    /// Returns an error if the agency is not ours or if the outbound state is
    /// invalid.
    pub async fn terminate_gracefully(&mut self) -> Result<(), Error> {
        let msg = Message::Done;
        self.send_message(&msg).await?;
        self.state = State::Done;

        Ok(())
    }

    /// Returns the current state of the client.
    fn state(&self) -> &State {
        &self.state
    }

    /// Checks if the client has agency.
    fn has_agency(&self) -> bool {
        match self.state() {
            State::Idle => true,
            State::Busy | State::Done => false,
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

    fn assert_outbound_state(&self, msg: &Message<Tx, Reject>) -> Result<(), Error> {
        match (&self.state, msg) {
            (State::Idle, Message::SubmitTx(_) | Message::Done) => Ok(()),
            _ => Err(Error::InvalidOutbound),
        }
    }

    fn assert_inbound_state(&self, msg: &Message<Tx, Reject>) -> Result<(), Error> {
        match (&self.state, msg) {
            (State::Busy, Message::AcceptTx | Message::RejectTx(_)) => Ok(()),
            _ => Err(Error::InvalidInbound),
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

    /// Sends SubmitTx message to the server.
    ///
    /// # Arguments
    /// * `tx` - transaction to submit.
    ///
    /// # Errors
    /// Returns an error if the agency is not ours or if the outbound state is
    /// invalid.
    async fn send_submit_tx(&mut self, tx: Tx) -> Result<(), Error> {
        let msg = Message::SubmitTx(tx);
        self.send_message(&msg).await?;
        self.state = State::Busy;

        debug!("sent SubmitTx");

        Ok(())
    }

    /// Receives SubmitTx response from the server.
    ///
    /// # Errors
    /// Returns an error if the inbound message is invalid.
    async fn recv_submit_tx_response(&mut self) -> Result<Response<Reject>, Error> {
        debug!("waiting for SubmitTx response");

        match self.recv_message().await? {
            Message::AcceptTx => {
                self.state = State::Idle;
                Ok(Response::Accepted)
            }
            Message::RejectTx(rejection) => {
                self.state = State::Idle;
                Ok(Response::Rejected(rejection))
            }
            _ => Err(Error::InvalidInbound),
        }
    }
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("attempted to receive message while agency is ours")]
    AgencyIsOurs,

    #[error("attempted to send message while agency is theirs")]
    AgencyIsTheirs,

    #[error("inbound message is not valid for current state")]
    InvalidInbound,

    #[error("outbound message is not valid for current state")]
    InvalidOutbound,

    #[error("error while sending or receiving data through the channel")]
    ChannelError(multiplexer::Error),
}

#[derive(Debug)]
pub enum Response<Reject> {
    Accepted,
    Rejected(Reject),
}
