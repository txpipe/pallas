//! Dijkstra-era transaction rejections (unstable).
//!
//! Mirrors the predicate failures of the Dijkstra ledger rules at
//! [cardano-ledger `9f6a9fb6`](https://github.com/IntersectMBO/cardano-ledger/tree/9f6a9fb68/eras/dijkstra/impl/src/Cardano/Ledger/Dijkstra/Rules).
//! The root rule of a Dijkstra rejection is `MEMPOOL` (not `LEDGER` as in
//! Conway) and the tags differ from Conway's, hence a separate tree.
//!
//! Payloads whose Dijkstra encoding has no type in this crate yet (transaction
//! outputs, certificates, governance actions and proposals, account balance
//! intervals, BLS keys, script context errors) are kept as
//! [`AnyCbor`](pallas_codec::utils::AnyCbor).

use pallas_codec::minicbor::{self, Decode, Encode};

// Types whose encoding Dijkstra keeps unchanged, re-exported so this module
// names every type its failures carry.
pub use pallas_codec::utils::{AnyCbor, Bytes, Set};
pub use pallas_crypto::hash::Hash;

pub use super::primitives::Voter;
pub use super::{
    DisplayAddress, DisplayRewardAccount, FailureDescription, KeyHash, Network, OHashMap, SMaybe,
    SafeHash, TagMismatchDescription, VKey, ValidityInterval,
};
pub use crate::miniprotocols::localstate::queries_v16::{
    Coin, ExUnits, GovActionId, PolicyId, ProtocolVersion, ScriptHash, TransactionInput, Value,
};

/// Rejection of a transaction by the Dijkstra ledger, as sent in `MsgRejectTx`:
/// `DijkstraApplyTxError (NonEmpty (DijkstraMempoolPredFailure DijkstraEra))`.
#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
#[cbor(transparent)]
pub struct ApplyTxError(#[n(0)] pub Vec<MempoolFailure>);

/// A `Mismatch` between a supplied and an expected value, encoded as a 2-array
/// (unlike the Conway types of this crate, which read the two values flattened).
#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
pub struct Mismatch<T> {
    #[n(0)]
    pub supplied: T,
    #[n(1)]
    pub expected: T,
}

/// A ledger `Credential`: key hash is tag 0, script hash tag 1
/// ([`Credential.hs`](https://github.com/IntersectMBO/cardano-ledger/blob/9f6a9fb68/libs/cardano-ledger-core/src/Cardano/Ledger/Credential.hs)).
///
/// Not [`super::primitives::Credential`]: that one swaps the two tags
/// (`ScriptHashObj` = 0), so it decodes a key hash as a script hash and vice
/// versa. It is left as is because fixing it changes how existing Conway
/// rejections decode, which belongs in its own change.
#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
#[cbor(flat)]
pub enum Credential {
    #[n(0)]
    KeyHash(#[n(0)] Hash<28>),
    #[n(1)]
    ScriptHash(#[n(0)] ScriptHash),
}

/// Plutus language version, Dijkstra adds Plutus V4.
#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
#[cbor(index_only)]
pub enum Language {
    #[n(0)]
    PlutusV1,
    #[n(1)]
    PlutusV2,
    #[n(2)]
    PlutusV3,
    #[n(3)]
    PlutusV4,
}

/// `DijkstraPlutusPurpose AsItem`.
#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
#[cbor(flat)]
pub enum PlutusPurposeItem {
    #[n(0)]
    Spending(#[n(0)] TransactionInput),
    #[n(1)]
    Minting(#[n(0)] PolicyId),
    #[n(2)]
    Certifying(#[n(0)] AnyCbor),
    #[n(3)]
    Withdrawing(#[n(0)] DisplayRewardAccount),
    #[n(4)]
    Voting(#[n(0)] Voter),
    #[n(5)]
    Proposing(#[n(0)] AnyCbor),
    #[n(6)]
    Guarding(#[n(0)] ScriptHash),
}

/// `DijkstraPlutusPurpose AsIx`: the purpose and the index of its item.
#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
#[cbor(flat)]
pub enum PlutusPurposeIx {
    #[n(0)]
    Spending(#[n(0)] u32),
    #[n(1)]
    Minting(#[n(0)] u32),
    #[n(2)]
    Certifying(#[n(0)] u32),
    #[n(3)]
    Withdrawing(#[n(0)] u32),
    #[n(4)]
    Voting(#[n(0)] u32),
    #[n(5)]
    Proposing(#[n(0)] u32),
    #[n(6)]
    Guarding(#[n(0)] u32),
}

/// `DijkstraMempoolPredFailure` (`Rules/Mempool.hs`).
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
#[cbor(flat)]
pub enum MempoolFailure {
    #[n(1)]
    LedgerFailure(#[n(0)] LedgerFailure),
    #[n(2)]
    MempoolFailure(#[n(0)] String),
    #[n(3)]
    AllInputsAreSpent,
}

/// `DijkstraLedgerPredFailure` (`Rules/Ledger.hs`).
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
#[cbor(flat)]
pub enum LedgerFailure {
    #[n(1)]
    UtxowFailure(#[n(0)] UtxowFailure),
    #[n(2)]
    EntitiesFailure(#[n(0)] EntitiesFailure),
    #[n(3)]
    GovFailure(#[n(0)] GovFailure),
    #[n(4)]
    TreasuryValueMismatch(#[n(0)] Mismatch<Coin>),
    #[n(5)]
    TxRefScriptsSizeTooBig(#[n(0)] Mismatch<i64>),
    /// `DijkstraSubLedgersPredFailure` is a newtype over the `SUBLEDGER` failure.
    #[n(6)]
    SubLedgersFailure(#[n(0)] SubLedgerFailure),
}

/// `DijkstraUtxowPredFailure` (`Rules/Utxow.hs`).
#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
#[cbor(flat)]
pub enum UtxowFailure {
    #[n(0)]
    UtxoFailure(#[n(0)] UtxoFailure),
    #[n(1)]
    InvalidWitnessesUTXOW(#[n(0)] Vec<VKey>),
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
    ConflictingMetadataHash(#[n(0)] Mismatch<Bytes>),
    #[n(8)]
    InvalidMetadata,
    #[n(9)]
    ExtraneousScriptWitnessesUTXOW(#[n(0)] Set<ScriptHash>),
    #[n(10)]
    MissingRedeemers(#[n(0)] Vec<(PlutusPurposeItem, ScriptHash)>),
    #[n(11)]
    MissingRequiredDatums(#[n(0)] Set<SafeHash>, #[n(1)] Set<SafeHash>),
    #[n(12)]
    NotAllowedSupplementalDatums(#[n(0)] Set<SafeHash>, #[n(1)] Set<SafeHash>),
    #[n(13)]
    PPViewHashesDontMatch(#[n(0)] Mismatch<SMaybe<SafeHash>>),
    #[n(14)]
    UnspendableUTxONoDatumHash(#[n(0)] Set<TransactionInput>),
    #[n(15)]
    ExtraRedeemers(#[n(0)] Vec<PlutusPurposeIx>),
    #[n(16)]
    MalformedScriptWitnesses(#[n(0)] Set<ScriptHash>),
    #[n(17)]
    MalformedReferenceScripts(#[n(0)] Set<ScriptHash>),
    #[n(18)]
    ScriptIntegrityHashMismatch(#[n(0)] Mismatch<SMaybe<SafeHash>>, #[n(1)] SMaybe<Bytes>),
    #[n(19)]
    MissingRequiredGuards(#[n(0)] Set<Credential>),
    #[n(20)]
    MalformedGuardDatums(#[n(0)] Set<Credential>),
}

/// `DijkstraUtxoPredFailure` (`Rules/Utxo.hs`). Tag 8 is unused.
#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
#[cbor(flat)]
pub enum UtxoFailure {
    #[n(0)]
    UtxosFailure(#[n(0)] UtxosFailure),
    #[n(1)]
    BadInputsUTxO(#[n(0)] Set<TransactionInput>),
    #[n(2)]
    OutsideValidityIntervalUTxO(#[n(0)] ValidityInterval, #[n(1)] u64),
    #[n(3)]
    MaxTxSizeUTxO(#[n(0)] Mismatch<u32>),
    #[n(4)]
    InputSetEmptyUTxO,
    #[n(5)]
    FeeTooSmallUTxO(#[n(0)] Mismatch<Coin>),
    #[n(6)]
    ValueNotConservedUTxO(#[n(0)] Mismatch<Value>),
    #[n(7)]
    WrongNetwork(#[n(0)] Network, #[n(1)] Set<DisplayAddress>),
    #[n(9)]
    OutputBootAddrAttrsTooBig(#[n(0)] Vec<AnyCbor>),
    #[n(10)]
    OutputTooBigUTxO(#[n(0)] Vec<(i64, i64, AnyCbor)>),
    #[n(11)]
    InsufficientCollateral(#[n(0)] i64, #[n(1)] Coin),
    #[n(12)]
    ScriptsNotPaidUTxO(#[n(0)] OHashMap<TransactionInput, AnyCbor>),
    #[n(13)]
    ExUnitsTooBigUTxO(#[n(0)] Mismatch<ExUnits>),
    #[n(14)]
    CollateralContainsNonADA(#[n(0)] Value),
    #[n(15)]
    WrongNetworkInTxBody(#[n(0)] Mismatch<Network>),
    #[n(16)]
    OutsideForecast(#[n(0)] u64),
    #[n(17)]
    TooManyCollateralInputs(#[n(0)] Mismatch<u16>),
    #[n(18)]
    NoCollateralInputs,
    #[n(19)]
    IncorrectTotalCollateralField(#[n(0)] i64, #[n(1)] Coin),
    #[n(20)]
    BabbageOutputTooSmallUTxO(#[n(0)] Vec<(AnyCbor, Coin)>),
    #[n(21)]
    BabbageNonDisjointRefInputs(#[n(0)] Vec<TransactionInput>),
    #[n(22)]
    PtrPresentInCollateralReturn(#[n(0)] AnyCbor),
    #[n(23)]
    ValueNotConservedInLegacyMode(#[n(0)] Mismatch<Value>),
}

/// `ConwayUtxosPredFailure DijkstraEra` (Conway `Rules/Utxos.hs`).
#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
#[cbor(flat)]
pub enum UtxosFailure {
    #[n(0)]
    ValidationTagMismatch(#[n(0)] bool, #[n(1)] TagMismatchDescription),
    #[n(1)]
    CollectErrors(#[n(0)] Vec<CollectError>),
}

/// `CollectError DijkstraEra` (Alonzo `Plutus/Context.hs`).
#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
#[cbor(flat)]
pub enum CollectError {
    #[n(0)]
    NoRedeemer(#[n(0)] PlutusPurposeItem),
    #[n(1)]
    NoWitness(#[n(0)] ScriptHash),
    #[n(2)]
    NoCostModel(#[n(0)] Language),
    /// `DijkstraContextError`, kept encoded.
    #[n(3)]
    BadTranslation(#[n(0)] AnyCbor),
}

/// `EntitiesPredFailure` (`Rules/Entities.hs`). Account balance intervals are
/// kept encoded.
#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
#[cbor(flat)]
pub enum EntitiesFailure {
    /// `ConwayCertsPredFailure DijkstraEra` is a one-constructor sum (tag 1)
    /// over the `CERT` failure.
    #[n(0)]
    CertsFailure(#[n(0)] CertsFailure),
    #[n(1)]
    WithdrawalAddressesWithWrongNetwork(#[n(0)] Network, #[n(1)] Set<DisplayRewardAccount>),
    #[n(2)]
    WithdrawalAccountsMissing(#[n(0)] OHashMap<DisplayRewardAccount, Coin>),
    #[n(3)]
    WithdrawalAccountsMissingFromOriginal(#[n(0)] OHashMap<DisplayRewardAccount, Coin>),
    #[n(4)]
    WithdrawalAmountsInexactInLegacyMode(#[n(0)] OHashMap<DisplayRewardAccount, Mismatch<Coin>>),
    #[n(5)]
    WithdrawalAmountsExceedingOriginalBalance(
        #[n(0)] OHashMap<DisplayRewardAccount, Mismatch<Coin>>,
    ),
    #[n(6)]
    DirectDepositAddressesWithWrongNetwork(#[n(0)] Network, #[n(1)] Set<DisplayRewardAccount>),
    #[n(7)]
    DirectDepositAccountsMissing(#[n(0)] OHashMap<DisplayRewardAccount, Coin>),
    #[n(8)]
    WrongNetworkInAccountBalanceIntervals(#[n(0)] Network, #[n(1)] Set<DisplayRewardAccount>),
    #[n(9)]
    MissingAccountsInAccountBalanceIntervals(#[n(0)] OHashMap<DisplayRewardAccount, AnyCbor>),
    #[n(10)]
    BalancesOutsideAccountBalanceIntervals(#[n(0)] OHashMap<DisplayRewardAccount, (Coin, AnyCbor)>),
    #[n(11)]
    WrongNetworkInStartingAccountBalanceIntervals(
        #[n(0)] Network,
        #[n(1)] Set<DisplayRewardAccount>,
    ),
    #[n(12)]
    MissingAccountsInStartingAccountBalanceIntervals(
        #[n(0)] OHashMap<DisplayRewardAccount, AnyCbor>,
    ),
    #[n(13)]
    BalancesOutsideStartingAccountBalanceIntervals(
        #[n(0)] OHashMap<DisplayRewardAccount, (Coin, AnyCbor)>,
    ),
}

/// `ConwayCertsPredFailure DijkstraEra` (Conway `Rules/Certs.hs`).
#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
#[cbor(flat)]
pub enum CertsFailure {
    #[n(1)]
    CertFailure(#[n(0)] CertFailure),
}

/// `ConwayCertPredFailure DijkstraEra` (Conway `Rules/Cert.hs`), also the
/// payload of `DijkstraSubCertPredFailure` (`Rules/SubCert.hs`, same tags).
#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
#[cbor(flat)]
pub enum CertFailure {
    #[n(1)]
    DelegFailure(#[n(0)] DelegFailure),
    #[n(2)]
    PoolFailure(#[n(0)] PoolFailure),
    #[n(3)]
    GovCertFailure(#[n(0)] GovCertFailure),
}

/// `ConwayDelegPredFailure DijkstraEra` (Conway `Rules/Deleg.hs`).
#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
#[cbor(flat)]
pub enum DelegFailure {
    #[n(1)]
    IncorrectDepositDELEG(#[n(0)] Coin),
    #[n(2)]
    DelegAccountAlreadyRegistered(#[n(0)] Credential),
    #[n(3)]
    StakeKeyNotRegisteredDELEG(#[n(0)] Credential),
    #[n(4)]
    StakeKeyHasNonZeroAccountBalanceDELEG(#[n(0)] Coin),
    #[n(5)]
    DelegateeDRepNotRegisteredDELEG(#[n(0)] Credential),
    #[n(6)]
    DelegateeStakePoolNotRegisteredDELEG(#[n(0)] KeyHash),
    #[n(7)]
    DepositIncorrectDELEG(#[n(0)] Mismatch<Coin>),
    #[n(8)]
    RefundIncorrectDELEG(#[n(0)] Mismatch<Coin>),
}

/// `DijkstraPoolPredFailure` (`Rules/Pool.hs`). Tags 2 and 3 are unused.
#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
#[cbor(flat)]
pub enum PoolFailure {
    #[n(0)]
    StakePoolNotRegisteredOnKeyPOOL(#[n(0)] KeyHash),
    #[n(1)]
    StakePoolRetirementWrongEpochPOOL(#[n(0)] Mismatch<u64>, #[n(1)] Mismatch<u64>),
    #[n(4)]
    WrongNetworkPOOL(#[n(0)] Mismatch<Network>, #[n(1)] KeyHash),
    #[n(5)]
    PoolMedataHashTooBig(#[n(0)] KeyHash, #[n(1)] i64),
    #[n(6)]
    VRFKeyHashAlreadyRegistered(#[n(0)] KeyHash, #[n(1)] Bytes),
    #[n(7)]
    BlsKeyInvalidProofOfPossession(#[n(0)] KeyHash, #[n(1)] AnyCbor),
    #[n(8)]
    BlsKeyAlreadyRegistered(#[n(0)] KeyHash, #[n(1)] AnyCbor),
}

/// `DijkstraGovCertPredFailure` (`Rules/GovCert.hs`).
#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
#[cbor(flat)]
pub enum GovCertFailure {
    #[n(0)]
    DRepAlreadyRegistered(#[n(0)] Credential),
    #[n(1)]
    DRepNotRegistered(#[n(0)] Credential),
    #[n(2)]
    DRepIncorrectDeposit(#[n(0)] Mismatch<Coin>),
    #[n(3)]
    CommitteeHasPreviouslyResigned(#[n(0)] Credential),
    #[n(4)]
    DRepIncorrectRefund(#[n(0)] Mismatch<Coin>),
    #[n(5)]
    CommitteeIsUnknown(#[n(0)] Credential),
}

/// `DijkstraGovPredFailure` (`Rules/Gov.hs`), also the payload of
/// `DijkstraSubGovPredFailure`. Governance actions and proposals are kept
/// encoded.
#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
#[cbor(flat)]
pub enum GovFailure {
    #[n(0)]
    GovActionsDoNotExist(#[n(0)] Vec<GovActionId>),
    #[n(1)]
    MalformedProposal(#[n(0)] AnyCbor),
    #[n(2)]
    ProposalProcedureNetworkIdMismatch(#[n(0)] DisplayRewardAccount, #[n(1)] Network),
    #[n(3)]
    TreasuryWithdrawalsNetworkIdMismatch(#[n(0)] Set<DisplayRewardAccount>, #[n(1)] Network),
    #[n(4)]
    ProposalDepositIncorrect(#[n(0)] Mismatch<Coin>),
    #[n(5)]
    DisallowedVoters(#[n(0)] Vec<(Voter, GovActionId)>),
    #[n(6)]
    ConflictingCommitteeUpdate(#[n(0)] Set<Credential>),
    #[n(7)]
    ExpirationEpochTooSmall(#[n(0)] OHashMap<Credential, u64>),
    #[n(8)]
    InvalidPrevGovActionId(#[n(0)] AnyCbor),
    #[n(9)]
    VotingOnExpiredGovAction(#[n(0)] Vec<(Voter, GovActionId)>),
    #[n(10)]
    ProposalCantFollow(
        #[n(0)] SMaybe<GovActionId>,
        #[n(1)] Mismatch<ProtocolVersion>,
    ),
    #[n(11)]
    InvalidGuardrailsScriptHash(#[n(0)] SMaybe<ScriptHash>, #[n(1)] SMaybe<ScriptHash>),
    #[n(12)]
    DisallowedProposalDuringBootstrap(#[n(0)] AnyCbor),
    #[n(13)]
    DisallowedVotesDuringBootstrap(#[n(0)] Vec<(Voter, GovActionId)>),
    #[n(14)]
    VotersDoNotExist(#[n(0)] Vec<Voter>),
    #[n(15)]
    ZeroTreasuryWithdrawals(#[n(0)] AnyCbor),
    #[n(16)]
    ProposalReturnAccountDoesNotExist(#[n(0)] DisplayRewardAccount),
    #[n(17)]
    TreasuryWithdrawalReturnAccountsDoNotExist(#[n(0)] Vec<DisplayRewardAccount>),
    #[n(18)]
    UnelectedCommitteeVoters(#[n(0)] Vec<Credential>),
}

/// `DijkstraSubLedgerPredFailure` (`Rules/SubLedger.hs`): failure of a
/// sub-transaction. Tag 4 is unused.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
#[cbor(flat)]
pub enum SubLedgerFailure {
    #[n(1)]
    SubUtxowFailure(#[n(0)] SubUtxowFailure),
    #[n(2)]
    SubEntitiesFailure(#[n(0)] SubEntitiesFailure),
    #[n(3)]
    SubGovFailure(#[n(0)] GovFailure),
    #[n(5)]
    SubTreasuryValueMismatch(#[n(0)] Mismatch<Coin>),
}

/// `DijkstraSubUtxowPredFailure` (`Rules/SubUtxow.hs`).
#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
#[cbor(flat)]
pub enum SubUtxowFailure {
    #[n(0)]
    SubUtxoFailure(#[n(0)] SubUtxoFailure),
    #[n(1)]
    SubInvalidWitnessesUTXOW(#[n(0)] Vec<VKey>),
    #[n(2)]
    SubMissingVKeyWitnessesUTXOW(#[n(0)] Set<KeyHash>),
    #[n(3)]
    SubScriptWitnessNotValidatingUTXOW(#[n(0)] Set<ScriptHash>),
    #[n(4)]
    SubMissingTxBodyMetadataHash(#[n(0)] Bytes),
    #[n(5)]
    SubMissingTxMetadata(#[n(0)] Bytes),
    #[n(6)]
    SubConflictingMetadataHash(#[n(0)] Mismatch<Bytes>),
    #[n(7)]
    SubInvalidMetadata,
    #[n(8)]
    SubMissingRedeemers(#[n(0)] Vec<(PlutusPurposeItem, ScriptHash)>),
    #[n(9)]
    SubMissingRequiredDatums(#[n(0)] Set<SafeHash>, #[n(1)] Set<SafeHash>),
    #[n(10)]
    SubNotAllowedSupplementalDatums(#[n(0)] Set<SafeHash>, #[n(1)] Set<SafeHash>),
    #[n(11)]
    SubPPViewHashesDontMatch(#[n(0)] Mismatch<SMaybe<SafeHash>>),
    #[n(12)]
    SubUnspendableUTxONoDatumHash(#[n(0)] Set<TransactionInput>),
    #[n(13)]
    SubExtraRedeemers(#[n(0)] Vec<PlutusPurposeIx>),
    #[n(14)]
    SubMalformedScriptWitnesses(#[n(0)] Set<ScriptHash>),
    #[n(15)]
    SubMalformedReferenceScripts(#[n(0)] Set<ScriptHash>),
    #[n(16)]
    SubScriptIntegrityHashMismatch(#[n(0)] Mismatch<SMaybe<SafeHash>>, #[n(1)] SMaybe<Bytes>),
    #[n(17)]
    SubMalformedGuardDatums(#[n(0)] Set<Credential>),
}

/// `DijkstraSubUtxoPredFailure` (`Rules/SubUtxo.hs`). Tag 5 is unused.
#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
#[cbor(flat)]
pub enum SubUtxoFailure {
    #[n(0)]
    SubBadInputsUTxO(#[n(0)] Set<TransactionInput>),
    #[n(1)]
    SubOutsideValidityIntervalUTxO(#[n(0)] ValidityInterval, #[n(1)] u64),
    #[n(2)]
    SubMaxTxSizeUTxO(#[n(0)] Mismatch<u32>),
    #[n(3)]
    SubInputSetEmptyUTxO,
    #[n(4)]
    SubWrongNetwork(#[n(0)] Network, #[n(1)] Set<DisplayAddress>),
    #[n(6)]
    SubOutputBootAddrAttrsTooBig(#[n(0)] Vec<AnyCbor>),
    #[n(7)]
    SubOutputTooBigUTxO(#[n(0)] Vec<(i64, i64, AnyCbor)>),
    #[n(8)]
    SubWrongNetworkInTxBody(#[n(0)] Mismatch<Network>),
    #[n(9)]
    SubOutsideForecast(#[n(0)] u64),
    #[n(10)]
    SubBabbageOutputTooSmallUTxO(#[n(0)] Vec<(AnyCbor, Coin)>),
}

/// `SubEntitiesPredFailure` (`Rules/SubEntities.hs`). Account balance
/// intervals are kept encoded.
#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
#[cbor(flat)]
pub enum SubEntitiesFailure {
    /// `DijkstraSubCertsPredFailure` is a newtype over the `SUBCERT` failure,
    /// which has the tags of [`CertFailure`].
    #[n(0)]
    SubCertsFailure(#[n(0)] CertFailure),
    #[n(1)]
    SubWithdrawalAddressesWithWrongNetwork(#[n(0)] Network, #[n(1)] Set<DisplayRewardAccount>),
    #[n(2)]
    SubWithdrawalAccountsMissing(#[n(0)] OHashMap<DisplayRewardAccount, Coin>),
    #[n(3)]
    SubWithdrawalAccountsMissingFromOriginal(#[n(0)] OHashMap<DisplayRewardAccount, Coin>),
    #[n(4)]
    SubDirectDepositAddressesWithWrongNetwork(#[n(0)] Network, #[n(1)] Set<DisplayRewardAccount>),
    #[n(5)]
    SubDirectDepositAccountsMissing(#[n(0)] OHashMap<DisplayRewardAccount, Coin>),
    #[n(6)]
    SubBalancesOutsideAccountBalanceIntervals(
        #[n(0)] OHashMap<DisplayRewardAccount, (Coin, AnyCbor)>,
    ),
    #[n(7)]
    SubMissingAccountsInAccountBalanceIntervals(#[n(0)] OHashMap<DisplayRewardAccount, AnyCbor>),
    #[n(8)]
    SubWrongNetworkInAccountBalanceIntervals(#[n(0)] Network, #[n(1)] Set<DisplayRewardAccount>),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::miniprotocols::localtxsubmission::{
        EraTx, Message, ShelleyBasedEra, TxValidationError,
    };

    // Hand-built from the cardano-ledger `9f6a9fb6` encoders: `[[7, ApplyTxError]]`,
    // the reject reason of a Dijkstra `MsgRejectTx`.
    const ALL_INPUTS_ARE_SPENT: &str = "818207818103";
    const MEMPOOL_TEXT: &str = "81820781820264626f6f6d";
    const BAD_INPUTS: &str = "818207818201820182008201d9010281825820000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f00";
    const FEE_TOO_SMALL: &str = "81820781820182018200820582186418c8";
    const SCRIPT_FAILED: &str = "8182078182018201820082008300f4820181830163657272420102";
    const STAKE_KEY_NOT_REGISTERED: &str = "818207818201820282008201820182038200581c000102030405060708090a0b0c0d0e0f101112131415161718191a1b";
    const SUB_TX_BAD_INPUTS: &str = "8182078182018206820182008200d9010281825820000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f01";

    fn decode_reject(hex_cbor: &str) -> ApplyTxError {
        let bytes = hex::decode(hex_cbor).unwrap();
        match minicbor::decode(&bytes).unwrap() {
            TxValidationError::DijkstraTxValidationError { error, era } => {
                assert_eq!(era, ShelleyBasedEra::Dijkstra);
                // The encoder writes back the same bytes (after `[[7, `).
                assert_eq!(
                    hex::encode(minicbor::to_vec(&error).unwrap()),
                    hex_cbor[6..]
                );
                error
            }
            other => panic!("expected a Dijkstra rejection, got {other:?}"),
        }
    }

    fn ledger(failure: LedgerFailure) -> ApplyTxError {
        ApplyTxError(vec![MempoolFailure::LedgerFailure(failure)])
    }

    fn input(index: u64) -> TransactionInput {
        TransactionInput {
            transaction_id: Hash::new(core::array::from_fn(|i| i as u8)),
            index,
        }
    }

    #[test]
    fn decodes_mempool_failures() {
        assert_eq!(
            decode_reject(ALL_INPUTS_ARE_SPENT),
            ApplyTxError(vec![MempoolFailure::AllInputsAreSpent])
        );
        assert_eq!(
            decode_reject(MEMPOOL_TEXT),
            ApplyTxError(vec![MempoolFailure::MempoolFailure("boom".to_string())])
        );
    }

    #[test]
    fn decodes_utxo_failures() {
        assert_eq!(
            decode_reject(BAD_INPUTS),
            ledger(LedgerFailure::UtxowFailure(UtxowFailure::UtxoFailure(
                UtxoFailure::BadInputsUTxO(vec![input(0)].into())
            )))
        );
        assert_eq!(
            decode_reject(FEE_TOO_SMALL),
            ledger(LedgerFailure::UtxowFailure(UtxowFailure::UtxoFailure(
                UtxoFailure::FeeTooSmallUTxO(Mismatch {
                    supplied: Coin::U8(100),
                    expected: Coin::U8(200),
                })
            )))
        );
    }

    #[test]
    fn decodes_script_failures() {
        assert_eq!(
            decode_reject(SCRIPT_FAILED),
            ledger(LedgerFailure::UtxowFailure(UtxowFailure::UtxoFailure(
                UtxoFailure::UtxosFailure(UtxosFailure::ValidationTagMismatch(
                    false,
                    TagMismatchDescription::FailedUnexpectedly(vec![
                        FailureDescription::PlutusFailure("err".to_string(), vec![1, 2].into(),)
                    ]),
                ))
            )))
        );
    }

    #[test]
    fn decodes_certificate_failures() {
        assert_eq!(
            decode_reject(STAKE_KEY_NOT_REGISTERED),
            ledger(LedgerFailure::EntitiesFailure(
                EntitiesFailure::CertsFailure(CertsFailure::CertFailure(
                    CertFailure::DelegFailure(DelegFailure::StakeKeyNotRegisteredDELEG(
                        Credential::KeyHash(Hash::new(core::array::from_fn(|i| i as u8)))
                    ))
                ))
            ))
        );
    }

    #[test]
    fn decodes_sub_transaction_failures() {
        assert_eq!(
            decode_reject(SUB_TX_BAD_INPUTS),
            ledger(LedgerFailure::SubLedgersFailure(
                SubLedgerFailure::SubUtxowFailure(SubUtxowFailure::SubUtxoFailure(
                    SubUtxoFailure::SubBadInputsUTxO(vec![input(1)].into())
                ))
            ))
        );
    }

    #[test]
    fn decodes_reject_message_captured_on_musashi() {
        // A `MsgRejectTx` as the node sent it, see `test_data/dijkstra-fixtures.md`.
        let captured = include_str!("../../../../test_data/dijkstra-all-inputs-spent.reject");
        // Same bytes as the hand-built fixture, after the `MsgRejectTx` tag.
        assert_eq!(captured, format!("8202{ALL_INPUTS_ARE_SPENT}"));

        let bytes = hex::decode(captured).unwrap();
        match minicbor::decode::<Message<EraTx, TxValidationError>>(&bytes).unwrap() {
            Message::RejectTx(TxValidationError::DijkstraTxValidationError { error, era }) => {
                assert_eq!(era, ShelleyBasedEra::Dijkstra);
                assert_eq!(error, ApplyTxError(vec![MempoolFailure::AllInputsAreSpent]));
            }
            other => panic!("expected a Dijkstra RejectTx, got {other:?}"),
        }
    }
}
