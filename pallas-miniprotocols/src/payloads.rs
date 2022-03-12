use pallas_codec::Fragment;
use pallas_multiplexer::Payload;
use std::sync::mpsc::Receiver;

pub type Error = Box<dyn std::error::Error>;

enum Decoding<M> {
    Done(M),
    NotEnoughData,
    UnexpectedError(Error),
}

fn try_decode_message<M>(buffer: &[u8]) -> Decoding<M>
where
    M: Fragment,
{
    let maybe_msg: Result<M, _> = M::from_cbor(buffer);

    match maybe_msg {
        Ok(msg) => Decoding::Done(msg),
        Err(err) if err.is_end_of_input() => Decoding::NotEnoughData,
        Err(err) => Decoding::UnexpectedError(Box::new(err)),
    }
}

pub fn read_until_full_msg<M>(
    buffer: &mut Vec<u8>,
    receiver: &mut Receiver<Payload>,
) -> Result<M, Error>
where
    M: Fragment,
{
    let chunk = receiver.recv()?;
    buffer.extend(chunk);

    let decoding = try_decode_message::<M>(buffer);

    match decoding {
        Decoding::Done(msg) => Ok(msg),
        Decoding::UnexpectedError(err) => Err(err),
        Decoding::NotEnoughData => read_until_full_msg::<M>(buffer, receiver),
    }
}
