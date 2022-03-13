#[derive(Debug, PartialEq, Clone)]
pub enum State {
    Idle,
    Busy,
    Done,
}

#[derive(Debug)]
pub enum Message<T, E> {
    SubmitTx(T),
    AcceptTx,
    RejectTx(E),
    Done,
}
