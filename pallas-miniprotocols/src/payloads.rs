use pallas_codec::{minicbor, Fragment};
use pallas_multiplexer::Payload;
use std::sync::mpsc::Receiver;

pub type Error = Box<dyn std::error::Error>;

enum Decoding<M> {
    Done(M, usize),
    NotEnoughData,
    UnexpectedError(Error),
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

/// Reads from the receiver until a complete message is found
pub fn read_until_full_msg<M>(
    buffer: &mut Vec<u8>,
    receiver: &mut Receiver<Payload>,
) -> Result<M, Error>
where
    M: Fragment,
{
    // do an eager reading if buffer is empty, no point in going through the error
    // handling
    if buffer.is_empty() {
        let chunk = receiver.recv()?;
        buffer.extend(chunk);
    }

    let decoding = try_decode_message::<M>(buffer);

    match decoding {
        Decoding::Done(msg, pos) => {
            buffer.drain(0..pos);
            Ok(msg)
        }
        Decoding::UnexpectedError(err) => Err(err),
        Decoding::NotEnoughData => {
            let chunk = receiver.recv()?;
            buffer.extend(chunk);

            read_until_full_msg::<M>(buffer, receiver)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::read_until_full_msg;
    use pallas_codec::minicbor;
    use std::sync::mpsc::channel;

    #[test]
    fn multiple_messages_in_same_payload() {
        let mut input = Vec::new();
        let in_part1 = (1u8, 2u8, 3u8);
        let in_part2 = (6u8, 5u8, 4u8);

        minicbor::encode(in_part1, &mut input).unwrap();
        minicbor::encode(in_part2, &mut input).unwrap();

        let (tx, mut rx) = channel();
        tx.send(input).unwrap();

        let mut output = Vec::new();
        let out_part1 = read_until_full_msg::<(u8, u8, u8)>(&mut output, &mut rx).unwrap();
        let out_part2 = read_until_full_msg::<(u8, u8, u8)>(&mut output, &mut rx).unwrap();

        assert_eq!(in_part1, out_part1);
        assert_eq!(in_part2, out_part2);
    }

    #[test]
    fn fragmented_message_in_multiple_payload() {
        let mut input = Vec::new();
        let msg = (11u8, 12u8, 13u8, 14u8, 15u8, 16u8, 17u8);
        minicbor::encode(msg, &mut input).unwrap();

        let (tx, mut rx) = channel();

        while !input.is_empty() {
            let chunk = Vec::from(input.drain(0..2).as_slice());
            tx.send(chunk).unwrap();
        }

        let mut output = Vec::new();
        let out_msg =
            read_until_full_msg::<(u8, u8, u8, u8, u8, u8, u8)>(&mut output, &mut rx).unwrap();

        assert_eq!(msg, out_msg);
    }
}
