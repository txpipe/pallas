use std::marker::PhantomData;

use thiserror::Error;
use tracing::debug;

use pallas_codec::minicbor;
use tracing::error;
use tracing::trace;

use crate::miniprotocols::localtxsubmission::{EraTx, Message, RejectReason, State};
use crate::multiplexer;
use crate::multiplexer::AgentChannel;
use crate::multiplexer::MAX_SEGMENT_PAYLOAD_LENGTH;

use super::cardano_node_errors::TxApplyErrors;
use super::codec::DecodeCBORSplitPayload;
use super::codec::DecodingResult;

/// Cardano specific instantiation of LocalTxSubmission client.
pub type Client<'a, ErrDecoder> =
    GenericClient<'a, EraTx, DecodingResult<TxApplyErrors>, ErrDecoder>;

/// A generic Ouroboros client for submitting a generic transaction
/// to a server, which possibly results in a generic rejection.
pub struct GenericClient<'a, Tx, Reject, ErrDecoder> {
    state: State,
    muxer: LocalTxChannelBuffer<'a, Tx, Reject, ErrDecoder>,
    pd_tx: PhantomData<Tx>,
    pd_reject: PhantomData<Reject>,
    pd_err_decoder: PhantomData<ErrDecoder>,
}

impl<'a, Tx, Reject, ErrDecoder> GenericClient<'a, Tx, Reject, ErrDecoder> {
    /// Constructs a new LocalTxSubmission `Client` instance.
    ///
    /// # Arguments
    /// * `channel` - An instance of `multiplexer::AgentChannel` to be used for
    ///   communication.
    pub fn new(channel: multiplexer::AgentChannel, err_decoder: ErrDecoder) -> Self {
        Self {
            state: State::Idle,
            muxer: LocalTxChannelBuffer::new(channel, err_decoder),
            pd_tx: Default::default(),
            pd_reject: Default::default(),
            pd_err_decoder: Default::default(),
        }
    }
}

impl<'a, Tx, Reject, ErrDecoder> GenericClient<'a, Tx, Reject, ErrDecoder>
where
    DecodingResult<Message<Tx, Reject>>: minicbor::Encode<()> + minicbor::Decode<'a, ErrDecoder>,
    ErrDecoder: DecodeCBORSplitPayload<Entity = Message<Tx, Reject>>,
    Reject: minicbor::Decode<'a, ErrDecoder> + Send + Sync + 'static,
{
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
        self.state = State::Done;
        let msg = DecodingResult::Complete(Message::Done);
        self.send_message(&msg).await?;

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

    fn assert_outbound_state(
        &self,
        msg: &DecodingResult<Message<Tx, Reject>>,
    ) -> Result<(), Error> {
        match (&self.state, msg) {
            (
                State::Idle,
                DecodingResult::Complete(Message::SubmitTx(_))
                | DecodingResult::Complete(Message::Done),
            ) => Ok(()),
            _ => Err(Error::InvalidOutbound),
        }
    }

    fn assert_inbound_state(&self, msg: &DecodingResult<Message<Tx, Reject>>) -> Result<(), Error> {
        match (&self.state, msg) {
            (
                State::Busy,
                DecodingResult::Complete(Message::AcceptTx)
                | DecodingResult::Complete(Message::RejectTx(_)),
            ) => Ok(()),
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
    async fn send_message(
        &mut self,
        msg: &DecodingResult<Message<Tx, Reject>>,
    ) -> Result<(), Error> {
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
    async fn recv_message(&mut self) -> Result<DecodingResult<Message<Tx, Reject>>, Error> {
        self.assert_agency_is_theirs()?;

        let msg = {
            self.muxer
                .recv_full_msg()
                .await
                .map(DecodingResult::Complete)?
        };
        self.assert_inbound_state(&msg)?;
        match (&self.state, &msg) {
            (
                State::Busy,
                DecodingResult::Complete(Message::AcceptTx)
                | DecodingResult::Complete(Message::RejectTx(_)),
            ) => Ok(msg),
            _ => Err(Error::InvalidInbound),
        }
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
        self.state = State::Busy;
        let msg = DecodingResult::Complete(Message::SubmitTx(tx));
        self.send_message(&msg).await?;

        debug!("sent SubmitTx");

        Ok(())
    }

    /// Receives SubmitTx response from the server.
    ///
    /// # Errors
    /// Returns an error if the inbound message is invalid.
    async fn recv_submit_tx_response(&mut self) -> Result<Response<Reject>, Error> {
        debug!("waiting for SubmitTx response");

        let mut set_idle = false;
        let response = match self.recv_message().await? {
            DecodingResult::Complete(Message::AcceptTx) => {
                set_idle = true;
                Ok(Response::Accepted)
            }
            DecodingResult::Complete(Message::RejectTx(rejection)) => {
                set_idle = true;
                Ok(Response::Rejected(rejection))
            }
            _ => Err(Error::InvalidInbound),
        };

        if set_idle {
            self.state = State::Idle;
        }

        response
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

/// A channel abstraction to hide the complexity of partial payloads
struct LocalTxChannelBuffer<'a, Tx, Reject, ErrDecoder> {
    channel: AgentChannel,
    err_decoder: ErrDecoder,
    pd_tx: PhantomData<Tx>,
    pd_reject: PhantomData<Reject>,
    pd_lifetime: PhantomData<&'a ()>,
}
impl<'a, Tx, Reject, ErrDecoder> LocalTxChannelBuffer<'a, Tx, Reject, ErrDecoder> {
    pub fn new(channel: AgentChannel, err_decoder: ErrDecoder) -> Self {
        Self {
            channel,
            err_decoder,
            pd_lifetime: Default::default(),
            pd_tx: Default::default(),
            pd_reject: Default::default(),
        }
    }
}

impl<'a, Tx, Reject, ErrDecoder> LocalTxChannelBuffer<'a, Tx, Reject, ErrDecoder>
where
    DecodingResult<Message<Tx, Reject>>: minicbor::Encode<()> + minicbor::Decode<'a, ErrDecoder>,
    ErrDecoder: DecodeCBORSplitPayload<Entity = Message<Tx, Reject>>,
    Reject: minicbor::Decode<'a, ErrDecoder> + Send + Sync + 'static,
{
    /// Enqueues a msg as a sequence payload chunks
    pub async fn send_msg_chunks<M>(&mut self, msg: &M) -> Result<(), crate::multiplexer::Error>
    where
        M: minicbor::Encode<()> + minicbor::Decode<'a, ErrDecoder>,
    {
        let mut payload = Vec::new();
        minicbor::encode(msg, &mut payload)
            .map_err(|err| crate::multiplexer::Error::Encoding(err.to_string()))?;

        let chunks = payload.chunks(MAX_SEGMENT_PAYLOAD_LENGTH);

        for chunk in chunks {
            self.channel.enqueue_chunk(Vec::from(chunk)).await?;
        }

        Ok(())
    }

    /// Reads from the channel until a complete message is found
    pub async fn recv_full_msg(&mut self) -> Result<Message<Tx, Reject>, Error> {
        loop {
            let chunk: Vec<u8> = self
                .channel
                .dequeue_chunk()
                .await
                .map_err(Error::ChannelError)?;
            let result = self.err_decoder.try_decode_with_new_bytes(&chunk);

            match result {
                Ok(decoding_result) => match decoding_result {
                    DecodingResult::Complete(c) => {
                        return Ok(c);
                    }
                    DecodingResult::Incomplete(_) => (),
                },
                Err(_e) => {
                    return Err(Error::InvalidInbound);
                }
            }
        }
    }

    pub fn unwrap(self) -> AgentChannel {
        self.channel
    }
}
