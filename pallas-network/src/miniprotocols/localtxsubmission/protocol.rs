use super::Value;
pub use crate::miniprotocols::localstate::queries_v16::{Coin, ExUnits, TaggedSet};
use pallas_codec::minicbor::{self, Decode, Encode};
use pallas_codec::utils::Bytes;
use pallas_primitives::conway::{
    Certificate, GovAction, GovActionId, ProposalProcedure, TransactionOutput, Voter,
    VotingProcedures,
};
use pallas_primitives::{
    AddrKeyhash, PolicyId, ProtocolVersion, ScriptHash, Set, StakeCredential, TransactionInput,
};

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

// https://github.com/IntersectMBO/cardano-api/blob/a0df586e3a14b98ae4771a192c09391dacb44564/cardano-api/internal/Cardano/Api/Eon/ShelleyBasedEra.hs#L271
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ShelleyBasedEra {
    ShelleyBasedEraShelley,
    ShelleyBasedEraAllegra,
    ShelleyBasedEraMary,
    ShelleyBasedEraAlonzo,
    ShelleyBasedEraBabbage,
    ShelleyBasedEraConway,
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
    DisplayRewardAccount,
    Voter,
    ProposalProcedure,
>;

/// Purpose with the corresponding index. It corresponds to
/// [`ConwayPlutusPurpose`](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/eras/conway/impl/src/Cardano/Ledger/Conway/Scripts.hs#L188-L194)
/// in the Haskell sources, where the higher-order argument `f` equals `AsIx`.
pub type PlutusPurposeIx = PlutusPurpose<u64, u64, u64, u64, u64, u64>;

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

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum FailureDescription {
    PlutusFailure(String, Bytes),
}

/// Tag mismatch description for UTXO validation. It corresponds to
/// [TagMismatchDescription](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/eras/alonzo/impl/src/Cardano/Ledger/Alonzo/Rules/Utxos.hs#L367)
/// in the Haskell sources.
///
/// Represents the reasons why a tag mismatch occurred during validation.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TagMismatchDescription {
    PassedUnexpectedly,
    FailedUnexpectedly(Vec<FailureDescription>),
}

/// Errors that can occur when collecting arguments for phase-2 scripts.
/// It corresponds to [CollectError](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/eras/alonzo/impl/src/Cardano/Ledger/Alonzo/Plutus/Evaluate.hs#L78-L83).
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum CollectError {
    NoRedeemer(PlutusPurposeItem),
    NoWitness(DisplayScriptHash),
    NoCostModel(Language),
    BadTranslation(ConwayContextError),
}
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ConwayContextError {
    BabbageContextError(BabbageContextError),
    CertificateNotSupported(ConwayTxCert),
    PlutusPurposeNotSupported(PlutusPurposeItem),
    CurrentTreasuryFieldNotSupported(DisplayCoin),
    VotingProceduresFieldNotSupported(DisplayVotingProcedures),
    ProposalProceduresFieldNotSupported(DisplayOSet<ProposalProcedure>),
    TreasuryDonationFieldNotSupported(DisplayCoin),
}
#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
#[cbor(transparent)]
pub struct DisplayOSet<T>(#[n(0)] pub Set<T>);

#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
#[cbor(transparent)]
pub struct DisplayVotingProcedures(#[n(0)] pub VotingProcedures);

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum BabbageContextError {
    ByronTxOutInContext(TxOutSource),
    AlonzoMissingInput(TransactionInput),
    RedeemerPointerPointsToNothing(PlutusPurposeIx),
    InlineDatumsNotSupported(TxOutSource),
    ReferenceScriptsNotSupported(TxOutSource),
    ReferenceInputsNotSupported(Set<TransactionInput>),
    AlonzoTimeTranslationPastHorizon(String),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TxOutSource {
    TxOutFromInput(TransactionInput),
    TxOutFromOutput(u64),
}

#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
#[cbor(transparent)]
pub struct TxIx(#[n(0)] pub u64);

// this type can be used inside a SMaybe
#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
#[cbor(transparent)]
pub struct DisplayScriptHash(#[n(0)] pub ScriptHash);

#[derive(Encode, Decode, Debug, Clone, Eq, PartialEq)]
#[cbor(transparent)]
pub struct IsValid(#[n(0)] pub bool);

/// Conway Utxo subtransition errors. It corresponds to [ConwayUtxosPredFailure](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/eras/conway/impl/src/Cardano/Ledger/Conway/Rules/Utxos.hs#L74C6-L74C28)
/// in the Haskell sources. Not to be confused with [UtxoFailure].
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum UtxosFailure {
    ValidationTagMismatch(bool, TagMismatchDescription),
    CollectErrors(Array<CollectError>),
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

#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
#[cbor(transparent)]
pub struct DisplayAddress(#[n(0)] pub Bytes);

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Utxo(pub OHashMap<TransactionInput, TransactionOutput>);

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct OHashMap<K, V>(pub Vec<(K, V)>);

#[derive(Encode, Decode, Debug, Clone, Eq, PartialEq)]
#[cbor(index_only)]
pub enum Network {
    #[n(0)]
    Testnet,
    #[n(1)]
    Mainnet,
}

#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
#[cbor(transparent)]
pub struct DeltaCoin(#[n(0)] pub i32);

#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
#[cbor(transparent)]
pub struct DisplayRewardAccount(#[n(0)] pub Bytes);

impl From<&Bytes> for DisplayRewardAccount {
    fn from(bytes: &Bytes) -> Self {
        DisplayRewardAccount(bytes.to_owned())
    }
}

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
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum UtxoFailure {
    UtxosFailure(UtxosFailure),
    BadInputsUTxO(Set<TransactionInput>),
    OutsideValidityIntervalUTxO(ValidityInterval, Slot),
    MaxTxSizeUTxO(i64, i64),
    InputSetEmptyUTxO,
    FeeTooSmallUTxO(DisplayCoin, DisplayCoin),
    ValueNotConservedUTxO(Value, Value),
    WrongNetwork(Network, Set<DisplayAddress>),
    WrongNetworkWithdrawal(Network, Set<DisplayRewardAccount>),
    OutputTooSmallUTxO(Array<TransactionOutput>),
    OutputBootAddrAttrsTooBig(Array<TransactionOutput>),
    OutputTooBigUTxO(Array<(i64, i64, TransactionOutput)>),
    InsufficientCollateral(DeltaCoin, DisplayCoin),
    ScriptsNotPaidUTxO(Utxo),
    ExUnitsTooBigUTxO(ExUnits, ExUnits),
    CollateralContainsNonADA(Value),
    WrongNetworkInTxBody(Network, Network),
    OutsideForecast(Slot),
    TooManyCollateralInputs(u16, u16),
    NoCollateralInputs,
    IncorrectTotalCollateralField(DeltaCoin, DisplayCoin),
    BabbageOutputTooSmallUTxO(Array<(TransactionOutput, DisplayCoin)>),
    BabbageNonDisjointRefInputs(Vec<TransactionInput>),
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
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ConwayUtxoWPredFailure {
    UtxoFailure(UtxoFailure),
    InvalidWitnessesUTXOW(Array<VKey>),
    MissingVKeyWitnessesUTXOW(Set<KeyHash>),
    MissingScriptWitnessesUTXOW(Set<ScriptHash>),
    ScriptWitnessNotValidatingUTXOW(Set<ScriptHash>),
    MissingTxBodyMetadataHash(Bytes),
    MissingTxMetadata(Bytes),
    ConflictingMetadataHash(Bytes, Bytes),
    InvalidMetadata(),
    ExtraneousScriptWitnessesUTXOW(Set<ScriptHash>),
    MissingRedeemers(Array<(PlutusPurposeItem, ScriptHash)>),
    MissingRequiredDatums(Set<SafeHash>, Set<SafeHash>),
    NotAllowedSupplementalDatums(Set<SafeHash>, Set<SafeHash>),
    PPViewHashesDontMatch(SMaybe<SafeHash>, SMaybe<SafeHash>),
    UnspendableUTxONoDatumHash(Set<TransactionInput>),
    ExtraRedeemers(Array<PlutusPurposeIx>),
    MalformedScriptWitnesses(Set<ScriptHash>),
    MalformedReferenceScripts(Set<ScriptHash>),
}
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ConwayPlutusPurpose {
    ConwaySpending(AsItem<TransactionInput>),
    ConwayMinting(AsItem<DisplayPolicyId>),
    ConwayCertifying(AsItem<ConwayTxCert>),
    ConwayRewarding(AsItem<DisplayRewardAccount>),
    ConwayVoting(AsItem<Voter>),
    ConwayProposing(AsItem<ProposalProcedure>),
}
#[derive(Debug, Decode, Encode, Hash, PartialEq, Eq, Clone)]
#[cbor(transparent)]
pub struct SafeHash(#[n(0)] pub Bytes);
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ConwayTxCert {
    ConwayTxCertDeleg(Certificate),
    ConwayTxCertPool(Certificate),
    ConwayTxCertGov(Certificate),
}
#[derive(Debug, Decode, Hash, PartialEq, Eq, Clone)]
#[cbor(transparent)]
pub struct DisplayPolicyId(#[n(0)] pub PolicyId);

// https://github.com/IntersectMBO/cardano-ledger/blob/33e90ea03447b44a389985ca2b158568e5f4ad65/libs/cardano-ledger-core/src/Cardano/Ledger/Credential.hs#L82
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Credential {
    ScriptHashObj(ScriptHash),
    KeyHashObj(AddrKeyhash),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Mismatch<T>(pub T, pub T);

#[derive(Debug, Decode, Clone, Eq, PartialEq)]
#[cbor(transparent)]
pub struct AsItem<T>(#[n(0)] pub T);

#[derive(Debug, Decode, Encode, Clone, Eq, PartialEq)]
#[cbor(transparent)]
pub struct EpochNo(#[n(0)] pub u64);

#[derive(Debug, Decode, Clone, Eq, PartialEq)]
#[cbor(transparent)]
pub struct AsIx(#[n(0)] pub u64);
/// Conway era ledger transaction errors, corresponding to [`ConwayLedgerPredFailure`](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/eras/conway/impl/src/Cardano/Ledger/Conway/Rules/Ledger.hs#L138-L153)
/// in the Haskell sources.
///
/// The `u8` variant appears for backward compatibility.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ApplyConwayTxPredError {
    ConwayUtxowFailure(ConwayUtxoWPredFailure),
    ConwayCertsFailure(ConwayCertsPredFailure),
    ConwayGovFailure(ConwayGovPredFailure),
    ConwayWdrlNotDelegatedToDRep(Vec<KeyHash>),
    ConwayTreasuryValueMismatch(DisplayCoin, DisplayCoin),
    ConwayTxRefScriptsSizeTooBig(i64, i64),
    ConwayMempoolFailure(String),
    U8(u8),
}
// https://github.com/IntersectMBO/cardano-ledger/blob/33e90ea03447b44a389985ca2b158568e5f4ad65/eras/conway/impl/src/Cardano/Ledger/Conway/Rules/Certs.hs#L113
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ConwayCertsPredFailure {
    WithdrawalsNotInRewardsCERTS(OHashMap<DisplayRewardAccount, DisplayCoin>),
    CertFailure(ConwayCertPredFailure),
}

// https://github.com/IntersectMBO/cardano-ledger/blob/33e90ea03447b44a389985ca2b158568e5f4ad65/eras/conway/impl/src/Cardano/Ledger/Conway/Rules/Cert.hs#L102
#[derive(Debug, Encode, Clone, Eq, PartialEq)]
pub enum ConwayCertPredFailure {
    #[n(0)]
    DelegFailure(#[n(0)] ConwayDelegPredFailure),
    #[n(1)]
    PoolFailure(#[n(0)] ShelleyPoolPredFailure),
    #[n(2)]
    GovCertFailure(#[n(0)] ConwayGovCertPredFailure),
}

#[derive(Debug, Encode, Clone, Eq, PartialEq)]
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
#[derive(Debug, Encode, Clone, Eq, PartialEq)]
pub enum ConwayGovCertPredFailure {
    #[n(0)]
    ConwayDRepAlreadyRegistered(#[n(0)] Credential),
    #[n(1)]
    ConwayDRepNotRegistered(#[n(0)] Credential),
    #[n(2)]
    ConwayDRepIncorrectDeposit(#[n(0)] DisplayCoin, #[n(1)] DisplayCoin),
    #[n(3)]
    ConwayCommitteeHasPreviouslyResigned(#[n(0)] Credential),
    #[n(4)]
    ConwayDRepIncorrectRefund(#[n(0)] DisplayCoin, #[n(1)] DisplayCoin),
    #[n(5)]
    ConwayCommitteeIsUnknown(#[n(0)] Credential),
}

// https://github.com/IntersectMBO/cardano-ledger/blob/b14ba8190e21ced6cc68c18a02dd1dbc2ff45a3c/eras/conway/impl/src/Cardano/Ledger/Conway/Rules/Deleg.hs#L104
#[derive(Debug, Encode, Clone, Eq, PartialEq)]
pub enum ConwayDelegPredFailure {
    #[n(0)]
    IncorrectDepositDELEG(#[n(0)] DisplayCoin),
    #[n(1)]
    StakeKeyRegisteredDELEG(#[n(0)] Credential),
    #[n(2)]
    StakeKeyNotRegisteredDELEG(#[n(0)] Credential),
    #[n(3)]
    StakeKeyHasNonZeroRewardAccountBalanceDELEG(#[n(0)] DisplayCoin),
    #[n(4)]
    DelegateeDRepNotRegisteredDELEG(#[n(0)] Credential),
    #[n(5)]
    DelegateeStakePoolNotRegisteredDELEG(#[n(0)] KeyHash),
}

// https://github.com/IntersectMBO/cardano-ledger/blob/33e90ea03447b44a389985ca2b158568e5f4ad65/eras/conway/impl/src/Cardano/Ledger/Conway/Rules/Gov.hs#L164
#[derive(Debug, Encode, Clone, Eq, PartialEq)]
pub enum ConwayGovPredFailure {
    #[n(0)]
    GovActionsDoNotExist(#[n(0)] Vec<GovActionId>),
    #[n(1)]
    MalformedProposal(#[n(0)] GovAction),
    #[n(2)]
    ProposalProcedureNetworkIdMismatch(#[n(0)] DisplayRewardAccount, #[n(1)] Network),
    #[n(3)]
    TreasuryWithdrawalsNetworkIdMismatch(#[n(0)] Set<DisplayRewardAccount>, #[n(1)] Network),
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
    ProposalReturnAccountDoesNotExist(#[n(0)] DisplayRewardAccount),
    #[n(17)]
    TreasuryWithdrawalReturnAccountsDoNotExist(#[n(0)] Vec<DisplayRewardAccount>),
}

/// Reject reason. It can be a pair of an era number and a sequence of errors, or else a string.
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
pub struct ApplyTxError(pub Vec<ApplyConwayTxPredError>);
