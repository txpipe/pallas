#[derive(Debug, PartialEq, Eq, Clone)]
pub enum State {
    Idle,
    Busy,
    Done,
}

#[derive(Debug)]
pub enum Message<Tx, Reject> {
    SubmitTx(Tx),
    AcceptTx,
    RejectTx(Reject),
    Done,
}

// The bytes of a transaction with an era number.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct EraTx(pub u16, pub Vec<u8>);

/// Raw reject reason, as CBOR bytes. Note that the given bytes may not represent a complete error
/// response, as the multiplexer's segment length is at most `MAX_SEGMENT_PAYLOAD_LENGTH` bytes.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RejectReason(pub Vec<u8>);
