use super::Value;
pub use crate::miniprotocols::localstate::queries_v16::{
    Coin, ExUnits, TaggedSet, TransactionInput, TransactionOutput, UTxO,
};
use pallas_codec::minicbor::{self, Decode, Encode};
use pallas_codec::utils::{AnyCbor, Bytes};
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

pub type PolicyID = AnyCbor;
pub type TxCert = AnyCbor;
pub type Voter = AnyCbor;
pub type ProposalProcedure = AnyCbor;

/// Purpose of the script. It corresponds to
/// [`ConwayPlutusPurpose`](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/eras/conway/impl/src/Cardano/Ledger/Conway/Scripts.hs#L188-L194)
/// in the Haskell sources.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PlutusPurpose<T0, T1, T2, T3, T4, T5> {
    Spending(T0),
    Minting(T1),
    Certifying(T2),
    Rewarding(T3),
    Voting(T4),
    Proposing(T5),
}

impl<T0, T1, T2, T3, T4, T5> PlutusPurpose<T0, T1, T2, T3, T4, T5> {
    /// Returns the ordinal of the `PlutusPurpose` variant.
    pub fn ord(&self) -> u8 {
        match self {
            PlutusPurpose::Spending(_) => 0,
            PlutusPurpose::Minting(_) => 1,
            PlutusPurpose::Certifying(_) => 2,
            PlutusPurpose::Rewarding(_) => 3,
            PlutusPurpose::Voting(_) => 4,
            PlutusPurpose::Proposing(_) => 5,
        }
    }
}

/// Purpose with the corresponding item. It corresponds to
/// [`ConwayPlutusPurpose`](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/eras/conway/impl/src/Cardano/Ledger/Conway/Scripts.hs#L188-L194)
/// in the Haskell sources, where the higher-order argument `f` equals `AsItem`.
pub type PlutusPurposeItem =
    PlutusPurpose<TransactionInput, PolicyID, TxCert, Bytes, Voter, ProposalProcedure>;

/// Purpose with the corresponding index. It corresponds to
/// [`ConwayPlutusPurpose`](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/eras/conway/impl/src/Cardano/Ledger/Conway/Scripts.hs#L188-L194)
/// in the Haskell sources, where the higher-order argument `f` equals `AsIx`.
pub type PlutusPurposeIx = PlutusPurpose<u32, u32, u32, u32, u32, u32>;

#[derive(Encode, Decode, Debug, Clone, Eq, PartialEq)]
#[cbor(index_only)]
pub enum Language {
    #[n(0)]
    PlutusV1,
    #[n(1)]
    PlutusV2,
    #[n(3)]
    PlutusV3,
}

pub type ContextError = AnyCbor;
pub type FailureDescription = AnyCbor;

/// Tag mismatch description for UTXO validation. It corresponds to
/// [TagMismatchDescription](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/eras/alonzo/impl/src/Cardano/Ledger/Alonzo/Rules/Utxos.hs#L367)
/// in the Haskell sources.
///
/// Represents the reasons why a tag mismatch occurred during validation.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TagMismatchDescription {
    PassedUnexpectedly,
    // FIXME: Do we want to use `NonEmptySet`? Check other occurrences of `BTreeSet`.
    FailedUnexpectedly(BTreeSet<FailureDescription>),
}

/// Errors that can occur when collecting arguments for phase-2 scripts.
/// It corresponds to [CollectError](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/eras/alonzo/impl/src/Cardano/Ledger/Alonzo/Plutus/Evaluate.hs#L78-L83).
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum CollectError {
    NoRedeemer(PlutusPurposeItem),
    NoWitness(Bytes),
    NoCostModel(Language),
    BadTranslation(ContextError),
}

#[derive(Encode, Decode, Debug, Clone, Eq, PartialEq)]
#[cbor(transparent)]
pub struct IsValid(#[n(0)] pub bool);

/// Conway Utxo subtransition errors. It corresponds to [ConwayUtxosPredFailure](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/eras/conway/impl/src/Cardano/Ledger/Conway/Rules/Utxos.hs#L74C6-L74C28)
/// in the Haskell sources. Not to be confused with [UtxoFailure].
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum UtxosFailure {
    ValidationTagMismatch(IsValid, TagMismatchDescription),
    CollectErrors(CollectError),
}

#[derive(Encode, Decode, Debug, Clone, Eq, PartialEq)]
#[cbor(index_only)]
pub enum Network {
    #[n(0)]
    Testnet,
    #[n(1)]
    Mainnet,
}

pub type Slot = u64;

#[derive(Encode, Decode, Debug, Clone, Eq, PartialEq)]
pub struct ValidityInterval {
    #[n(0)]
    invalid_before: SMaybe<u64>,
    #[n(1)]
    invalid_hereafter: SMaybe<u64>,
}

/// Conway Utxo transaction errors. It corresponds to [ConwayUtxoPredFailure](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/eras/conway/impl/src/Cardano/Ledger/Conway/Rules/Utxo.hs#L78C6-L78C28)
/// in the Haskell sources. Not to be confused with [UtxosFailure].
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum UtxoFailure {
    UtxosFailure(UtxosFailure),
    BadInputsUTxO(TaggedSet<TransactionInput>),
    OutsideValidityIntervalUTxO(ValidityInterval, Slot),
    MaxTxSizeUTxO(u64, u64),
    InputSetEmptyUTxO,
    FeeTooSmallUTxO(u64, u64),
    ValueNotConservedUTxO(Value, Value),
    WrongNetwork(Network, TaggedSet<Bytes>),
    WrongNetworkWithdrawal(Network, TaggedSet<AnyCbor>),
    OutputTooSmallUTxO(Vec<TransactionOutput>),
    OutputBootAddrAttrsTooBig(Vec<TransactionOutput>),
    OutputTooBigUTxO(Vec<(i64, i64, TransactionOutput)>),
    InsufficientCollateral(i64, u64),
    ScriptsNotPaidUTxO(UTxO),
    ExUnitsTooBigUTxO(ExUnits, ExUnits),
    CollateralContainsNonADA(Value),
    WrongNetworkInTxBody(Network, Network),
    OutsideForecast(Slot),
    TooManyCollateralInputs(u16, u16),
    NoCollateralInputs,
    IncorrectTotalCollateralField(i64, u64),
    BabbageOutputTooSmallUTxO(Vec<(TransactionOutput, u64)>),
    BabbageNonDisjointRefInputs(AnyCbor),
}

/// An option type that de/encodes equivalently to [`StrictMaybe`](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/libs/cardano-ledger-binary/src/Cardano/Ledger/Binary/Encoding/Encoder.hs#L326-L329) in the Haskel sources.
///
/// `None` encodes as `[]`, Some(x) as `[encode(x)]`.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SMaybe<T> {
    Some(T),
    None,
}

/// Conway era transaction errors. It corresponds to [ConwayUtxowPredFailure](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/eras/conway/impl/src/Cardano/Ledger/Conway/Rules/Utxow.hs#L94)
/// in the Haskell sources.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum UtxowFailure {
    UtxoFailure(UtxoFailure),
    InvalidWitnessesUTXOW(AnyCbor),
    MissingVKeyWitnessesUTXOW(AnyCbor),
    MissingScriptWitnessesUTXOW(AnyCbor),
    ScriptWitnessNotValidatingUTXOW(AnyCbor),
    MissingTxBodyMetadataHash(Bytes),
    MissingTxMetadata(AnyCbor),
    ConflictingMetadataHash(AnyCbor, AnyCbor),
    InvalidMetadata,
    ExtraneousScriptWitnessesUTXOW(TaggedSet<Bytes>),
    MissingRedeemers(AnyCbor, AnyCbor),
    MissingRequiredDatums(AnyCbor, AnyCbor),
    NotAllowedSupplementalDatums(TaggedSet<Bytes>, TaggedSet<Bytes>),
    PPViewHashesDontMatch(SMaybe<Bytes>, SMaybe<Bytes>),
    UnspendableUTxONoDatumHash(AnyCbor),
    ExtraRedeemers(PlutusPurposeIx),
    MalformedScriptWitnesses(AnyCbor),
    MalformedReferenceScripts(AnyCbor),
}

/// Conway era ledger transaction errors, corresponding to [`ConwayLedgerPredFailure`](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/eras/conway/impl/src/Cardano/Ledger/Conway/Rules/Ledger.hs#L138-L153)
/// in the Haskell sources.
///
/// The `u8` variant appears for backward compatibility.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TxError {
    ConwayUtxowFailure(UtxowFailure),
    ConwayCertsFailure(AnyCbor),
    ConwayGovFailure(AnyCbor),
    ConwayWdrlNotDelegatedToDRep(Vec<Bytes>),
    ConwayTreasuryValueMismatch(Coin, Coin),
    ConwayTxRefScriptsSizeTooBig(i64, i64),
    ConwayMempoolFailure(String),
    U8(u8),
}

/// Reject reason. It can be a pair of an era number and a sequence of errors, or else a string.
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
