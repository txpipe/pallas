use std::{
    collections::{BTreeSet, HashMap},
    net::Ipv6Addr,
    ops::Deref,
};

use pallas_addresses::{
    byron::{AddrAttrProperty, AddrType, AddressPayload},
    ByronAddress, Pointer, ShelleyAddress, ShelleyDelegationPart, ShelleyPaymentPart,
};
use pallas_codec::{
    minicbor::bytes::ByteVec,
    utils::{
        CborWrap, Int, MaybeIndefArray, NonEmptyKeyValuePairs, Nullable, OrderPreservingProperties,
        PositiveCoin, Set, TagWrap,
    },
};
use pallas_crypto::hash::{Hash, Hasher};
use pallas_network::miniprotocols::{
    handshake::NetworkMagic,
    localstate::queries_v16::{
        primitives::Bytes, Anchor, AssetName, BoundedBytes, Constitution, CostModel, CostModels,
        DRep, DRepVotingThresholds, DatumHash, DatumOption, ExUnitPrices, FieldedRewardAccount,
        GovAction, GovActionId, PParamsUpdate, PlutusData, PolicyId, PoolMetadata,
        PoolVotingThresholds, ProposalProcedure, ProtocolVersion, RationalNumber, Relay,
        ScriptHash, TransactionInput, TransactionOutput, Value, Vote,
    },
    localtxsubmission::{
        primitives::{
            Certificate, CommitteeColdCredential, Credential, KeyValuePairs, Language, Multiasset,
            NativeScript, PoolKeyhash, PseudoScript, StakeCredential, Voter,
        },
        Coin, ConwayCertsPredFailure, ExUnits, Network, VotingProcedure,
    },
};

use pallas_network::miniprotocols::localtxsubmission::{
    Array, BabbageContextError, CollectError, ConwayCertPredFailure, ConwayContextError,
    ConwayDelegPredFailure, ConwayGovCertPredFailure, ConwayGovPredFailure, ConwayLedgerFailure,
    ConwayTxCert, ConwayUtxoWPredFailure, DeltaCoin, DisplayAddress, DisplayCoin, DisplayOSet,
    DisplayPolicyId, DisplayScriptHash, DisplayVotingProcedures, EpochNo, FailureDescription,
    KeyHash, Mismatch, OHashMap, PlutusPurpose, SMaybe, SafeHash, ShelleyPoolPredFailure, SlotNo,
    TagMismatchDescription, TxOutSource, Utxo, UtxoFailure, UtxosFailure, VKey, ValidityInterval,
};

use super::haskells_show_string::haskell_show_string;

/// Trait used to generated Haskell-like string representation of a type.
pub trait HaskellDisplay {
    /// Haskell representation of the type.
    fn to_haskell_str(&self) -> String;
    /// Haskell representation of the type, wrapped in parentheses.
    fn to_haskell_str_p(&self) -> String {
        format!("({})", self.to_haskell_str())
    }
    /// Haskell representation of the type, wrapped in extra parentheses.
    fn to_haskell_str_pp(&self) -> String {
        format!("(({}))", self.to_haskell_str())
    }
}

impl HaskellDisplay for ConwayLedgerFailure {
    fn to_haskell_str(&self) -> String {
        use ConwayLedgerFailure::*;

        match self {
            UtxowFailure(e) => format!("ConwayUtxowFailure {}", e.to_haskell_str()),
            CertsFailure(e) => format!("ConwayCertsFailure {}", e.to_haskell_str_p()),
            GovFailure(e) => format!("ConwayGovFailure {}", e.to_haskell_str_p()),
            WdrlNotDelegatedToDRep(v) => {
                format!("ConwayWdrlNotDelegatedToDRep {}", v.to_haskell_str_p())
            }
            TreasuryValueMismatch(c1, c2) => {
                format!(
                    "ConwayTreasuryValueMismatch {} {}",
                    c1.to_haskell_str_p(),
                    c2.to_haskell_str_p()
                )
            }
            TxRefScriptsSizeTooBig(s1, s2) => {
                format!(
                    "ConwayTxRefScriptsSizeTooBig {} {}",
                    s1.to_haskell_str_p(),
                    s2.to_haskell_str_p()
                )
            }
            MempoolFailure(e) => format!("ConwayMempoolFailure {}", e.to_haskell_str()),
            U8(v) => format!("U8 {v}"),
        }
    }
}

impl HaskellDisplay for ConwayGovCertPredFailure {
    fn to_haskell_str(&self) -> String {
        use ConwayGovCertPredFailure::*;

        match self {
            DRepAlreadyRegistered(cred) => {
                format!("ConwayDRepAlreadyRegistered {}", cred.to_haskell_str_p())
            }
            DRepNotRegistered(cred) => {
                format!("ConwayDRepNotRegistered {}", cred.to_haskell_str_p())
            }
            DRepIncorrectDeposit(expected, actual) => format!(
                "ConwayDRepIncorrectDeposit {} {}",
                expected.to_haskell_str_p(),
                actual.to_haskell_str_p()
            ),
            CommitteeHasPreviouslyResigned(cred) => {
                format!(
                    "ConwayCommitteeHasPreviouslyResigned {}",
                    cred.to_haskell_str_p()
                )
            }
            DRepIncorrectRefund(expected, actual) => format!(
                "ConwayDRepIncorrectRefund {} {}",
                expected.to_haskell_str_p(),
                actual.to_haskell_str_p()
            ),
            CommitteeIsUnknown(cred) => {
                format!("ConwayCommitteeIsUnknown {}", cred.to_haskell_str_p())
            }
        }
    }
}

impl HaskellDisplay for ConwayCertPredFailure {
    fn to_haskell_str(&self) -> String {
        use ConwayCertPredFailure::*;
        match self {
            DelegFailure(e) => format!("DelegFailure {}", e.to_haskell_str_p()),
            PoolFailure(e) => format!("PoolFailure {}", e.to_haskell_str_p()),
            GovCertFailure(e) => format!("GovCertFailure {}", e.to_haskell_str_p()),
        }
    }
}

impl HaskellDisplay for ConwayCertsPredFailure {
    fn to_haskell_str(&self) -> String {
        use ConwayCertsPredFailure::*;

        match self {
            WithdrawalsNotInRewardsCERTS(m) => {
                format!("WithdrawalsNotInRewardsCERTS {}", m.to_haskell_str_p())
            }
            CertFailure(e) => format!("CertFailure {}", e.to_haskell_str_p()),
        }
    }
}

impl HaskellDisplay for ShelleyPoolPredFailure {
    fn to_haskell_str(&self) -> String {
        use ShelleyPoolPredFailure::*;
        match self {
            StakePoolNotRegisteredOnKeyPOOL(kh) => {
                format!("StakePoolNotRegisteredOnKeyPOOL {}", kh.to_haskell_str_p())
            }
            StakePoolRetirementWrongEpochPOOL(mis1, mis2) => {
                format!(
                    "StakePoolRetirementWrongEpochPOOL {} {}",
                    mis1.to_haskell_str_p(),
                    mis2.to_haskell_str_p()
                )
            }
            StakePoolCostTooLowPOOL(mis1) => {
                format!("StakePoolCostTooLowPOOL {}", mis1.to_haskell_str_p())
            }
            WrongNetworkPOOL(mis1, kh) => {
                format!(
                    "WrongNetworkPOOL {} {}",
                    mis1.to_haskell_str_p(),
                    kh.to_haskell_str_p()
                )
            }
            PoolMedataHashTooBig(kh, size) => {
                format!(
                    "PoolMedataHashTooBig {} {}",
                    kh.to_haskell_str_p(),
                    size.to_haskell_str_p()
                )
            }
        }
    }
}

impl HaskellDisplay for ConwayUtxoWPredFailure {
    fn to_haskell_str(&self) -> String {
        use ConwayUtxoWPredFailure::*;

        match self {
            UtxoFailure(e) => format!("(UtxoFailure {})", e.to_haskell_str()),
            InvalidWitnessesUTXOW(e) => format!("(InvalidWitnessesUTXOW {})", e.to_haskell_str()),
            MissingVKeyWitnessesUTXOW(e) => {
                format!("(MissingVKeyWitnessesUTXOW {})", e.to_haskell_str_p())
            }
            MissingScriptWitnessesUTXOW(e) => {
                format!("(MissingScriptWitnessesUTXOW {})", e.to_haskell_str_p())
            }
            ScriptWitnessNotValidatingUTXOW(e) => {
                format!("(ScriptWitnessNotValidatingUTXOW {})", e.to_haskell_str_p())
            }
            MissingTxBodyMetadataHash(b) => {
                format!("(MissingTxBodyMetadataHash ({}))", b.as_aux_data_hash())
            }
            MissingTxMetadata(e) => format!("(MissingTxMetadata ({}))", e.as_aux_data_hash()),
            ConflictingMetadataHash(e1, e2) => format!(
                "(ConflictingMetadataHash ({}) ({}))",
                e1.as_aux_data_hash(),
                e2.as_aux_data_hash()
            ),
            InvalidMetadata() => "InvalidMetadata".to_string(),
            ExtraneousScriptWitnessesUTXOW(vec) => {
                format!(
                    "(ExtraneousScriptWitnessesUTXOW {})",
                    vec.to_haskell_str_p()
                )
            }
            MissingRedeemers(e) => format!("(MissingRedeemers {})", e.to_haskell_str()),
            MissingRequiredDatums(e1, e2) => format!(
                "(MissingRequiredDatums {} {})",
                e1.to_haskell_str_p(),
                e2.to_haskell_str_p()
            ),
            NotAllowedSupplementalDatums(e1, e2) => format!(
                "(NotAllowedSupplementalDatums {} {})",
                e1.to_haskell_str_p(),
                e2.to_haskell_str_p()
            ),
            PPViewHashesDontMatch(h1, h2) => format!(
                "(PPViewHashesDontMatch {} {})",
                h1.to_haskell_str_p(),
                h2.to_haskell_str_p()
            ),
            UnspendableUTxONoDatumHash(e) => {
                format!("(UnspendableUTxONoDatumHash {})", e.to_haskell_str_p())
            }
            ExtraRedeemers(e) => format!("(ExtraRedeemers {})", e.to_haskell_str()),
            MalformedScriptWitnesses(set) => {
                format!("(MalformedScriptWitnesses {})", set.to_haskell_str_p())
            }
            MalformedReferenceScripts(set) => {
                format!("(MalformedReferenceScripts {})", set.to_haskell_str_p())
            }
        }
    }
}

impl HaskellDisplay for ConwayGovPredFailure {
    fn to_haskell_str(&self) -> String {
        use ConwayGovPredFailure::*;
        match self {
            GovActionsDoNotExist(vec) => {
                format!("GovActionsDoNotExist {}", vec.to_haskell_str_p())
            }
            MalformedProposal(act) => {
                format!("MalformedProposal {}", act.to_haskell_str_p())
            }
            ProposalProcedureNetworkIdMismatch(ra, n) => {
                format!(
                    "ProposalProcedureNetworkIdMismatch {} {}",
                    ra.to_haskell_str_p(),
                    n.to_haskell_str()
                )
            }
            TreasuryWithdrawalsNetworkIdMismatch(set, n) => {
                format!(
                    "TreasuryWithdrawalsNetworkIdMismatch {} {}",
                    set.to_haskell_str_p(),
                    n.to_haskell_str()
                )
            }
            ProposalDepositIncorrect(c1, c2) => {
                format!(
                    "ProposalDepositIncorrect {} {}",
                    c1.to_haskell_str_p(),
                    c2.to_haskell_str_p()
                )
            }
            DisallowedVoters(v) => {
                format!("DisallowedVoters {}", v.to_haskell_str_p())
            }
            ConflictingCommitteeUpdate(set) => {
                format!("ConflictingCommitteeUpdate {}", set.to_haskell_str_p())
            }
            ExpirationEpochTooSmall(map) => {
                format!("ExpirationEpochTooSmall {}", map.to_haskell_str_p())
            }
            InvalidPrevGovActionId(s) => {
                format!("InvalidPrevGovActionId {}", s.to_haskell_str_p())
            }
            VotingOnExpiredGovAction(vec) => {
                format!("VotingOnExpiredGovAction {}", vec.to_haskell_str_p())
            }
            ProposalCantFollow(a, p1, p2) => {
                format!(
                    "ProposalCantFollow {} ({}) ({})",
                    a.to_haskell_str_p(),
                    p1.as_protocol_version(),
                    p2.as_protocol_version()
                )
            }
            InvalidPolicyHash(maybe1, maybe2) => {
                format!(
                    "InvalidPolicyHash {} {}",
                    maybe1.to_haskell_str_p(),
                    maybe2.to_haskell_str_p()
                )
            }
            DisallowedProposalDuringBootstrap(s) => {
                format!("DisallowedProposalDuringBootstrap {}", s.to_haskell_str_p())
            }
            DisallowedVotesDuringBootstrap(v) => {
                format!("DisallowedVotesDuringBootstrap {}", v.to_haskell_str_p())
            }
            VotersDoNotExist(s) => {
                format!("VotersDoNotExist {}", s.to_haskell_str_p())
            }
            ZeroTreasuryWithdrawals(s) => {
                format!("ZeroTreasuryWithdrawals {}", s.to_haskell_str_p())
            }
            ProposalReturnAccountDoesNotExist(s) => {
                format!("ProposalReturnAccountDoesNotExist {}", s.to_haskell_str_p())
            }
            TreasuryWithdrawalReturnAccountsDoNotExist(s) => {
                format!(
                    "TreasuryWithdrawalReturnAccountsDoNotExist {}",
                    s.to_haskell_str_p()
                )
            }
        }
    }
}

impl HaskellDisplay for UtxoFailure {
    fn to_haskell_str(&self) -> String {
        use UtxoFailure::*;

        match self {
            UtxosFailure(utxos_failure) => {
                format!("(UtxosFailure {})", utxos_failure.to_haskell_str_p())
            }
            BadInputsUTxO(inputs) => format!("(BadInputsUTxO {})", inputs.to_haskell_str_p()),
            OutsideValidityIntervalUTxO(interval, slot) => format!(
                "(OutsideValidityIntervalUTxO {} ({}))",
                interval.to_haskell_str(),
                slot.as_slot_no()
            ),
            MaxTxSizeUTxO(actual, max) => format!(
                "(MaxTxSizeUTxO {} {})",
                actual.to_haskell_str_p(),
                max.to_haskell_str_p()
            ),
            InputSetEmptyUTxO => "InputSetEmptyUTxO".to_string(),
            FeeTooSmallUTxO(required, provided) => format!(
                "(FeeTooSmallUTxO {} {})",
                required.to_haskell_str_p(),
                provided.to_haskell_str_p()
            ),
            ValueNotConservedUTxO(required, provided) => format!(
                "(ValueNotConservedUTxO {} {})",
                required.to_haskell_str_p(),
                provided.to_haskell_str_p()
            ),
            OutputTooSmallUTxO(outputs) => {
                format!("(OutputTooSmallUTxO {})", outputs.to_haskell_str_p())
            }
            OutputBootAddrAttrsTooBig(outputs) => {
                format!("(OutputBootAddrAttrsTooBig {})", outputs.to_haskell_str_p())
            }
            InsufficientCollateral(balance, required) => format!(
                "(InsufficientCollateral {} {})",
                balance.to_haskell_str_p(),
                required.to_haskell_str_p()
            ),
            ScriptsNotPaidUTxO(utxo) => format!("(ScriptsNotPaidUTxO {})", utxo.to_haskell_str_p()),
            ExUnitsTooBigUTxO(provided, max) => format!(
                "(ExUnitsTooBigUTxO {} {})",
                provided.to_haskell_str_p(),
                max.to_haskell_str_p()
            ),
            WrongNetwork(network, addrs) => format!(
                "(WrongNetwork {} {})",
                network.to_haskell_str(),
                addrs.to_haskell_str_p()
            ),
            WrongNetworkWithdrawal(network, accounts) => format!(
                "(WrongNetworkWithdrawal {} {})",
                network.to_haskell_str(),
                accounts.to_haskell_str_p()
            ),
            OutsideForecast(slot) => format!("(OutsideForecast ({}))", slot.as_slot_no()),
            CollateralContainsNonADA(value) => {
                format!("(CollateralContainsNonADA {})", value.to_haskell_str_p())
            }
            NoCollateralInputs => "NoCollateralInputs".to_string(),
            TooManyCollateralInputs(actual, max) => {
                format!("(TooManyCollateralInputs {actual} {max})")
            }
            WrongNetworkInTxBody(expected, actual) => format!(
                "(WrongNetworkInTxBody {} {})",
                expected.to_haskell_str(),
                actual.to_haskell_str()
            ),
            IncorrectTotalCollateralField(actual, provided) => format!(
                "(IncorrectTotalCollateralField {} {})",
                actual.to_haskell_str_p(),
                provided.to_haskell_str_p()
            ),
            OutputTooBigUTxO(outputs) => format!("(OutputTooBigUTxO {})", outputs.to_haskell_str()),
            BabbageOutputTooSmallUTxO(outputs) => {
                format!("(BabbageOutputTooSmallUTxO {})", outputs.to_haskell_str_p())
            }
            BabbageNonDisjointRefInputs(inputs) => format!(
                "(BabbageNonDisjointRefInputs {})",
                inputs.to_haskell_str_p()
            ),
        }
    }
}

impl HaskellDisplay for UtxosFailure {
    fn to_haskell_str(&self) -> String {
        use UtxosFailure::*;

        match self {
            ValidationTagMismatch(is_valid, desc) => format!(
                "ValidationTagMismatch ({}) {}",
                is_valid.as_is_valid(),
                desc.to_haskell_str_p()
            ),
            CollectErrors(errors) => format!("CollectErrors {}", errors.to_haskell_str()),
        }
    }
}

impl HaskellDisplay for CollectError {
    fn to_haskell_str(&self) -> String {
        match self {
            CollectError::NoRedeemer(conway_plutus_purpose) => {
                format!("NoRedeemer {}", conway_plutus_purpose.to_haskell_str_p())
            }
            CollectError::NoWitness(display_script_hash) => {
                format!("NoWitness {}", display_script_hash.to_haskell_str_p())
            }
            CollectError::NoCostModel(language) => {
                format!("NoCostModel {}", language.to_haskell_str())
            }
            CollectError::BadTranslation(error) => {
                format!("BadTranslation ({})", error.to_haskell_str())
            }
        }
    }
}

impl HaskellDisplay for ConwayContextError {
    fn to_haskell_str(&self) -> String {
        use ConwayContextError::*;

        match self {
            BabbageContextError(babbage_context_error) => format!(
                "BabbageContextError ({})",
                babbage_context_error.to_haskell_str()
            ),
            CertificateNotSupported(conway_tx_cert) => format!(
                "CertificateNotSupported ({})",
                conway_tx_cert.to_haskell_str()
            ),
            PlutusPurposeNotSupported(conway_plutus_purpose) => format!(
                "PlutusPurposeNotSupported ({})",
                conway_plutus_purpose.to_haskell_str()
            ),
            CurrentTreasuryFieldNotSupported(display_coin) => format!(
                "CurrentTreasuryFieldNotSupported ({})",
                display_coin.to_haskell_str()
            ),
            VotingProceduresFieldNotSupported(vp) => format!(
                "VotingProceduresFieldNotSupported ({})",
                vp.to_haskell_str()
            ),
            ProposalProceduresFieldNotSupported(proposal_procedures) => format!(
                "ProposalProceduresFieldNotSupported ({})",
                proposal_procedures.to_haskell_str()
            ),
            TreasuryDonationFieldNotSupported(display_coin) => format!(
                "TreasuryDonationFieldNotSupported ({})",
                display_coin.to_haskell_str()
            ),
        }
        .to_string()
    }
}

impl HaskellDisplay for BabbageContextError {
    fn to_haskell_str(&self) -> String {
        use BabbageContextError::*;
        match self {
            ByronTxOutInContext(tx_out) => {
                format!("ByronTxOutInContext ({})", tx_out.to_haskell_str())
            }
            AlonzoMissingInput(tx_in) => format!(
                "AlonzoContextError (TranslationLogicMissingInput {})",
                tx_in.to_haskell_str_p()
            ),
            RedeemerPointerPointsToNothing(ptr) => {
                format!("RedeemerPointerPointsToNothing ({})", ptr.to_haskell_str())
            }
            InlineDatumsNotSupported(datum) => {
                format!("InlineDatumsNotSupported ({})", datum.to_haskell_str())
            }
            ReferenceScriptsNotSupported(script) => {
                format!("ReferenceScriptsNotSupported ({})", script.to_haskell_str())
            }
            ReferenceInputsNotSupported(input) => {
                format!("ReferenceInputsNotSupported ({})", input.to_haskell_str())
            }
            AlonzoTimeTranslationPastHorizon(time) => format!(
                "AlonzoContextError (TimeTranslationPastHorizon {})",
                time.to_haskell_str()
            ),
        }
    }
}

impl HaskellDisplay for TxOutSource {
    fn to_haskell_str(&self) -> String {
        use TxOutSource::*;
        match self {
            Input(tx) => format!("TxOutFromInput {}", tx.to_haskell_str_p()),
            Output(tx) => format!("TxOutFromOutput ({})", tx.as_tx_ix()),
        }
    }
}

impl HaskellDisplay for Language {
    fn to_haskell_str(&self) -> String {
        use Language::*;
        match self {
            PlutusV1 => "PlutusV1".to_string(),
            PlutusV2 => "PlutusV2".to_string(),
            PlutusV3 => "PlutusV3".to_string(),
        }
    }
}

impl HaskellDisplay for TagMismatchDescription {
    fn to_haskell_str(&self) -> String {
        use TagMismatchDescription::*;
        match self {
            PassedUnexpectedly => "PassedUnexpectedly".to_string(),
            FailedUnexpectedly(desc) => format!("FailedUnexpectedly {}", desc.to_haskell_str_p()),
        }
    }
    fn to_haskell_str_p(&self) -> String {
        use TagMismatchDescription::*;
        match self {
            PassedUnexpectedly => "PassedUnexpectedly".to_string(),
            FailedUnexpectedly(desc) => format!("(FailedUnexpectedly {})", desc.to_haskell_str_p()),
        }
    }
}

impl HaskellDisplay for FailureDescription {
    fn to_haskell_str(&self) -> String {
        use FailureDescription::*;
        match self {
            PlutusFailure(s, b) => format!(
                "PlutusFailure {} {}",
                s.to_haskell_str(),
                b.to_haskell_str()
            ),
        }
    }
}

impl HaskellDisplay for ConwayDelegPredFailure {
    fn to_haskell_str(&self) -> String {
        use ConwayDelegPredFailure::*;

        match self {
            IncorrectDepositDELEG(coin) => {
                format!("IncorrectDepositDELEG ({})", coin.to_haskell_str())
            }
            StakeKeyRegisteredDELEG(cred) => {
                format!("StakeKeyRegisteredDELEG ({})", cred.to_haskell_str())
            }
            StakeKeyNotRegisteredDELEG(cred) => {
                format!("StakeKeyNotRegisteredDELEG ({})", cred.to_haskell_str())
            }
            StakeKeyHasNonZeroRewardAccountBalanceDELEG(coin) => format!(
                "StakeKeyHasNonZeroRewardAccountBalanceDELEG ({})",
                coin.to_haskell_str()
            ),
            DelegateeDRepNotRegisteredDELEG(cred) => format!(
                "DelegateeDRepNotRegisteredDELEG ({})",
                cred.to_haskell_str()
            ),
            DelegateeStakePoolNotRegisteredDELEG(hash) => format!(
                "DelegateeStakePoolNotRegisteredDELEG ({})",
                hash.to_haskell_str()
            ),
        }
    }
}

impl HaskellDisplay for TransactionInput {
    fn to_haskell_str(&self) -> String {
        format!(
            "TxIn ({}) ({})",
            self.transaction_id.as_tx_id(),
            self.index.as_tx_ix()
        )
    }
}

impl<T> HaskellDisplay for Mismatch<T>
where
    T: HaskellDisplay,
{
    fn to_haskell_str(&self) -> String {
        format!(
            "Mismatch {{mismatchSupplied = {}, mismatchExpected = {}}}",
            self.0.to_haskell_str(),
            self.1.to_haskell_str()
        )
    }
}

impl HaskellDisplay for FieldedRewardAccount {
    fn to_haskell_str(&self) -> String {
        format!(
            "RewardAccount {{raNetwork = {}, raCredential = {}}}",
            self.network.to_haskell_str(),
            self.stake_credential.to_haskell_str()
        )
    }
}

impl HaskellDisplay for KeyHash {
    fn to_haskell_str(&self) -> String {
        format!("KeyHash {{unKeyHash = \"{}\"}}", self.0)
    }
}

impl HaskellDisplay for SafeHash {
    fn to_haskell_str(&self) -> String {
        self.0.as_safe_hash()
    }
}

impl HaskellDisplay for GovActionId {
    fn to_haskell_str(&self) -> String {
        format!(
            "GovActionId {{gaidTxId = {}, gaidGovActionIx = {}}}",
            self.tx_id.as_tx_id(),
            display_governance_action_id_index(&self.gov_action_ix)
        )
    }
}

impl HaskellDisplay for ValidityInterval {
    fn to_haskell_str(&self) -> String {
        format!(
            "(ValidityInterval {{invalidBefore = {}, invalidHereafter = {}}})",
            &self.invalid_before.as_slot_no(),
            &self.invalid_hereafter.as_slot_no()
        )
    }
}

impl<T> HaskellDisplay for Nullable<T>
where
    T: HaskellDisplay + std::clone::Clone + 'static,
{
    fn to_haskell_str(&self) -> String {
        match self {
            Nullable::Some(v) => format!("SJust {}", v.to_haskell_str_p()),

            _ => "SNothing".to_string(),
        }
    }

    fn to_haskell_str_p(&self) -> String {
        match self {
            Nullable::Some(v) => {
                if is_primitive::<T>() {
                    format!("SJust {}", v.to_haskell_str())
                } else {
                    format!("(SJust {})", v.to_haskell_str_p())
                }
            }
            _ => "SNothing".to_string(),
        }
    }
}

impl<T> HaskellDisplay for Option<T>
where
    T: HaskellDisplay + 'static,
{
    fn to_haskell_str(&self) -> String {
        match self {
            Option::Some(v) => {
                format!("SJust {}", v.to_haskell_str())
            }
            _ => "SNothing".to_string(),
        }
    }

    fn to_haskell_str_p(&self) -> String {
        match self {
            Option::Some(v) => {
                if is_primitive::<T>() {
                    format!("SJust {}", v.to_haskell_str())
                } else {
                    format!("SJust ({})", v.to_haskell_str())
                }
            }
            _ => "SNothing".to_string(),
        }
    }

    fn to_haskell_str_pp(&self) -> String {
        match self {
            Option::Some(v) => {
                if is_primitive::<T>() {
                    format!("SJust ({})", v.to_haskell_str())
                } else {
                    format!("(SJust ({}))", v.to_haskell_str())
                }
            }
            _ => "SNothing".to_string(),
        }
    }
}

fn is_primitive<T: 'static>() -> bool {
    std::any::TypeId::of::<T>() == std::any::TypeId::of::<bool>()
        || std::any::TypeId::of::<T>() == std::any::TypeId::of::<char>()
        || std::any::TypeId::of::<T>() == std::any::TypeId::of::<u8>()
        || std::any::TypeId::of::<T>() == std::any::TypeId::of::<u16>()
        || std::any::TypeId::of::<T>() == std::any::TypeId::of::<u32>()
        || std::any::TypeId::of::<T>() == std::any::TypeId::of::<u64>()
        || std::any::TypeId::of::<T>() == std::any::TypeId::of::<i8>()
        || std::any::TypeId::of::<T>() == std::any::TypeId::of::<i16>()
        || std::any::TypeId::of::<T>() == std::any::TypeId::of::<i32>()
        || std::any::TypeId::of::<T>() == std::any::TypeId::of::<i64>()
        || std::any::TypeId::of::<T>() == std::any::TypeId::of::<f32>()
        || std::any::TypeId::of::<T>() == std::any::TypeId::of::<f64>()
        || std::any::TypeId::of::<T>() == std::any::TypeId::of::<String>()
}
impl HaskellDisplay for DRep {
    fn to_haskell_str(&self) -> String {
        use DRep::*;
        match self {
            KeyHash(hash) => format!("DRepKeyHash ({})", hash.as_key_hash()),
            ScriptHash(hash) => format!("DRepScriptHash ({})", hash.as_script_hash()),
            AlwaysAbstain => "DRepAlwaysAbstain".to_string(),
            AlwaysNoConfidence => "DRepAlwaysNoConfidence".to_string(),
        }
    }

    fn to_haskell_str_p(&self) -> String {
        use DRep::*;

        match self {
            KeyHash(hash) => format!("(DRepKeyHash ({}))", hash.as_key_hash()),
            ScriptHash(hash) => format!("(DRepScriptHash ({}))", hash.as_script_hash()),
            AlwaysAbstain => "DRepAlwaysAbstain".to_string(),
            AlwaysNoConfidence => "DRepAlwaysNoConfidence".to_string(),
        }
    }
}

impl HaskellDisplay for GovAction {
    fn to_haskell_str(&self) -> String {
        use GovAction::*;

        match self {
            ParameterChange(a, b, c) => {
                format!(
                    "ParameterChange {} {} {}",
                    a.to_haskell_str_pp(),
                    b.to_haskell_str_p(),
                    c.to_haskell_str_pp(),
                )
            }
            HardForkInitiation(a, b) => {
                format!(
                    "HardForkInitiation {} ({})",
                    a.to_haskell_str_pp(),
                    b.as_protocol_version()
                )
            }
            TreasuryWithdrawals(a, b) => {
                let data: KeyValuePairs<FieldedRewardAccount, DisplayCoin> =
                    a.iter().map(|(k, v)| (k.clone(), v.into())).collect();

                format!(
                    "TreasuryWithdrawals {} {}",
                    data.to_haskell_str_p(),
                    b.to_haskell_str_pp(),
                )
            }
            NoConfidence(a) => {
                format!("NoConfidence {}", a.to_haskell_str_pp())
            }
            UpdateCommittee(a, b, kv, d) => {
                let kv: KeyValuePairs<CommitteeColdCredential, EpochNo> = kv
                    .iter()
                    .map(|(k, v)| (k.to_owned(), EpochNo(*v)))
                    .collect();

                format!(
                    "UpdateCommittee {} {} {} {}",
                    a.to_haskell_str_pp(),
                    b.to_haskell_str_p(),
                    kv.to_haskell_str_p(),
                    d.to_haskell_str_p()
                )
            }
            NewConstitution(a, c) => {
                format!(
                    "NewConstitution {} {}",
                    a.to_haskell_str_pp(),
                    c.to_haskell_str_p()
                )
            }
            InfoAction => "InfoAction".to_string(),
        }
    }

    fn to_haskell_str_p(&self) -> String {
        let str = self.to_haskell_str();

        if str == "InfoAction" {
            return str;
        }
        format!("({})", self.to_haskell_str())
    }
}

// https://github.com/IntersectMBO/cardano-ledger/blob/7683b73971a800b36ca7317601552685fa0701ed/eras/conway/impl/src/Cardano/Ledger/Conway/PParams.hs#L511
impl HaskellDisplay for PParamsUpdate {
    fn to_haskell_str(&self) -> String {
        format!(
            "PParamsUpdate (ConwayPParams {{cppMinFeeA = {}, cppMinFeeB = {}, cppMaxBBSize = {}, cppMaxTxSize = {}, cppMaxBHSize = {}, cppKeyDeposit = {}, cppPoolDeposit = {}, \
             cppEMax = {}, cppNOpt = {}, cppA0 = {}, cppRho = {}, cppTau = {}, cppProtocolVersion = {}, cppMinPoolCost = {}, cppCoinsPerUTxOByte = {}, cppCostModels = {}, \
             cppPrices = {}, cppMaxTxExUnits = {}, cppMaxBlockExUnits = {}, cppMaxValSize = {}, cppCollateralPercentage = {}, cppMaxCollateralInputs = {}, \
             cppPoolVotingThresholds = {}, cppDRepVotingThresholds = {}, cppCommitteeMinSize = {}, cppCommitteeMaxTermLength = {}, cppGovActionLifetime = {}, \
             cppGovActionDeposit = {}, cppDRepDeposit = {}, cppDRepActivity = {}, cppMinFeeRefScriptCostPerByte = {}}})",
            self.minfee_a.as_display_coin(),
            self.minfee_b.as_display_coin(),
            self.max_block_body_size.to_haskell_str(),
            self.max_transaction_size.to_haskell_str(),
            self.max_block_header_size.to_haskell_str(),
            self.key_deposit.as_display_coin(),
            self.pool_deposit.as_display_coin(),
            self.maximum_epoch.as_epoch_interval(),
            self.desired_number_of_stake_pools.to_haskell_str(),
            self.pool_pledge_influence.to_haskell_str_p(),
            self.expansion_rate.to_haskell_str_p(),
            self.treasury_growth_rate.to_haskell_str_p(),
            "NoUpdate",
            self.min_pool_cost.as_display_coin(),
            self.ada_per_utxo_byte.as_display_coin(),
            self.cost_models_for_script_languages.to_haskell_str_p(),
            self.execution_costs.to_haskell_str_p(),
            self.max_tx_ex_units.to_haskell_str_p(),
            self.max_block_ex_units.to_haskell_str_p(),
            self.max_value_size.to_haskell_str(),
            self.collateral_percentage.to_haskell_str(),
            self.max_collateral_inputs.to_haskell_str(),
            self.pool_voting_thresholds.to_haskell_str_p(),
            self.drep_voting_thresholds.to_haskell_str_p(),
            self.min_committee_size.to_haskell_str(),
            self.committee_term_limit.as_epoch_interval(),
            self.governance_action_validity_period.as_epoch_interval(),
            self.governance_action_deposit.as_display_coin(),
            self.drep_deposit.as_display_coin(),
            self.drep_inactivity_period.as_epoch_interval(),
            self.minfee_refscript_cost_per_byte.to_haskell_str_p()
        )
    }
}

impl HaskellDisplay for PoolVotingThresholds {
    fn to_haskell_str(&self) -> String {
        format!(
            "PoolVotingThresholds {{pvtMotionNoConfidence = {}, pvtCommitteeNormal = {}, pvtCommitteeNoConfidence = {}, pvtHardForkInitiation = {}, pvtPPSecurityGroup = {}}}",
            self.motion_no_confidence.to_haskell_str(),
            self.committee_normal.to_haskell_str(),
            self.committee_no_confidence.to_haskell_str(),
            self.hard_fork_initiation.to_haskell_str(),
            self.pp_security_group.to_haskell_str()
        )
    }
}

impl HaskellDisplay for DRepVotingThresholds {
    fn to_haskell_str(&self) -> String {
        format!(
            "DRepVotingThresholds {{dvtMotionNoConfidence = {}, dvtCommitteeNormal = {}, dvtCommitteeNoConfidence = {}, \
     dvtUpdateToConstitution = {}, dvtHardForkInitiation = {}, dvtPPNetworkGroup = {}, dvtPPEconomicGroup = {}, dvtPPTechnicalGroup = {}, dvtPPGovGroup = {}, dvtTreasuryWithdrawal = {}}}",
            self.motion_no_confidence.to_haskell_str(),
            self.committee_normal.to_haskell_str(),
            self.committee_no_confidence.to_haskell_str(),
            self.update_to_constitution.to_haskell_str(),
            self.hard_fork_initiation.to_haskell_str(),
            self.pp_network_group.to_haskell_str(),
            self.pp_economic_group.to_haskell_str(),
            self.pp_technical_group.to_haskell_str(),
            self.pp_gov_group.to_haskell_str(),
            self.treasury_withdrawal.to_haskell_str()
        )
    }
}

impl HaskellDisplay for ExUnits {
    fn to_haskell_str(&self) -> String {
        format!(
            "WrapExUnits {{unWrapExUnits = ExUnits' {{exUnitsMem' = {}, exUnitsSteps' = {}}}}}",
            self.mem, self.steps
        )
    }
}
impl HaskellDisplay for ExUnitPrices {
    fn to_haskell_str(&self) -> String {
        format!(
            "Prices {{prMem = {}, prSteps = {}}}",
            self.mem_price.to_haskell_str(),
            self.step_price.to_haskell_str()
        )
    }
}
impl HaskellDisplay for RationalNumber {
    fn to_haskell_str(&self) -> String {
        format!("{} % {}", self.numerator, self.denominator)
    }
}

impl HaskellDisplay for Constitution {
    fn to_haskell_str(&self) -> String {
        format!(
            "Constitution {{constitutionAnchor = {}, constitutionScript = {}}}",
            self.anchor.to_haskell_str(),
            self.script.to_haskell_str_p()
        )
    }
}
impl HaskellDisplay for Anchor {
    fn to_haskell_str(&self) -> String {
        format!(
            "Anchor {{anchorUrl = {}, anchorDataHash = {}}}",
            self.url.as_url(),
            self.data_hash.as_safe_hash()
        )
    }
}

impl HaskellDisplay for ProposalProcedure {
    fn to_haskell_str(&self) -> String {
        format!(
            "ProposalProcedure {{pProcDeposit = {}, pProcReturnAddr = {}, pProcGovAction = {}, pProcAnchor = {}}}",
            self.deposit.as_display_coin(),
            self.return_addr.to_haskell_str(),
            self.gov_action.to_haskell_str(),
            self.anchor.to_haskell_str()
        )
    }
}

impl HaskellDisplay for ScriptHash {
    fn to_haskell_str(&self) -> String {
        format!("ScriptHash \"{self}\"")
    }
}

impl HaskellDisplay for StakeCredential {
    fn to_haskell_str(&self) -> String {
        use StakeCredential::*;

        match self {
            AddrKeyhash(key_hash) => key_hash.as_key_hash_obj(),
            ScriptHash(scrpt) => scrpt.as_script_hash_obj(),
        }
    }
}

impl HaskellDisplay for Credential {
    fn to_haskell_str(&self) -> String {
        use Credential::*;

        match self {
            ScriptHashObj(key_hash) => key_hash.as_script_hash_obj(),
            KeyHashObj(key_hash) => key_hash.as_key_hash_obj(),
        }
    }
}

impl<K, V> HaskellDisplay for HashMap<K, V>
where
    K: HaskellDisplay + Eq + std::hash::Hash,
    V: HaskellDisplay,
{
    fn to_haskell_str(&self) -> String {
        let result = self
            .iter()
            .map(|item| format!("({},{})", item.0.to_haskell_str(), item.1.to_haskell_str()))
            .collect::<Vec<_>>()
            .join(",");

        format!("fromList [{result}]")
    }
}

impl<K, V> HaskellDisplay for OHashMap<K, V>
where
    K: HaskellDisplay,
    V: HaskellDisplay,
{
    fn to_haskell_str(&self) -> String {
        let result = self
            .0
            .iter()
            .map(|item| format!("({},{})", item.0.to_haskell_str(), item.1.to_haskell_str()))
            .collect::<Vec<_>>()
            .join(",");

        format!("fromList [{result}]")
    }
}

impl HaskellDisplay for EpochNo {
    fn to_haskell_str(&self) -> String {
        self.0.as_epoch_no()
    }
}

impl HaskellDisplay for i64 {
    fn to_haskell_str(&self) -> String {
        self.to_string()
    }

    fn to_haskell_str_p(&self) -> String {
        if *self >= 0 {
            self.to_string()
        } else {
            format!("({self})")
        }
    }
}

impl HaskellDisplay for u8 {
    fn to_haskell_str(&self) -> String {
        format!("{self}")
    }
}

impl<T> HaskellDisplay for Vec<T>
where
    T: HaskellDisplay,
{
    fn to_haskell_str(&self) -> String {
        let mut iter = self.iter();
        if let Some(first) = iter.next() {
            let mut result = first.to_haskell_str();
            result.push_str(" :| [");

            if iter.len() > 0 {
                for item in iter {
                    result.push_str(&format!("{},", item.to_haskell_str()));
                }
                result.truncate(result.len() - 1);
            }
            result.push(']');

            result
        } else {
            "fromList []".to_string()
        }
    }
}

impl<T> HaskellDisplay for Set<T>
where
    T: HaskellDisplay,
{
    fn to_haskell_str(&self) -> String {
        self.deref().as_from_list()
    }
}

impl<T> HaskellDisplay for BTreeSet<T>
where
    T: HaskellDisplay,
{
    fn to_haskell_str(&self) -> String {
        self.as_from_list()
    }
}

impl<T, H> HaskellDisplay for (T, H)
where
    T: HaskellDisplay,
    H: HaskellDisplay,
{
    fn to_haskell_str(&self) -> String {
        format!("({},{})", self.0.to_haskell_str(), self.1.to_haskell_str())
    }
    fn to_haskell_str_p(&self) -> String {
        format!(
            "({},{})",
            self.0.to_haskell_str_p(),
            self.1.to_haskell_str()
        )
    }
}

impl<T, H, K> HaskellDisplay for (T, H, K)
where
    T: HaskellDisplay + 'static,
    H: HaskellDisplay + 'static,
    K: HaskellDisplay + 'static,
{
    fn to_haskell_str(&self) -> String {
        format!(
            "({},{},{})",
            if is_primitive::<T>() {
                self.0.to_haskell_str()
            } else {
                self.0.to_haskell_str_p()
            },
            if is_primitive::<H>() {
                self.1.to_haskell_str()
            } else {
                self.1.to_haskell_str_p()
            },
            if is_primitive::<K>() {
                self.2.to_haskell_str()
            } else {
                self.2.to_haskell_str_p()
            }
        )
    }
}

impl HaskellDisplay for Voter {
    fn to_haskell_str(&self) -> String {
        use Voter::*;

        match self {
            ConstitutionalCommitteeKey(addr) => {
                format!("CommitteeVoter ({})", addr.as_key_hash_obj())
            }
            ConstitutionalCommitteeScript(scrpt) => {
                format!("CommitteeVoter ({})", scrpt.as_script_hash_obj())
            }
            DRepKey(addr) => {
                format!("DRepVoter ({})", addr.as_key_hash_obj())
            }
            DRepScript(scrpt) => {
                format!("DRepVoter ({})", scrpt.as_script_hash_obj())
            }
            StakePoolKey(addr) => {
                format!("StakePoolVoter ({})", addr.as_key_hash())
            }
        }
    }
}

impl HaskellDisplay for DisplayScriptHash {
    fn to_haskell_str(&self) -> String {
        self.0.as_script_hash()
    }
}

impl HaskellDisplay for VKey {
    fn to_haskell_str(&self) -> String {
        self.0.as_vkey()
    }
}
trait AsTransactionId {
    fn as_tx_id(&self) -> String;
}

trait AsTransactionIx {
    fn as_tx_ix(&self) -> String;
}

trait AsSafeHash {
    fn as_safe_hash(&self) -> String;
}

trait AsKeyHash {
    fn as_key_hash(&self) -> String;
}

trait AsVKey {
    fn as_vkey(&self) -> String;
}

trait AsScriptHashObject {
    fn as_script_hash_obj(&self) -> String;
}

trait AsFromList {
    fn as_from_list(&self) -> String;
}

trait AsKeyHashObject {
    fn as_key_hash_obj(&self) -> String;
}

trait AsScriptHash {
    fn as_script_hash(&self) -> String;
}

trait AsUrl {
    fn as_url(&self) -> String;
}

trait AsProtocolVersion {
    fn as_protocol_version(&self) -> String;
}

impl AsUrl for String {
    fn as_url(&self) -> String {
        format!("Url {{urlToText = \"{self}\"}}")
    }
}
impl AsTransactionId for [u8] {
    fn as_tx_id(&self) -> String {
        format!("TxId {{unTxId = {}}}", self.as_safe_hash())
    }
}

impl AsTransactionIx for u64 {
    fn as_tx_ix(&self) -> String {
        format!("TxIx {{unTxIx = {self}}}")
    }
}

trait AsIx {
    fn as_asix(&self) -> String;
}

impl AsIx for u64 {
    fn as_asix(&self) -> String {
        format!("AsIx {{unAsIx = {self}}}")
    }
}

trait AsItem {
    fn as_asitem(&self) -> String;
}

impl<T> AsItem for T
where
    T: HaskellDisplay,
{
    fn as_asitem(&self) -> String {
        format!("AsItem {{unAsItem = {}}}", self.to_haskell_str())
    }
}

trait AsCertIx {
    fn as_cert_ix(&self) -> String;
}

impl AsCertIx for u64 {
    fn as_cert_ix(&self) -> String {
        format!("CertIx {{unCertIx = {self}}}")
    }
}

impl AsSafeHash for [u8] {
    fn as_safe_hash(&self) -> String {
        let hex = hex::encode(self);
        format!("SafeHash \"{hex}\"")
    }
}

impl AsSafeHash for Hash<28> {
    fn as_safe_hash(&self) -> String {
        let hex = hex::encode(self.as_ref());
        format!("SafeHash \"{hex}\"")
    }
}

impl<T> AsSafeHash for Nullable<T>
where
    T: AsSafeHash + std::clone::Clone,
{
    fn as_safe_hash(&self) -> String {
        match self {
            Nullable::Some(v) => v.as_safe_hash(),
            _ => "SNothing".to_string(),
        }
    }
}

impl AsKeyHash for [u8] {
    fn as_key_hash(&self) -> String {
        let hex = hex::encode(self);
        format!("KeyHash {{unKeyHash = \"{hex}\"}}")
    }
}
impl AsKeyHash for Hash<28> {
    fn as_key_hash(&self) -> String {
        self.as_ref().as_key_hash()
    }
}
impl AsKeyHash for Set<Hash<28>> {
    fn as_key_hash(&self) -> String {
        self.deref()
            .iter()
            .map(|x| x.as_key_hash().as_is())
            .collect::<Vec<_>>()
            .as_from_list()
    }
}

trait AsDelegStake {
    fn as_deleg_stake(&self) -> String;
    fn as_deleg_stake_vote(&self) -> String;
}
impl AsDelegStake for PoolKeyhash {
    fn as_deleg_stake(&self) -> String {
        format!("DelegStake ({})", self.deref().as_key_hash())
    }
    fn as_deleg_stake_vote(&self) -> String {
        format!("DelegStakeVote ({})", self.deref().as_key_hash())
    }
}
trait AsPolicyId {
    fn as_policy_id(&self) -> String;
}
impl AsPolicyId for Hash<28> {
    fn as_policy_id(&self) -> String {
        format!("PolicyID {{policyID = {}}}", self.as_script_hash())
    }
}

impl AsVKey for [u8] {
    fn as_vkey(&self) -> String {
        let hex = hex::encode(self);
        format!("VKey (VerKeyEd25519DSIGN \"{hex}\")")
    }
}

impl AsScriptHashObject for [u8] {
    fn as_script_hash_obj(&self) -> String {
        format!("ScriptHashObj ({})", self.as_script_hash())
    }
}

impl AsKeyHashObject for [u8] {
    fn as_key_hash_obj(&self) -> String {
        format!("KeyHashObj ({})", self.as_key_hash())
    }
}

impl AsScriptHash for [u8] {
    fn as_script_hash(&self) -> String {
        let hex = hex::encode(self);
        format!("ScriptHash \"{hex}\"")
    }
}

impl AsScriptHash for Hash<28> {
    fn as_script_hash(&self) -> String {
        let hex = hex::encode(self.deref());
        format!("ScriptHash \"{hex}\"")
    }
}

impl AsProtocolVersion for ProtocolVersion {
    fn as_protocol_version(&self) -> String {
        format!(
            "ProtVer {{pvMajor = Version {}, pvMinor = {}}}",
            self.0, self.1
        )
    }
}

impl<T: HaskellDisplay> AsFromList for Vec<&Option<T>> {
    fn as_from_list(&self) -> String {
        let result = self
            .iter()
            .map(|item| match item {
                Some(v) => v.to_haskell_str(),
                None => "Nothing".to_string(),
            })
            .collect::<Vec<_>>()
            .join(",");

        format!("fromList [{result}]")
    }
}

impl<T: HaskellDisplay> AsFromList for Vec<T>
where
    T: HaskellDisplay,
{
    fn as_from_list(&self) -> String {
        let result = self
            .iter()
            .map(|item| item.to_haskell_str())
            .collect::<Vec<_>>()
            .join(",");

        format!("fromList [{result}]")
    }
}

impl<T: HaskellDisplay> AsFromList for BTreeSet<T>
where
    T: HaskellDisplay,
{
    fn as_from_list(&self) -> String {
        let result = self
            .iter()
            .map(|item| item.to_haskell_str())
            .collect::<Vec<_>>()
            .join(",");

        format!("fromList [{result}]")
    }
}

impl<T: HaskellDisplay> AsFromList for &Vec<T> {
    fn as_from_list(&self) -> String {
        let result = self
            .iter()
            .map(|item| item.to_haskell_str())
            .collect::<Vec<_>>()
            .join(",");

        format!("fromList [{result}]")
    }
}

impl HaskellDisplay for [String] {
    fn to_haskell_str(&self) -> String {
        let result = self.join(",");

        format!("fromList [{result}]")
    }
}

trait AsDisplayCoin {
    fn as_display_coin(&self) -> String;
}

impl AsDisplayCoin for u64 {
    fn as_display_coin(&self) -> String {
        format!("Coin {self}")
    }
}

impl AsDisplayCoin for Coin {
    fn as_display_coin(&self) -> String {
        format!("Coin {}", self.to_haskell_str())
    }
}
trait AsEpochInterval {
    fn as_epoch_interval(&self) -> String;
}

impl AsEpochInterval for Option<u64> {
    fn as_epoch_interval(&self) -> String {
        match self {
            Option::Some(v) => format!("SJust (EpochInterval {})", v.to_haskell_str()),
            _ => "SNothing".to_string(),
        }
    }
}

impl AsDisplayCoin for Option<u64> {
    fn as_display_coin(&self) -> String {
        match self {
            Option::Some(v) => format!("SJust (Coin {})", v.to_haskell_str()),
            _ => "SNothing".to_string(),
        }
    }
}

impl AsDisplayCoin for Option<Coin> {
    fn as_display_coin(&self) -> String {
        match self {
            Option::Some(v) => format!("SJust (Coin {})", v.to_haskell_str()),
            _ => "SNothing".to_string(),
        }
    }
}
impl AsDisplayCoin for Option<&Coin> {
    fn as_display_coin(&self) -> String {
        match self {
            Option::Some(v) => format!("(SJust (Coin {}))", v.to_haskell_str()),
            _ => "SNothing".to_string(),
        }
    }
}

impl HaskellDisplay for u64 {
    fn to_haskell_str(&self) -> String {
        self.to_string()
    }
}

impl HaskellDisplay for String {
    fn to_haskell_str(&self) -> String {
        self.as_text()
    }
}

trait AsStrictSeq {
    fn as_strict_seq(&self) -> String;
}

impl<T> AsStrictSeq for Vec<T>
where
    T: HaskellDisplay,
{
    fn as_strict_seq(&self) -> String {
        format!("StrictSeq {{fromStrict = {}}}", self.as_from_list())
    }
}

trait AsText {
    fn as_text(&self) -> String;
}

impl AsText for String {
    fn as_text(&self) -> String {
        haskell_show_string(self)
    }
}

impl AsText for Bytes {
    fn as_text(&self) -> String {
        let v = self.deref();
        let str = v.iter().map(|&c| c as char).collect::<String>();
        haskell_show_string(&str)
    }
}

impl AsText for ByteVec {
    fn as_text(&self) -> String {
        let v = self.deref();
        let str = v.iter().skip(2).map(|&c| c as char).collect::<String>();

        haskell_show_string(&str)
    }
}

impl AsText for [u8] {
    fn as_text(&self) -> String {
        let str = self.iter().map(|&c| c as char).collect::<String>();

        haskell_show_string(&str)
    }
}

trait AsDerivationPath {
    fn as_deriv_path(&self) -> String;
}

impl AsDerivationPath for ByteVec {
    fn as_deriv_path(&self) -> String {
        format!(
            "HDAddressPayload {{getHDAddressPayload = {}}}",
            self.as_text()
        )
    }
}

impl HaskellDisplay for Bytes {
    fn to_haskell_str(&self) -> String {
        self.as_text()
    }
}

impl<K, V> HaskellDisplay for KeyValuePairs<K, V>
where
    K: Clone + HaskellDisplay,
    V: Clone + HaskellDisplay,
{
    fn to_haskell_str(&self) -> String {
        let result = self
            .iter()
            .map(|(k, v)| format!("({},{})", k.to_haskell_str(), v.to_haskell_str()))
            .collect::<Vec<_>>()
            .join(",");
        format!("fromList [{result}]")
    }
}

impl HaskellDisplay
    for NonEmptyKeyValuePairs<Voter, NonEmptyKeyValuePairs<GovActionId, VotingProcedure>>
{
    fn to_haskell_str(&self) -> String {
        let result = self
            .iter()
            .map(|(k, v)| format!("({},{})", k.to_haskell_str(), v.as_from_list()))
            .collect::<Vec<_>>()
            .join(",");
        format!("fromList [{result}]")
    }
}

impl HaskellDisplay for VotingProcedure {
    fn to_haskell_str(&self) -> String {
        format!(
            "VotingProcedure {{vProcVote = {}, vProcAnchor = {}}}",
            self.vote.to_haskell_str(),
            self.anchor.to_haskell_str()
        )
    }
}

impl HaskellDisplay for Vote {
    fn to_haskell_str(&self) -> String {
        use Vote::*;
        match self {
            Yes => "VoteYes".to_string(),
            No => "VoteNo".to_string(),
            Abstain => "Abstain".to_string(),
        }
    }
}

impl HaskellDisplay for DisplayPolicyId {
    fn to_haskell_str(&self) -> String {
        format!("PolicyID {{policyID = {}}}", self.0.as_script_hash())
    }
}

trait AsAssetName {
    fn as_asset_name(&self) -> String;
}

impl AsAssetName for AssetName {
    fn as_asset_name(&self) -> String {
        format!("\"{self}\"")
    }
}

impl HaskellDisplay for DisplayCoin {
    fn to_haskell_str(&self) -> String {
        self.0.as_display_coin()
    }
}

// This type is used to escape showing strings as Haskell strings.
#[derive(Debug, Clone)]
pub struct DisplayAsIs(String);

impl HaskellDisplay for DisplayAsIs {
    fn to_haskell_str(&self) -> String {
        self.0.to_string()
    }
}

trait AsIs {
    fn as_is(&self) -> DisplayAsIs;
}

impl AsIs for String {
    fn as_is(&self) -> DisplayAsIs {
        DisplayAsIs(self.to_string())
    }
}

impl HaskellDisplay for TransactionOutput {
    fn to_haskell_str(&self) -> String {
        use TransactionOutput::*;
        match self {
            Legacy(txo) => {
                let address = txo.address.as_address();
                let value = txo.amount.to_haskell_str();
                let datum = match txo.datum_hash {
                    Some(hash) => hash.as_datum_hash(),
                    None => "NoDatum".to_string(),
                };

                format!("{address},{value},{datum},SNothing")
            }
            Current(txo) => {
                let address = txo.address.as_address();
                let value = txo.amount.to_haskell_str();
                let datum = match &txo.inline_datum {
                    Some(option) => match option {
                        DatumOption::Hash(hash) => hash.as_datum_hash(),
                        DatumOption::Data(cbor_wrap) => {
                            let mut payload: Vec<u8> = vec![];
                            pallas_codec::minicbor::encode(cbor_wrap.deref(), &mut payload)
                                .unwrap();
                            let str = payload.iter().map(|&c| c as char).collect::<String>();
                            format!("Datum {}", haskell_show_string(&str))
                        }
                    },
                    None => "NoDatum".to_string(),
                };

                let script = txo.script_ref.to_haskell_str();

                format!("{address},{value},{datum},{script}")
            }
        }
    }
}

impl HaskellDisplay for PseudoScript<NativeScript> {
    fn to_haskell_str(&self) -> String {
        use PseudoScript::*;
        match self {
            NativeScript(ns) => format!("TimelockScript {}", ns.to_haskell_str()),
            PlutusV1Script(ps) => format!(
                "PlutusScript PlutusV1 {}",
                Hasher::<224>::hash_tagged(ps.0.as_slice(), 1).as_script_hash()
            ),
            PlutusV2Script(ps) => format!(
                "PlutusScript PlutusV2 {}",
                Hasher::<224>::hash_tagged(ps.0.as_slice(), 2).as_script_hash()
            ),
            PlutusV3Script(ps) => format!(
                "PlutusScript PlutusV3 {}",
                Hasher::<224>::hash_tagged(ps.0.as_slice(), 3).as_script_hash()
            ),
        }
    }
}

impl HaskellDisplay for NativeScript {
    fn to_haskell_str(&self) -> String {
        use NativeScript::*;

        let str = match self {
            ScriptPubkey(key_hash) => {
                format!("Signature ({})", key_hash.as_key_hash())
            }
            ScriptAll(vec) => format!("AllOf ({})", vec.as_strict_seq()),
            ScriptAny(vec) => format!("AnyOf ({})", vec.as_strict_seq()),
            ScriptNOfK(m, vec) => format!("MOfN {} ({})", m, vec.as_strict_seq()),
            InvalidBefore(slot_no) => format!("TimeStart ({})", slot_no.as_slot_no()),
            InvalidHereafter(slot_no) => format!("TimeExpire ({})", slot_no.as_slot_no()),
        };

        format!(
            "TimelockConstr {} ({})",
            str,
            Hasher::<256>::hash_cbor(self).as_blake2b256()
        )
    }
}

impl HaskellDisplay for DatumOption {
    fn to_haskell_str(&self) -> String {
        use DatumOption::*;
        match self {
            Hash(hash) => hash.as_datum_hash(),
            Data(cbor_wrap) => {
                format!("Datum ({})", cbor_wrap.to_haskell_str())
            }
        }
    }
}
impl<T> HaskellDisplay for CborWrap<T>
where
    T: HaskellDisplay,
{
    fn to_haskell_str(&self) -> String {
        self.0.to_haskell_str()
    }
}

impl<T> HaskellDisplay for TagWrap<T, 256>
where
    T: HaskellDisplay,
{
    fn to_haskell_str(&self) -> String {
        self.0.to_haskell_str()
    }
}
impl HaskellDisplay for ByronAddress {
    fn to_haskell_str(&self) -> String {
        let payload = self.decode().unwrap();
        format!("BootstrapAddress {}", payload.to_haskell_str_p())
    }
}

impl HaskellDisplay for AddressPayload {
    fn to_haskell_str(&self) -> String {
        let root = hex::encode(self.root);

        use AddrType::*;
        let addr_type = match self.addrtype {
            PubKey => "ATVerKey",
            Script => "Not used",
            Redeem => "ATRedeem",
            Other(_) => "Not possible",
        };
        format!(
            "Address {{addrRoot = {}, addrAttributes = {}, addrType = {}}}",
            root,
            self.attributes.to_haskell_str(),
            addr_type
        )
    }
}

impl HaskellDisplay for OrderPreservingProperties<AddrAttrProperty> {
    fn to_haskell_str(&self) -> String {
        let items = self.deref();

        let mut att_map: HashMap<&str, String> = HashMap::with_capacity(2);

        for item in items {
            use AddrAttrProperty::*;

            match item {
                DerivationPath(bv) => {
                    att_map.insert(
                        "aaVKDerivationPath",
                        format!("Just ({})", bv.as_deriv_path()),
                    );
                }
                NetworkTag(bv) => {
                    let magic: NetworkMagic = pallas_codec::minicbor::decode(bv.as_ref()).unwrap();
                    att_map.insert("aaNetworkMagic", magic.as_network_magic());
                }
                _ => {}
            }
        }

        format!(
            "Attributes {{ data_ = AddrAttributes {{aaVKDerivationPath = {}, aaNetworkMagic = {}}} }}",
            att_map
                .get("aaVKDerivationPath")
                .unwrap_or(&"Nothing".to_string()),
            att_map
                .get("aaNetworkMagic")
                .unwrap_or(&"NetworkMainOrStage".to_string())
        )
    }
}

impl HaskellDisplay for Utxo {
    fn to_haskell_str(&self) -> String {
        let result = self
            .0
             .0
            .iter()
            .map(|item| {
                format!(
                    "({},{})",
                    item.0.to_haskell_str(),
                    item.1.to_haskell_str_p()
                )
            })
            .collect::<Vec<_>>()
            .join(",");

        format!("UTxO (fromList [{result}])")
    }
}

impl HaskellDisplay for DisplayVotingProcedures {
    fn to_haskell_str(&self) -> String {
        format!(
            "VotingProcedures {{unVotingProcedures = {}}}",
            self.0.to_haskell_str()
        )
    }
}

impl HaskellDisplay for Value {
    fn to_haskell_str(&self) -> String {
        use Value::*;

        match self {
            Coin(c) => format!("{} (MultiAsset (fromList []))", c.as_mary_value()),
            Multiasset(mary_value, multi_asset) => format!(
                "{} {}",
                mary_value.as_mary_value(),
                multi_asset.to_haskell_str_p()
            ),
        }
    }
}

impl<T> HaskellDisplay for Multiasset<T>
where
    T: HaskellDisplay + Clone + std::fmt::Debug,
{
    fn to_haskell_str(&self) -> String {
        self.as_multiasset()
    }
}

impl HaskellDisplay for PositiveCoin {
    fn to_haskell_str(&self) -> String {
        u64::from(*self).as_mary_value()
    }
}

impl HaskellDisplay for Coin {
    fn to_haskell_str(&self) -> String {
        u64::from(*self).to_string()
    }
}

impl HaskellDisplay for &Coin {
    fn to_haskell_str(&self) -> String {
        u64::from(*self).to_string()
    }
}

trait AsMaryValue {
    fn as_mary_value(&self) -> String;
}

impl AsMaryValue for Coin {
    fn as_mary_value(&self) -> String {
        u64::from(*self).as_mary_value()
    }
}

impl AsMaryValue for u64 {
    fn as_mary_value(&self) -> String {
        format!("MaryValue (Coin {self})")
    }
}

impl HaskellDisplay for ShelleyAddress {
    fn to_haskell_str(&self) -> String {
        format!(
            "Addr {} ({}) {}",
            self.network().to_haskell_str(),
            self.payment().to_haskell_str(),
            self.delegation().to_haskell_str_p()
        )
    }
}

impl HaskellDisplay for pallas_addresses::Network {
    fn to_haskell_str(&self) -> String {
        match self {
            pallas_addresses::Network::Mainnet => "Mainnet".to_string(),
            pallas_addresses::Network::Testnet => "Testnet".to_string(),
            pallas_addresses::Network::Other(magic) => format!("Other {magic}"),
        }
    }
}

impl HaskellDisplay for ShelleyPaymentPart {
    fn to_haskell_str(&self) -> String {
        use ShelleyPaymentPart::*;
        match self {
            Key(hash) => hash.as_key_hash_obj(),
            Script(hash) => hash.as_script_hash_obj(),
        }
    }
}

impl HaskellDisplay for ShelleyDelegationPart {
    fn to_haskell_str(&self) -> String {
        use ShelleyDelegationPart::*;
        match self {
            Key(hash) => {
                format!("StakeRefBase ({})", hash.as_key_hash_obj())
            }
            Script(hash) => {
                format!("StakeRefBase ({})", hash.as_script_hash_obj())
            }
            Pointer(pointer) => {
                format!("StakeRefPtr ({})", pointer.to_haskell_str())
            }
            Null => "StakeRefNull".to_string(),
        }
    }

    fn to_haskell_str_p(&self) -> String {
        let str = self.to_haskell_str();
        if str == "StakeRefNull" {
            str.to_string()
        } else {
            format!("({str})")
        }
    }
}

impl HaskellDisplay for Pointer {
    fn to_haskell_str(&self) -> String {
        format!(
            "Ptr ({}) ({}) ({})",
            self.slot().as_slot_no(),
            self.tx_idx().as_tx_ix(),
            self.cert_idx().as_cert_ix()
        )
    }
}

impl HaskellDisplay for pallas_addresses::Address {
    fn to_haskell_str(&self) -> String {
        use pallas_addresses::Address::*;
        match self {
            Byron(addr) => format!("AddrBootstrap {}", addr.to_haskell_str_p()),
            Shelley(addr) => addr.to_haskell_str(),
            Stake(addr) => addr.to_hex(),
        }
    }
}

impl HaskellDisplay for DisplayAddress {
    fn to_haskell_str(&self) -> String {
        use pallas_addresses::Address::*;
        match pallas_addresses::Address::from_bytes(&self.0).unwrap() {
            Byron(addr) => format!("AddrBootstrap {}", addr.to_haskell_str_p()),
            Shelley(addr) => addr.to_haskell_str(),
            Stake(addr) => addr.to_hex(),
        }
    }
}

trait AsMultiasset {
    fn as_multiasset(&self) -> String;
}

impl<T> AsMultiasset for Multiasset<T>
where
    T: HaskellDisplay + Clone + std::fmt::Debug,
{
    fn as_multiasset(&self) -> String {
        let v = self.clone().to_vec();
        let str = v
            .iter()
            .map(|item| {
                let policy_id = item.0.as_policy_id();
                let rest = item
                    .1
                    .iter()
                    .map(move |inner| {
                        let asset_name = inner.0.as_asset_name();
                        let amount = inner.1.to_haskell_str();
                        format!("({asset_name},{amount})")
                    })
                    .collect::<Vec<_>>();
                format!("({},fromList [{}])", policy_id, rest.join(","))
            })
            .collect::<Vec<_>>()
            .join(",");
        format!("MultiAsset (fromList [{str}])")
    }
}

trait AsDatumHash {
    fn as_datum_hash(&self) -> String;
}

impl AsDatumHash for DatumHash {
    fn as_datum_hash(&self) -> String {
        format!("DatumHash ({})", self.as_safe_hash())
    }
}

impl HaskellDisplay for PlutusData {
    fn to_haskell_str(&self) -> String {
        use pallas_network::miniprotocols::localstate::queries_v16::BigInt as Big;
        use PlutusData::*;

        match self {
            Constr(constr) => constr.fields.to_haskell_str(),
            Map(key_value_pairs) => key_value_pairs.to_haskell_str().to_string(),
            BigInt(big_int) => match big_int {
                Big::Int(i) => format!("BigInt Int {}", i.to_haskell_str()),
                Big::BigNInt(bb) => {
                    format!("BigNInt {}", bb.to_haskell_str())
                }
                Big::BigUInt(bb) => {
                    format!("BigUInt {}", bb.to_haskell_str())
                }
            },
            BoundedBytes(bb) => bb.to_haskell_str(),
            Array(arr) => format!("Array {}", arr.to_haskell_str()),
        }
    }
}

impl HaskellDisplay for BoundedBytes {
    fn to_haskell_str(&self) -> String {
        let arr = self.deref();

        let str = arr.iter().map(|&c| c as char).collect::<String>();

        haskell_show_string(&str)
    }
}

impl<T> HaskellDisplay for MaybeIndefArray<T>
where
    T: HaskellDisplay,
{
    fn to_haskell_str(&self) -> String {
        use MaybeIndefArray::*;
        let str = match self {
            Def(vec) => vec.as_from_list(),
            Indef(vec) => vec.as_from_list(),
        };

        format!("MaybeIndefArray {str}")
    }
}

impl HaskellDisplay for Int {
    fn to_haskell_str(&self) -> String {
        format!("Int {}", self.0)
    }
}

impl HaskellDisplay for SlotNo {
    fn to_haskell_str(&self) -> String {
        self.0.as_slot_no()
    }
}

trait AsSlotNo {
    fn as_slot_no(&self) -> String;
}

impl AsSlotNo for u64 {
    fn as_slot_no(&self) -> String {
        format!("SlotNo {self}")
    }
}

impl AsSlotNo for SMaybe<u64> {
    fn as_slot_no(&self) -> String {
        match self {
            SMaybe::Some(v) => format!("SJust (SlotNo {v})"),
            _ => "SNothing".to_string(),
        }
    }
}

trait AsBlake2b256 {
    fn as_blake2b256(&self) -> String;
}

impl AsBlake2b256 for Hash<32> {
    fn as_blake2b256(&self) -> String {
        format!("blake2b_256: SafeHash \"{self}\"")
    }
}
impl AsBlake2b256 for Hash<28> {
    fn as_blake2b256(&self) -> String {
        format!("blake2b_256: SafeHash \"{self}\"")
    }
}

impl HaskellDisplay
    for PlutusPurpose<
        TransactionInput,
        PolicyId,
        ConwayTxCert,
        FieldedRewardAccount,
        Voter,
        ProposalProcedure,
    >
{
    fn to_haskell_str(&self) -> String {
        use PlutusPurpose::*;

        match self {
            Minting(policy_id) => format!(
                "ConwayMinting ({})",
                policy_id.as_policy_id().as_is().as_asitem()
            ),
            Spending(txin) => format!("ConwaySpending ({})", txin.as_asitem()),
            Rewarding(item) => {
                format!("ConwayRewarding ({})", item.as_asitem())
            }
            Certifying(cert_index) => format!("ConwayCertifying ({})", cert_index.as_asitem()),
            Voting(gov_id) => format!("ConwayVoting ({})", gov_id.as_asitem()),
            Proposing(proposal_id) => format!("ConwayProposing ({})", proposal_id.as_asitem()),
        }
    }
}

// @todo check this
impl HaskellDisplay for PlutusPurpose<u64, u64, u64, u64, u64, u64> {
    fn to_haskell_str(&self) -> String {
        use PlutusPurpose::*;

        match self {
            Minting(policy_id) => format!("ConwayMinting ({})", policy_id.as_asix()),
            Spending(txin) => format!("ConwaySpending ({})", txin.as_asix()),
            Rewarding(item) => {
                format!("ConwayRewarding ({})", item.as_asix())
            }
            Certifying(cert_index) => format!("ConwayCertifying ({})", cert_index.as_asix()),
            Voting(gov_id) => format!("ConwayVoting ({})", gov_id.as_asix()),
            Proposing(proposal_id) => format!("ConwayProposing ({})", proposal_id.as_asix()),
        }
    }
}

impl HaskellDisplay for ConwayTxCert {
    fn to_haskell_str(&self) -> String {
        use ConwayTxCert::*;
        match self {
            Deleg(cert) => {
                format!("ConwayTxCertDeleg {}", cert.to_haskell_str_p())
            }
            Pool(cert) => {
                format!("ConwayTxCertPool {}", cert.to_haskell_str_p())
            }
            Gov(cert) => {
                format!("ConwayTxCertGov {}", cert.to_haskell_str_p())
            }
        }
    }
}

impl HaskellDisplay for Relay {
    fn to_haskell_str(&self) -> String {
        use Relay::*;
        match self {
            SingleHostAddr(port, ipv4, ipv6) => {
                format!(
                    "SingleHostAddr {} {} {}",
                    port.as_port(),
                    ipv4.as_ipv4(),
                    ipv6.as_ipv6()
                )
            }
            SingleHostName(port, dns) => {
                format!("SingleHostName {} ({})", port.as_port(), dns.as_dns_name())
            }
            MultiHostName(dns) => format!("MultiHostName ({})", dns.as_dns_name()),
        }
    }
}
impl HaskellDisplay for PoolMetadata {
    fn to_haskell_str(&self) -> String {
        format!(
            "PoolMetadata {{pmUrl = {}, pmHash = {}}}",
            self.url.as_url(),
            self.hash.as_text()
        )
    }
}

impl HaskellDisplay for Certificate {
    fn to_haskell_str(&self) -> String {
        use Certificate::*;
        match self {
            StakeRegistration(cred) => {
                format!("ConwayRegCert {} SNothing", cred.to_haskell_str_p())
            },
            StakeDeregistration(cred) => {
                format!("ConwayUnRegCert {} SNothing", cred.to_haskell_str_p())
            },
            StakeDelegation(cred, hash) => format!(
                "ConwayDelegCert {} ({})",
                cred.to_haskell_str_p(),
                hash.as_deleg_stake()
            ),
            PoolRegistration {
                operator,
                vrf_keyhash,
                pledge,
                cost,
                margin,
                reward_account,
                pool_owners,
                relays,
                pool_metadata,
            } => format!(
                "RegPool (PoolParams {{ppId = {}, ppVrf = {}, ppPledge = {}, ppCost = {}, ppMargin = {}, ppRewardAccount = {}, ppOwners = {}, ppRelays = {}, ppMetadata = {}}})",
                operator.as_key_hash(),
                vrf_keyhash.to_string().to_haskell_str(),
                pledge.as_display_coin(),
                cost.as_display_coin(),
                margin.to_haskell_str(),
                reward_account.to_haskell_str(),
                pool_owners.as_key_hash(),
                relays.as_strict_seq(),
                pool_metadata.to_haskell_str()
            ),
            PoolRetirement(hash, epoch) => format!(
                "RetirePool ({}) ({})",
                hash.as_key_hash(),
                epoch.as_epoch_no()
            ),
            Reg(cred, deposit) => format!(
                "ConwayRegCert {} {}",
                cred.to_haskell_str_p(),
                Some(deposit).as_display_coin()
            ),
            UnReg(cred, deposit) => format!(
                "ConwayUnRegCert {} {}",
                cred.to_haskell_str_p(),
                Some(deposit).as_display_coin()
            ),
            VoteDeleg(cred, drep) => format!(
                "ConwayDelegCert {} (DelegVote {})",
                cred.to_haskell_str_p(),
                drep.to_haskell_str_p()
            ),
            StakeVoteDeleg(cred, hash, drep) => format!(
                "ConwayDelegCert {} ({} {})",
                cred.to_haskell_str_p(),
                hash.as_deleg_stake_vote(),
                drep.to_haskell_str_p()
            ),
            StakeRegDeleg(cred, hash, coin) => format!(
                "ConwayRegDelegCert {} ({}) ({})",
                cred.to_haskell_str_p(),
                hash.as_deleg_stake(),
                coin.as_display_coin()
            ),
            VoteRegDeleg(cred, drep, coin) => {
                format!(
                    "ConwayRegDelegCert {} (DelegVote {}) ({})",
                    cred.to_haskell_str_p(),
                    drep.to_haskell_str_p(),
                    coin.as_display_coin()
                )
            },
            StakeVoteRegDeleg(cred, hash, drep, coin) => format!(
                "ConwayRegDelegCert {} ({} {}) ({})",
                cred.to_haskell_str_p(),
                hash.as_deleg_stake_vote(),
                drep.to_haskell_str_p(),
                coin.as_display_coin()
            ),
            AuthCommitteeHot(cred, key) => format!(
                "ConwayAuthCommitteeHotKey {} {}",
                cred.to_haskell_str_p(),
                key.to_haskell_str_p()
            ),
            ResignCommitteeCold(cred, anchor) => format!(
                "ConwayResignCommitteeColdKey {} {}",
                cred.to_haskell_str_p(),
                anchor.to_haskell_str_p()
            ),
            RegDRepCert(cred, deposit, anchor) => format!(
                "ConwayRegDRep {} ({}) {}",
                cred.to_haskell_str_p(),
                deposit.as_display_coin(),
                anchor.to_haskell_str_p()
            ),
            UnRegDRepCert(cred, deposit) => format!(
                "ConwayUnRegDRep {} ({})",
                cred.to_haskell_str_p(),
                deposit.as_display_coin()
            ),
            UpdateDRepCert(cred, anchor) => format!(
                "ConwayUpdateDRep {} {}",
                cred.to_haskell_str_p(),
                anchor.to_haskell_str_p()
            ),
        }
    }
}

impl<T> HaskellDisplay for SMaybe<T>
where
    T: HaskellDisplay + 'static,
{
    fn to_haskell_str(&self) -> String {
        match self {
            SMaybe::Some(v) => {
                if is_primitive::<T>() {
                    format!("SJust {}", v.to_haskell_str())
                } else {
                    format!("SJust ({})", v.to_haskell_str())
                }
            }
            SMaybe::None => "SNothing".to_string(),
        }
    }

    fn to_haskell_str_p(&self) -> String {
        let str = self.to_haskell_str();
        if &str == "SNothing" {
            return str;
        }
        format!("({})", self.to_haskell_str())
    }
}

impl HaskellDisplay for DisplayOSet<ProposalProcedure> {
    fn to_haskell_str(&self) -> String {
        let seq = self.0.deref().as_strict_seq();

        let mut sorted_vec = self.0.deref().clone();

        sorted_vec.sort();

        format!(
            "OSet {{osSSeq = {}, osSet = {}}}",
            seq,
            sorted_vec.as_from_list()
        )
    }
}

impl<T> HaskellDisplay for Array<T>
where
    T: HaskellDisplay + Clone,
{
    fn to_haskell_str(&self) -> String {
        let value = self
            .0
            .iter()
            .map(|item| item.to_haskell_str())
            .collect::<Vec<_>>()
            .join(",");
        format!("[{value}]")
    }

    fn to_haskell_str_p(&self) -> String {
        let value = self
            .0
            .iter()
            .map(|item| item.to_haskell_str_p())
            .collect::<Vec<_>>()
            .join(",");
        format!("[{value}]")
    }
}

impl HaskellDisplay for DeltaCoin {
    fn to_haskell_str(&self) -> String {
        format!("DeltaCoin {}", self.0.to_haskell_str())
    }
}

impl HaskellDisplay for i32 {
    fn to_haskell_str(&self) -> String {
        if *self >= 0 {
            format!("{self}")
        } else {
            format!("({self})")
        }
    }
}
impl HaskellDisplay for DatumHash {
    fn to_haskell_str(&self) -> String {
        format!("DatumHash \"{}\"", hex::encode(self.as_ref()))
    }
}

fn display_governance_action_id_index(index: &u32) -> String {
    format!("GovActionIx {{unGovActionIx = {index}}}")
}

trait AsDnsName {
    fn as_dns_name(&self) -> String;
}
impl AsDnsName for String {
    fn as_dns_name(&self) -> String {
        format!("DnsName {{dnsToText = {}}}", haskell_show_string(self))
    }
}
trait AsAddress {
    fn as_address(&self) -> String;
}

impl AsAddress for Bytes {
    fn as_address(&self) -> String {
        pallas_addresses::Address::from_bytes(self)
            .unwrap()
            .to_haskell_str()
    }
}
trait AsAuxDataHash {
    fn as_aux_data_hash(&self) -> String;
}

impl AsAuxDataHash for Bytes {
    fn as_aux_data_hash(&self) -> String {
        format!("AuxiliaryDataHash {{unsafeAuxiliaryDataHash = SafeHash \"{self}\"}}")
    }
}

trait AsIPv4 {
    fn as_ipv4(&self) -> String;
}
impl AsIPv4 for Nullable<Bytes> {
    fn as_ipv4(&self) -> String {
        match self {
            Nullable::Some(b) => {
                let str = b
                    .iter()
                    .map(|byte| byte.to_string())
                    .collect::<Vec<_>>()
                    .join(".");
                format!("(SJust {str})")
            }
            _ => "SNothing".to_string(),
        }
    }
}

trait AsIPv6 {
    fn as_ipv6(&self) -> String;
}
impl AsIPv6 for Nullable<Bytes> {
    fn as_ipv6(&self) -> String {
        match self {
            Nullable::Some(b) => {
                let data: [u8; 16] = b
                    .deref()
                    .chunks_exact(4)
                    .flat_map(|x| {
                        let mut y = x.to_vec();
                        y.reverse();
                        y
                    })
                    .collect::<Vec<_>>()
                    .try_into()
                    .expect("slice with incorrect length");

                let ip = Ipv6Addr::from(data).to_string();

                format!("(SJust {ip})")
            }
            _ => "SNothing".to_string(),
        }
    }
}
trait AsIsValid {
    fn as_is_valid(&self) -> &str;
}

impl AsIsValid for bool {
    fn as_is_valid(&self) -> &str {
        if *self {
            "IsValid True"
        } else {
            "IsValid False"
        }
    }
}

trait AsEpochNo {
    fn as_epoch_no(&self) -> String;
}
impl AsEpochNo for u64 {
    fn as_epoch_no(&self) -> String {
        format!("EpochNo {self}")
    }
}

trait AsNetworkMagic {
    fn as_network_magic(&self) -> String;
}

impl AsNetworkMagic for u64 {
    fn as_network_magic(&self) -> String {
        match self {
            0 => "NetworkMainOrStage".to_string(),
            _ => format!("NetworkTestnet {self}"),
        }
    }
}

trait AsPort {
    fn as_port(&self) -> String;
}

impl AsPort for Nullable<u32> {
    fn as_port(&self) -> String {
        match self {
            Nullable::Some(p) => format!("(SJust (Port {{portToWord16 = {p}}}))"),
            _ => "SNothing".to_string(),
        }
    }
}

impl HaskellDisplay for CostModels {
    fn to_haskell_str(&self) -> String {
        fn display_cost_model(version: u64, model_opt: &Option<CostModel>) -> Option<DisplayAsIs> {
            match model_opt {
                Some(model) => {
                    let model_str = model
                        .iter()
                        .map(|cost| cost.to_string())
                        .collect::<Vec<_>>()
                        .join(",");

                    let str =
                        format!("(PlutusV{version},CostModel PlutusV{version} [{model_str}])",);
                    Some(str.as_is())
                }
                _ => None,
            }
        }

        fn display_unknown(kv: &(u64, CostModel)) -> DisplayAsIs {
            let model_str =
                kv.1.iter()
                    .map(|cost| cost.to_string())
                    .collect::<Vec<_>>()
                    .join(",");

            format!("({},[{}])", kv.0, model_str).as_is()
        }

        let known_str: Vec<DisplayAsIs> = [
            display_cost_model(1, &self.plutus_v1),
            display_cost_model(2, &self.plutus_v2),
            display_cost_model(3, &self.plutus_v3),
        ]
        .into_iter()
        .flatten()
        .collect();

        let mut unknown_str = vec![];

        for kv in self.unknown.iter() {
            unknown_str.push(display_unknown(kv));
        }

        format!(
            "CostModels {{_costModelsValid = {}, _costModelsUnknown = {}}}",
            known_str.as_from_list(),
            unknown_str.as_from_list()
        )
    }
}

impl HaskellDisplay for Network {
    fn to_haskell_str(&self) -> String {
        use Network::*;
        match self {
            Mainnet => "Mainnet".to_string(),
            Testnet => "Testnet".to_string(),
        }
    }
}
