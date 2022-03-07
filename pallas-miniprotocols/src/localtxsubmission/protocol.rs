use crate::{DecodePayload, EncodePayload};

#[derive(Debug, PartialEq, Clone)]
pub enum State {
    Idle,
    Busy,
    Done,
}

#[derive(Debug)]
pub enum Message<T, E>
where
    T: EncodePayload + DecodePayload,
    E: EncodePayload + DecodePayload,
{
    SubmitTx(T),
    AcceptTx,
    RejectTx(E),
    Done,
}
