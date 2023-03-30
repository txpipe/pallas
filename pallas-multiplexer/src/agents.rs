//! Interface to interact with the multiplexer as an agent

use crate::Payload;
use pallas_codec::{minicbor, Fragment};
use thiserror::Error;

#[derive(Debug, Clone, Error)]
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
pub struct ChannelBuffer<C: Channel> {
    channel: C,
    temp: Vec<u8>,
}

impl<C: Channel> ChannelBuffer<C> {
    pub fn new(channel: C) -> Self {
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
        minicbor::encode(msg, &mut payload)
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

    pub fn unwrap(self) -> C {
        self.channel
    }
}

impl<C: Channel> From<C> for ChannelBuffer<C> {
    fn from(channel: C) -> Self {
        ChannelBuffer::new(channel)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use super::*;

    impl Channel for VecDeque<Payload> {
        fn enqueue_chunk(&mut self, chunk: Payload) -> Result<(), ChannelError> {
            self.push_back(chunk);
            Ok(())
        }

        fn dequeue_chunk(&mut self) -> Result<Payload, ChannelError> {
            let chunk = self.pop_front().ok_or(ChannelError::NotConnected(None))?;
            Ok(chunk)
        }
    }

    #[test]
    fn multiple_messages_in_same_payload() {
        let mut input = Vec::new();
        let in_part1 = (1u8, 2u8, 3u8);
        let in_part2 = (6u8, 5u8, 4u8);

        minicbor::encode(in_part1, &mut input).unwrap();
        minicbor::encode(in_part2, &mut input).unwrap();

        let mut channel = VecDeque::<Payload>::new();
        channel.push_back(input);

        let mut buf = ChannelBuffer::new(channel);

        let out_part1 = buf.recv_full_msg::<(u8, u8, u8)>().unwrap();
        let out_part2 = buf.recv_full_msg::<(u8, u8, u8)>().unwrap();

        assert_eq!(in_part1, out_part1);
        assert_eq!(in_part2, out_part2);
    }

    #[test]
    fn fragmented_message_in_multiple_payloads() {
        let mut input = Vec::new();
        let msg = (11u8, 12u8, 13u8, 14u8, 15u8, 16u8, 17u8);
        minicbor::encode(msg, &mut input).unwrap();

        let mut channel = VecDeque::<Payload>::new();

        while !input.is_empty() {
            let chunk = Vec::from(input.drain(0..2).as_slice());
            channel.push_back(chunk);
        }

        let mut buf = ChannelBuffer::new(channel);

        let out_msg = buf.recv_full_msg::<(u8, u8, u8, u8, u8, u8, u8)>().unwrap();

        assert_eq!(msg, out_msg);
    }
}
