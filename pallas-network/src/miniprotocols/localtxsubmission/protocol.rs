use thiserror::Error;

use super::primitives::{Certificate, Credential, Language, StakeCredential, Voter};
use crate::miniprotocols::localstate::queries_v16::{
    Anchor, BigInt, FieldedRewardAccount, GovAction, GovActionId, PolicyId, ProposalProcedure, ProtocolVersion, ScriptHash,
    TransactionInput, TransactionOutput, Value, Vote, };
pub use crate::miniprotocols::localstate::queries_v16::{Coin, ExUnits, TaggedSet};
use crate::multiplexer;
use pallas_codec::minicbor::{self, Decode, Encode};
use pallas_codec::utils::{AnyUInt, Bytes, NonEmptyKeyValuePairs, Nullable, Set};
pub use pallas_crypto::hash::Hash;

#[derive(Error, Debug)]
pub enum Error {
    #[error("attempted to receive message while agency is ours")]
    AgencyIsOurs,

    #[error("attempted to send message while agency is theirs")]
    AgencyIsTheirs,

    #[error("inbound message is not valid for current state")]
    InvalidInbound,

    #[error("outbound message is not valid for current state")]
    InvalidOutbound,

    #[error("error while sending or receiving data through the channel")]
    ChannelError(multiplexer::Error),
}

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

/// The bytes of a transaction with an era number.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct EraTx(pub u16, pub Vec<u8>);

/// Era to be used in tx errors
/// https://github.com/IntersectMBO/cardano-api/blob/a0df586e3a14b98ae4771a192c09391dacb44564/cardano-api/internal/Cardano/Api/Eon/ShelleyBasedEra.hs#L271
#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
#[cbor(index_only)]
pub enum ShelleyBasedEra {
    #[n(1)]
    Shelley,
    #[n(2)]
    Allegra,
    #[n(3)]
    Mary,
    #[n(4)]
    Alonzo,
    #[n(5)]
    Babbage,
    #[n(6)]
    Conway,
}

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
        use PlutusPurpose::*;

        match self {
            Spending(_) => 0,
            Minting(_) => 1,
            Certifying(_) => 2,
            Rewarding(_) => 3,
            Voting(_) => 4,
            Proposing(_) => 5,
        }
    }
}

/// Purpose with the corresponding item. It corresponds to
/// [`ConwayPlutusPurpose`](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/eras/conway/impl/src/Cardano/Ledger/Conway/Scripts.hs#L188-L194)
/// in the Haskell sources, where the higher-order argument `f` equals `AsItem`.
pub type PlutusPurposeItem = PlutusPurpose<
    TransactionInput,
    PolicyId,
    ConwayTxCert,
    FieldedRewardAccount,
    Voter,
    ProposalProcedure,
>;

/// Purpose with the corresponding index. It corresponds to
/// [`ConwayPlutusPurpose`](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/eras/conway/impl/src/Cardano/Ledger/Conway/Scripts.hs#L188-L194)
/// in the Haskell sources, where the higher-order argument `f` equals `AsIx`.
pub type PlutusPurposeIx = PlutusPurpose<u64, u64, u64, u64, u64, u64>;

#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
#[cbor(flat)]
pub enum FailureDescription {
    #[n(1)]
    PlutusFailure(#[n(0)] String, #[n(1)] Bytes),
}

/// Tag mismatch description for UTXO validation. It corresponds to
/// [TagMismatchDescription](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/eras/alonzo/impl/src/Cardano/Ledger/Alonzo/Rules/Utxos.hs#L367)
/// in the Haskell sources.
///
/// Represents the reasons why a tag mismatch occurred during validation.
#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
#[cbor(flat)]
pub enum TagMismatchDescription {
    #[n(0)]
    PassedUnexpectedly,
    #[n(1)]
    FailedUnexpectedly(#[n(0)] Vec<FailureDescription>),
}

/// Errors that can occur when collecting arguments for phase-2 scripts.
/// It corresponds to [CollectError](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/eras/alonzo/impl/src/Cardano/Ledger/Alonzo/Plutus/Evaluate.hs#L78-L83).
#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
#[cbor(flat)]
pub enum CollectError {
    #[n(0)]
    NoRedeemer(#[n(0)] PlutusPurposeItem),
    #[n(1)]
    NoWitness(#[n(0)] DisplayScriptHash),
    #[n(2)]
    NoCostModel(#[n(0)] Language),
    #[n(3)]
    BadTranslation(#[n(0)] ConwayContextError),
}

pub type VotingProcedures =
    NonEmptyKeyValuePairs<Voter, NonEmptyKeyValuePairs<GovActionId, VotingProcedure>>;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct VotingProcedure {
    pub vote: Vote,
    pub anchor: Nullable<Anchor>,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
#[cbor(flat)]
pub enum ConwayContextError {
    #[n(8)]
    BabbageContextError(#[n(0)] BabbageContextError),
    #[n(9)]
    CertificateNotSupported(#[n(0)] ConwayTxCert),
    #[n(10)]
    PlutusPurposeNotSupported(#[n(0)] PlutusPurposeItem),
    #[n(11)]
    CurrentTreasuryFieldNotSupported(#[n(0)] DisplayCoin),
    #[n(12)]
    VotingProceduresFieldNotSupported(#[n(0)] DisplayVotingProcedures),
    #[n(13)]
    ProposalProceduresFieldNotSupported(#[n(0)] DisplayOSet<ProposalProcedure>),
    #[n(14)]
    TreasuryDonationFieldNotSupported(#[n(0)] DisplayCoin),
}
#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
#[cbor(transparent)]
pub struct DisplayOSet<T>(#[n(0)] pub Set<T>);

#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
#[cbor(transparent)]
pub struct DisplayVotingProcedures(#[n(0)] pub VotingProcedures);

#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
#[cbor(flat)]
pub enum BabbageContextError {
    #[n(0)]
    ByronTxOutInContext(#[n(0)] TxOutSource),
    #[n(1)]
    AlonzoMissingInput(#[n(0)] TransactionInput),
    #[n(2)]
    RedeemerPointerPointsToNothing(#[n(0)] PlutusPurposeIx),
    #[n(4)]
    InlineDatumsNotSupported(#[n(0)] TxOutSource),
    #[n(5)]
    ReferenceScriptsNotSupported(#[n(0)] TxOutSource),
    #[n(6)]
    ReferenceInputsNotSupported(#[n(0)] Set<TransactionInput>),
    #[n(7)]
    AlonzoTimeTranslationPastHorizon(#[n(0)] String),
}

#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
#[cbor(flat)]
pub enum TxOutSource {
    #[n(0)]
    Input(#[n(0)] TransactionInput),
    #[n(1)]
    Output(#[n(0)] u64),
}

// this type can be used inside a SMaybe
#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
#[cbor(transparent)]
pub struct DisplayScriptHash(#[n(0)] pub ScriptHash);

#[derive(Encode, Decode, Debug, Clone, Eq, PartialEq)]
#[cbor(transparent)]
pub struct IsValid(#[n(0)] pub bool);

/// Conway Utxo subtransition errors. It corresponds to [ConwayUtxosPredFailure](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/eras/conway/impl/src/Cardano/Ledger/Conway/Rules/Utxos.hs#L74C6-L74C28)
/// in the Haskell sources. Not to be confused with [UtxoFailure].
#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
#[cbor(flat)]
pub enum UtxosFailure {
    #[n(0)]
    ValidationTagMismatch(#[n(0)] bool, #[n(1)] TagMismatchDescription),
    #[n(1)]
    CollectErrors(#[n(0)] Array<CollectError>),
}

#[derive(Debug, Decode, Clone, Eq, PartialEq)]
#[cbor(transparent)]
pub struct SlotNo(#[n(0)] pub u64);

#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
#[cbor(transparent)]
pub struct DisplayCoin(#[n(0)] pub Coin);

impl From<&u64> for DisplayCoin {
    fn from(v: &u64) -> Self {
        DisplayCoin(Coin::U64(*v))
    }
}

impl From<&AnyUInt> for DisplayCoin {
    fn from(v: &AnyUInt) -> Self {
        DisplayCoin(*v)
    }
}

#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
#[cbor(transparent)]
pub struct DisplayAddress(#[n(0)] pub Bytes);

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Utxo(pub OHashMap<TransactionInput, TransactionOutput>);

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct OHashMap<K, V>(pub Vec<(K, V)>);

#[derive(Encode, Decode, Debug, Clone, Eq, PartialEq, PartialOrd, Ord)]
#[cbor(index_only)]
pub enum Network {
    #[n(0)]
    Testnet,
    #[n(1)]
    Mainnet,
}

impl From<Network> for u8 {
    fn from(value: Network) -> u8 {
        match value {
            Network::Mainnet => 1,
            Network::Testnet => 0,
        }
    }
}
#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
#[cbor(transparent)]
pub struct DeltaCoin(#[n(0)] pub BigInt);

pub type Slot = u64;

#[derive(Encode, Decode, Debug, Clone, Eq, PartialEq)]
pub struct ValidityInterval {
    #[n(0)]
    pub invalid_before: SMaybe<u64>,
    #[n(1)]
    pub invalid_hereafter: SMaybe<u64>,
}

/// Conway Utxo transaction errors. It corresponds to [ConwayUtxoPredFailure](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/eras/conway/impl/src/Cardano/Ledger/Conway/Rules/Utxo.hs#L78C6-L78C28)
/// in the Haskell sources. Not to be confused with [UtxosFailure].
#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
#[cbor(flat)]
pub enum UtxoFailure {
    #[n(0)]
    UtxosFailure(#[n(0)] UtxosFailure),
    #[n(1)]
    BadInputsUTxO(#[n(0)] Set<TransactionInput>),
    #[n(2)]
    OutsideValidityIntervalUTxO(#[n(0)] ValidityInterval, #[n(1)] Slot),
    #[n(3)]
    MaxTxSizeUTxO(#[n(0)] BigInt, #[n(1)] BigInt),
    #[n(4)]
    InputSetEmptyUTxO,
    #[n(5)]
    FeeTooSmallUTxO(#[n(0)] DisplayCoin, #[n(1)] DisplayCoin),
    #[n(6)]
    ValueNotConservedUTxO(#[n(0)] Value, #[n(1)] Value),
    #[n(7)]
    WrongNetwork(#[n(0)] Network, #[n(1)] Set<DisplayAddress>),
    #[n(8)]
    WrongNetworkWithdrawal(#[n(0)] Network, #[n(1)] Set<FieldedRewardAccount>),
    #[n(9)]
    OutputTooSmallUTxO(#[n(0)] Array<TransactionOutput>),
    #[n(10)]
    OutputBootAddrAttrsTooBig(#[n(0)] Array<TransactionOutput>),
    #[n(11)]
    OutputTooBigUTxO(#[n(0)] Array<(i64, i64, TransactionOutput)>),
    #[n(12)]
    InsufficientCollateral(#[n(0)] DeltaCoin, #[n(1)] DisplayCoin),
    #[n(13)]
    ScriptsNotPaidUTxO(#[n(0)] Utxo),
    #[n(14)]
    ExUnitsTooBigUTxO(#[n(0)] ExUnits, #[n(1)] ExUnits),
    #[n(15)]
    CollateralContainsNonADA(#[n(0)] Value),
    #[n(16)]
    WrongNetworkInTxBody(#[n(0)] Network, #[n(1)] Network),
    #[n(17)]
    OutsideForecast(#[n(0)] Slot),
    #[n(18)]
    TooManyCollateralInputs(#[n(0)] u64, #[n(1)] u64),
    #[n(19)]
    NoCollateralInputs,
    #[n(20)]
    IncorrectTotalCollateralField(#[n(0)] DeltaCoin, #[n(1)] DisplayCoin),
    #[n(21)]
    BabbageOutputTooSmallUTxO(#[n(0)] Array<(TransactionOutput, DisplayCoin)>),
    #[n(22)]
    BabbageNonDisjointRefInputs(#[n(0)] Vec<TransactionInput>),
}

/// An option type that de/encodes equivalently to [`StrictMaybe`](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/libs/cardano-ledger-binary/src/Cardano/Ledger/Binary/Encoding/Encoder.hs#L326-L329) in the Haskel sources.
///
/// `None` encodes as `[]`, Some(x) as `[encode(x)]`.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SMaybe<T> {
    Some(T),
    None,
}

#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
#[cbor(transparent)]
pub struct Array<T: Clone>(#[n(0)] pub Vec<T>);

#[derive(Debug, Decode, Encode, Hash, PartialEq, Eq, Clone)]
#[cbor(transparent)]
pub struct VKey(#[n(0)] pub Bytes);

#[derive(Debug, Decode, Encode, Hash, PartialEq, Eq, Clone)]
#[cbor(transparent)]
pub struct KeyHash(#[n(0)] pub Bytes);

/// Conway era transaction errors. It corresponds to [ConwayUtxowPredFailure](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/eras/conway/impl/src/Cardano/Ledger/Conway/Rules/Utxow.hs#L94)
/// in the Haskell sources.
#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
#[cbor(flat)]
pub enum ConwayUtxoWPredFailure {
    #[n(0)]
    UtxoFailure(#[n(0)] UtxoFailure),
    #[n(1)]
    InvalidWitnessesUTXOW(#[n(0)] Array<VKey>),
    #[n(2)]
    MissingVKeyWitnessesUTXOW(#[n(0)] Set<KeyHash>),
    #[n(3)]
    MissingScriptWitnessesUTXOW(#[n(0)] Set<ScriptHash>),
    #[n(4)]
    ScriptWitnessNotValidatingUTXOW(#[n(0)] Set<ScriptHash>),
    #[n(5)]
    MissingTxBodyMetadataHash(#[n(0)] Bytes),
    #[n(6)]
    MissingTxMetadata(#[n(0)] Bytes),
    #[n(7)]
    ConflictingMetadataHash(#[n(0)] Bytes, #[n(1)] Bytes),
    #[n(8)]
    InvalidMetadata(),
    #[n(9)]
    ExtraneousScriptWitnessesUTXOW(#[n(0)] Set<ScriptHash>),
    #[n(10)]
    MissingRedeemers(#[n(0)] Array<(PlutusPurposeItem, ScriptHash)>),
    #[n(11)]
    MissingRequiredDatums(#[n(0)] Set<SafeHash>, #[n(1)] Set<SafeHash>),
    #[n(12)]
    NotAllowedSupplementalDatums(#[n(0)] Set<SafeHash>, #[n(1)] Set<SafeHash>),
    #[n(13)]
    PPViewHashesDontMatch(#[n(0)] SMaybe<SafeHash>, #[n(1)] SMaybe<SafeHash>),
    #[n(14)]
    UnspendableUTxONoDatumHash(#[n(0)] Set<TransactionInput>),
    #[n(15)]
    ExtraRedeemers(#[n(0)] Array<PlutusPurposeIx>),
    #[n(16)]
    MalformedScriptWitnesses(#[n(0)] Set<ScriptHash>),
    #[n(17)]
    MalformedReferenceScripts(#[n(0)] Set<ScriptHash>),
}

#[derive(Debug, Decode, Encode, Hash, PartialEq, Eq, Clone)]
#[cbor(transparent)]
pub struct SafeHash(#[n(0)] pub Bytes);

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ConwayTxCert {
    Deleg(Certificate),
    Pool(Certificate),
    Gov(Certificate),
}
#[derive(Debug, Decode, Hash, PartialEq, Eq, Clone)]
#[cbor(transparent)]
pub struct DisplayPolicyId(#[n(0)] pub PolicyId);

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Mismatch<T>(pub T, pub T);

#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
#[cbor(transparent)]
pub struct EpochNo(#[n(0)] pub u64);

/// Conway era ledger transaction errors, corresponding to [`ConwayLedgerPredFailure`](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/eras/conway/impl/src/Cardano/Ledger/Conway/Rules/Ledger.hs#L138-L153)
/// in the Haskell sources.
///
/// The `u8` variant appears for backward compatibility.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
#[cbor(flat)]
pub enum ConwayLedgerFailure {
    #[n(1)]
    UtxowFailure(#[n(0)] ConwayUtxoWPredFailure),
    #[n(2)]
    CertsFailure(#[n(0)] ConwayCertsPredFailure),
    #[n(3)]
    GovFailure(#[n(0)] ConwayGovPredFailure),
    #[n(4)]
    WdrlNotDelegatedToDRep(#[n(0)] Vec<KeyHash>),
    #[n(5)]
    TreasuryValueMismatch(#[n(0)] DisplayCoin, #[n(1)] DisplayCoin),
    #[n(6)]
    TxRefScriptsSizeTooBig(#[n(0)] i64, #[n(1)] i64),
    #[n(7)]
    MempoolFailure(#[n(0)] String),
    #[n(8)]
    U8(#[n(0)] u8),
}
// https://github.com/IntersectMBO/cardano-ledger/blob/33e90ea03447b44a389985ca2b158568e5f4ad65/eras/conway/impl/src/Cardano/Ledger/Conway/Rules/Certs.hs#L113
#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
#[cbor(flat)]
pub enum ConwayCertsPredFailure {
    #[n(0)]
    WithdrawalsNotInRewardsCERTS(#[n(0)] OHashMap<FieldedRewardAccount, DisplayCoin>),  
    #[n(1)]
    CertFailure(#[n(0)] ConwayCertPredFailure),
}

// https://github.com/IntersectMBO/cardano-ledger/blob/33e90ea03447b44a389985ca2b158568e5f4ad65/eras/conway/impl/src/Cardano/Ledger/Conway/Rules/Cert.hs#L102
#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
#[cbor(flat)]
pub enum ConwayCertPredFailure {
    #[n(1)]
    DelegFailure(#[n(0)] ConwayDelegPredFailure),
    #[n(2)]
    PoolFailure(#[n(0)] ShelleyPoolPredFailure),
    #[n(3)]
    GovCertFailure(#[n(0)] ConwayGovCertPredFailure),
}

// Reminder, encoding of this enum should be custom, see decoder for info.
#[derive(Debug, Encode, Clone, Eq, PartialEq)]
#[cbor(flat)]
pub enum ShelleyPoolPredFailure {
    #[n(0)]
    StakePoolNotRegisteredOnKeyPOOL(#[n(0)] KeyHash),
    #[n(1)]
    StakePoolRetirementWrongEpochPOOL(#[n(0)] Mismatch<EpochNo>, #[n(1)] Mismatch<EpochNo>),
    #[n(2)]
    StakePoolCostTooLowPOOL(#[n(0)] Mismatch<DisplayCoin>),
    #[n(3)]
    WrongNetworkPOOL(#[n(0)] Mismatch<Network>, #[n(1)] KeyHash),
    #[n(4)]
    PoolMedataHashTooBig(#[n(0)] KeyHash, #[n(1)] i64),
}

// https://github.com/IntersectMBO/cardano-ledger/blob/33e90ea03447b44a389985ca2b158568e5f4ad65/eras/conway/impl/src/Cardano/Ledger/Conway/Rules/GovCert.hs#L118C6-L118C30
#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
#[cbor(flat)]
pub enum ConwayGovCertPredFailure {
    #[n(0)]
    DRepAlreadyRegistered(#[n(0)] Credential),
    #[n(1)]
    DRepNotRegistered(#[n(0)] Credential),
    #[n(2)]
    DRepIncorrectDeposit(#[n(0)] DisplayCoin, #[n(1)] DisplayCoin),
    #[n(3)]
    CommitteeHasPreviouslyResigned(#[n(0)] Credential),
    #[n(4)]
    DRepIncorrectRefund(#[n(0)] DisplayCoin, #[n(1)] DisplayCoin),
    #[n(5)]
    CommitteeIsUnknown(#[n(0)] Credential),
}

// https://github.com/IntersectMBO/cardano-ledger/blob/b14ba8190e21ced6cc68c18a02dd1dbc2ff45a3c/eras/conway/impl/src/Cardano/Ledger/Conway/Rules/Deleg.hs#L104
#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
#[cbor(flat)]
pub enum ConwayDelegPredFailure {
    #[n(1)]
    IncorrectDepositDELEG(#[n(0)] DisplayCoin),
    #[n(2)]
    StakeKeyRegisteredDELEG(#[n(0)] Credential),
    #[n(3)]
    StakeKeyNotRegisteredDELEG(#[n(0)] Credential),
    #[n(4)]
    StakeKeyHasNonZeroRewardAccountBalanceDELEG(#[n(0)] DisplayCoin),
    #[n(5)]
    DelegateeDRepNotRegisteredDELEG(#[n(0)] Credential),
    #[n(6)]
    DelegateeStakePoolNotRegisteredDELEG(#[n(0)] KeyHash),
}

// https://github.com/IntersectMBO/cardano-ledger/blob/33e90ea03447b44a389985ca2b158568e5f4ad65/eras/conway/impl/src/Cardano/Ledger/Conway/Rules/Gov.hs#L164
#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
#[cbor(flat)]
pub enum ConwayGovPredFailure {
    #[n(0)]
    GovActionsDoNotExist(#[n(0)] Vec<GovActionId>),
    #[n(1)]
    MalformedProposal(#[n(0)] GovAction),
    #[n(2)]
    ProposalProcedureNetworkIdMismatch(#[n(0)] FieldedRewardAccount, #[n(1)] Network),
    #[n(3)]
    TreasuryWithdrawalsNetworkIdMismatch(#[n(0)] Set<FieldedRewardAccount>, #[n(1)] Network),
    #[n(4)]
    ProposalDepositIncorrect(#[n(0)] DisplayCoin, #[n(1)] DisplayCoin),
    #[n(5)]
    DisallowedVoters(#[n(0)] Vec<(Voter, GovActionId)>),
    #[n(6)]
    ConflictingCommitteeUpdate(#[n(0)] Set<Credential>),
    #[n(7)]
    ExpirationEpochTooSmall(#[n(0)] OHashMap<StakeCredential, EpochNo>),
    #[n(8)]
    InvalidPrevGovActionId(#[n(0)] ProposalProcedure),
    #[n(9)]
    VotingOnExpiredGovAction(#[n(0)] Vec<(Voter, GovActionId)>),
    #[n(10)]
    ProposalCantFollow(
        #[n(0)] SMaybe<GovActionId>,
        #[n(1)] ProtocolVersion,
        #[n(2)] ProtocolVersion,
    ),
    #[n(11)]
    InvalidPolicyHash(
        #[n(0)] SMaybe<DisplayScriptHash>,
        #[n(1)] SMaybe<DisplayScriptHash>,
    ),
    #[n(12)]
    DisallowedProposalDuringBootstrap(#[n(0)] ProposalProcedure),
    #[n(13)]
    DisallowedVotesDuringBootstrap(#[n(0)] Vec<(Voter, GovActionId)>),
    #[n(14)]
    VotersDoNotExist(#[n(0)] Vec<Voter>),
    #[n(15)]
    ZeroTreasuryWithdrawals(#[n(0)] GovAction),
    #[n(16)]
    ProposalReturnAccountDoesNotExist(#[n(0)] FieldedRewardAccount),
    #[n(17)]
    TreasuryWithdrawalReturnAccountsDoNotExist(#[n(0)] Vec<FieldedRewardAccount>),
}

/// Reject reason. It can be a pair of an era number and a sequence of errors,
/// or else a string.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TxValidationError {
    ByronTxValidationError {
        error: ApplyTxError,
    },
    ShelleyTxValidationError {
        error: ApplyTxError,
        era: ShelleyBasedEra,
    },
    Plutus(String),
}

impl From<String> for TxValidationError {
    fn from(string: String) -> TxValidationError {
        TxValidationError::Plutus(string)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ApplyTxError(pub Vec<ConwayLedgerFailure>);
