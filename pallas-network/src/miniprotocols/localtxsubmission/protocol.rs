pub use crate::miniprotocols::localstate::queries_v16::TransactionInput;
use pallas_codec::utils::Bytes;
use std::collections::BTreeSet;

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

/// Conway Utxo transaction errors. It corresponds to [ConwayUtxoFailure](https://github.com/IntersectMBO/cardano-ledger/blob/b7fe1c31edabf8863669d8948f362e78bbbae14c/eras/conway/impl/src/Cardano/Ledger/Conway/Rules/Utxo.hs#L83)
/// in the Haskell sources.
///
/// It is partially structured; the `Raw` variant collects errors that have not
/// been implemented yet keeping their raw form (to be deprecated).
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum UtxoFailure {
    BadInputsUTxO(BTreeSet<TransactionInput>),
    Raw(Vec<u8>),
}

/// Conway era transaction errors. It corresponds to [ConwayUtxowPredFailure](https://github.com/IntersectMBO/cardano-ledger/blob/12f6aa6f094af5dab722edf03d4ab3f4ec99aa48/eras/conway/impl/src/Cardano/Ledger/Conway/Rules/Utxow.hs#L77)
/// in the Haskell sources.
///
/// It is partially structured; the `Raw` variant collects errors that have not
/// been implemented yet keeping their raw form (to be deprecated).
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum UtxowFailure {
    ExtraneousScriptWitnessesUTXOW(Vec<Bytes>),
    UtxoFailure(UtxoFailure),
    U8(u8),
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
