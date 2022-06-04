//! Interface to interact with the multiplexer as an agent

use crate::Payload;
use pallas_codec::{minicbor, Fragment};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ChannelError {
    #[error("channel is not connected, failed to send payload")]
    NotConnected(Option<Payload>),

    #[error("failure encoding message into CBOR")]
    Encoding(String),

    #[error("failure decoding message from CBOR")]
    Decoding(String),
}

/// A raw link to the ingress / egress of the multiplexer
pub trait Channel {
    fn enqueue_chunk(&mut self, chunk: Payload) -> Result<(), ChannelError>;
    fn dequeue_chunk(&mut self) -> Result<Payload, ChannelError>;
}

/// Protocol value that defines max segment length
pub const MAX_SEGMENT_PAYLOAD_LENGTH: usize = 65535;

enum Decoding<M> {
    Done(M, usize),
    NotEnoughData,
    UnexpectedError(Box<dyn std::error::Error>),
}

fn try_decode_message<M>(buffer: &[u8]) -> Decoding<M>
where
    M: Fragment,
{
    let mut decoder = minicbor::Decoder::new(buffer);
    let maybe_msg = decoder.decode();

    match maybe_msg {
        Ok(msg) => Decoding::Done(msg, decoder.position()),
        Err(err) if err.is_end_of_input() => Decoding::NotEnoughData,
        Err(err) => Decoding::UnexpectedError(Box::new(err)),
    }
}

/// A channel abstraction to hide the complexity of partial payloads
pub struct ChannelBuffer<'c, C: Channel> {
    channel: &'c mut C,
    temp: Vec<u8>,
}

impl<'c, C: Channel> ChannelBuffer<'c, C> {
    pub fn new(channel: &'c mut C) -> Self {
        Self {
            channel,
            temp: Vec::new(),
        }
    }

    /// Enqueues a msg as a sequence payload chunks
    pub fn send_msg_chunks<M>(&mut self, msg: &M) -> Result<(), ChannelError>
    where
        M: Fragment,
    {
        let mut payload = Vec::new();
        minicbor::encode(&msg, &mut payload)
            .map_err(|err| ChannelError::Encoding(err.to_string()))?;

        let chunks = payload.chunks(MAX_SEGMENT_PAYLOAD_LENGTH);

        for chunk in chunks {
            self.channel.enqueue_chunk(Vec::from(chunk))?;
        }

        Ok(())
    }

    /// Reads from the channel until a complete message is found
    pub fn recv_full_msg<M>(&mut self) -> Result<M, ChannelError>
    where
        M: Fragment,
    {
        // do an eager reading if buffer is empty, no point in going through the error
        // handling
        if self.temp.is_empty() {
            let chunk = self.channel.dequeue_chunk()?;
            self.temp.extend(chunk);
        }

        let decoding = try_decode_message::<M>(&self.temp);

        match decoding {
            Decoding::Done(msg, pos) => {
                self.temp.drain(0..pos);
                Ok(msg)
            }
            Decoding::UnexpectedError(err) => Err(ChannelError::Decoding(err.to_string())),
            Decoding::NotEnoughData => {
                let chunk = self.channel.dequeue_chunk()?;
                self.temp.extend(chunk);

                self.recv_full_msg()
            }
        }
    }
}
