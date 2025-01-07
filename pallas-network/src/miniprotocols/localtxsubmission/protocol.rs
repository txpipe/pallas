use pallas_codec::utils::Bytes;

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

/// Conway era transaction errors.
/// It is partially structured; the `Raw` variant collects errors that have not
/// been implemented yet keeping their raw form (to be deprecated).
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum UtxowFailure {
    ExtraneousScriptWitnessesUTXOW(Vec<Bytes>),
    Raw(Vec<u8>),
}

/// Conway era ledger transaction errors.
/// It is partially structured; the `Raw` variant collects errors that have not
/// been implemented yet keeping their raw form (to be deprecated).
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TxError {
    ConwayUtxowFailure(UtxowFailure),
    Raw(Vec<u8>),
}

// Raw reject reason.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum RejectReason {
    EraErrors(u8, Vec<TxError>),
    Plutus(String),
}

impl From<String> for RejectReason {
    fn from(string: String) -> RejectReason {
        RejectReason::Plutus(string)
    }
}
