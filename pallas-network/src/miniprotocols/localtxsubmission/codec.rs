use pallas_codec::minicbor::data::{IanaTag, Type as CborType};
use pallas_codec::minicbor::{decode, encode, Decode, Decoder, Encode, Encoder};
use pallas_primitives::conway::Certificate;
use pallas_primitives::NetworkId;

use crate::miniprotocols::localtxsubmission::{
    BabbageContextError, CollectError, ConwayCertPredFailure, ConwayCertsPredFailure,
    ConwayContextError, ConwayDelegPredFailure, ConwayGovCertPredFailure, ConwayPlutusPurpose,
    ConwayUtxoWPredFailure, Credential, EpochNo, EraTx, FailureDescription, Message, Mismatch,
    Network, PlutusPurpose, RejectReason, SMaybe, ShelleyPoolPredFailure, TagMismatchDescription,
    TxError, TxOutSource,
};

use std::str::from_utf8;

use super::{
    ConwayTxCert, OHashMap, ShelleyBasedEra, SlotNo, Utxo, UtxoFailure, UtxosFailure,
    ValidityInterval,
};

// `Ctx` generic needed after introducing `ValidityInterval`.
impl<'b, T: Decode<'b, Ctx>, Ctx> Decode<'b, Ctx> for SMaybe<T> {
    fn decode(d: &mut Decoder<'b>, ctx: &mut Ctx) -> Result<Self, decode::Error> {
        let len = d.array()?;
        match len {
            Some(0) => Ok(SMaybe::None),
            Some(1) => Ok(SMaybe::Some(d.decode_with(ctx)?)),
            Some(_) => Err(decode::Error::message("Expected array of length <=1")),
            None => Err(decode::Error::message(
                "Expected array of length <=1, obtained `None`",
            )),
        }
    }
}

// `Ctx` generic needed after introducing `ValidityInterval`.
impl<T, Ctx> Encode<Ctx> for SMaybe<T>
where
    T: Encode<Ctx>,
{
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        ctx: &mut Ctx,
    ) -> Result<(), encode::Error<W::Error>> {
        match self {
            SMaybe::None => {
                e.array(0)?;
            }
            SMaybe::Some(t) => {
                e.array(1)?;
                e.encode_with(t, ctx)?;
            }
        }
        Ok(())
    }
}

impl<Tx, Reject> Encode<()> for Message<Tx, Reject>
where
    Tx: Encode<()>,
    Reject: Encode<()>,
{
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        match self {
            Message::SubmitTx(tx) => {
                e.array(2)?.u16(0)?;
                e.encode(tx)?;
                Ok(())
            }
            Message::AcceptTx => {
                e.array(1)?.u16(1)?;
                Ok(())
            }
            Message::RejectTx(rejection) => {
                e.array(2)?.u16(2)?;
                e.encode(rejection)?;
                Ok(())
            }
            Message::Done => {
                e.array(1)?.u16(3)?;
                Ok(())
            }
        }
    }
}

impl<'b, Tx: Decode<'b, ()>, Reject: Decode<'b, ()> + From<String>> Decode<'b, ()>
    for Message<Tx, Reject>
{
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        if d.array().is_err() {
            // if the first element isn't an array, it's a plutus error
            // the node sends string data
            let rejection = from_utf8(d.input())
                .or(Err(decode::Error::message("Not valid as a string")))?
                .to_string()
                .into();

            return Ok(Message::RejectTx(rejection));
        }

        let label = d.u16()?;

        match label {
            0 => Ok(Message::SubmitTx(d.decode()?)),
            1 => Ok(Message::AcceptTx),
            2 => Ok(Message::RejectTx(d.decode()?)),
            3 => Ok(Message::Done),
            _ => Err(decode::Error::message("can't decode Message")),
        }
    }
}

impl<'b> Decode<'b, ()> for EraTx {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        d.array()?;
        let era = d.u16()?;
        let tag = d.tag()?;
        if tag != IanaTag::Cbor.tag() {
            return Err(decode::Error::message("Expected encoded CBOR data item"));
        }
        Ok(EraTx(era, d.bytes()?.to_vec()))
    }
}

impl Encode<()> for EraTx {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        e.array(2)?;
        e.u16(self.0)?;
        e.tag(IanaTag::Cbor)?;
        e.bytes(&self.1)?;
        Ok(())
    }
}

impl<'b> Decode<'b, ()> for RejectReason {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        d.array()?;
        d.array()?;
        let era = d.u8()?;
        let errors = d.decode()?;

        Ok(RejectReason::EraErrors(era, errors))
    }
}

impl Encode<()> for RejectReason {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        match self {
            RejectReason::Plutus(s) => e
                .writer_mut()
                .write_all(s.as_bytes())
                .map_err(encode::Error::write)?,
            RejectReason::EraErrors(era, errors) => {
                e.array(1)?;
                e.array(2)?;
                e.u8(*era)?;
                e.encode(errors)?;
            }
        }
        Ok(())
    }
}

impl<'b, C> Decode<'b, C> for TxError {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut C) -> Result<Self, decode::Error> {
        match d.datatype()? {
            CborType::Array => d.array()?,
            CborType::U8 => {
                return Ok(TxError::U8(d.u8()?));
            }
            _ => {
                return Err(decode::Error::message("Unknown ledger error CBOR type"));
            }
        };

        use TxError::*;

        match d.u8()? {
            1 => Ok(ConwayUtxowFailure(d.decode()?)),
            2 => Ok(ConwayCertsFailure(d.decode()?)),
            3 => Ok(ConwayGovFailure(d.decode()?)),
            4 => Ok(ConwayWdrlNotDelegatedToDRep(d.decode()?)),
            5 => Ok(ConwayTreasuryValueMismatch(d.decode()?, d.decode()?)),
            6 => Ok(ConwayTxRefScriptsSizeTooBig(d.decode()?, d.decode()?)),
            7 => Ok(ConwayMempoolFailure(d.decode()?)),
            _ => Err(decode::Error::message("Unknown variant")),
        }
    }
}

impl<C> Encode<C> for TxError {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut C,
    ) -> Result<(), encode::Error<W::Error>> {
        match self {
            Self::ConwayUtxowFailure(inner) => {
                e.array(2)?;
                e.u16(0)?;
                e.encode(inner)?;
            }
            Self::ConwayCertsFailure(inner) => {
                e.array(2)?;
                e.u16(1)?;
                e.encode(inner)?;
            }
            Self::ConwayGovFailure(inner) => {
                e.array(2)?;
                e.u16(2)?;
                e.encode(inner)?;
            }
            Self::ConwayWdrlNotDelegatedToDRep(keys) => {
                e.array(2)?;
                e.u16(3)?;
                e.encode(keys)?;
            }
            Self::ConwayTreasuryValueMismatch(actual, submitted) => {
                e.array(3)?;
                e.u16(4)?;
                e.encode(actual)?;
                e.encode(submitted)?;
            }
            Self::ConwayTxRefScriptsSizeTooBig(computed_size, max_size) => {
                e.array(3)?;
                e.u16(5)?;
                e.encode(computed_size)?;
                e.encode(max_size)?;
            }
            Self::ConwayMempoolFailure(msg) => {
                e.array(2)?;
                e.u16(6)?;
                e.encode(msg)?;
            }
            Self::U8(x) => {
                e.u8(*x)?;
            }
        }
        Ok(())
    }
}

impl<'b, C> Decode<'b, C> for ConwayUtxoWPredFailure {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        d.array()?;
        let error = d.u16()?;

        use ConwayUtxoWPredFailure::*;

        match error {
            0 => Ok(UtxoFailure(d.decode_with(ctx)?)),
            1 => Ok(InvalidWitnessesUTXOW(d.decode_with(ctx)?)),
            2 => Ok(MissingVKeyWitnessesUTXOW(d.decode_with(ctx)?)),
            3 => Ok(MissingScriptWitnessesUTXOW(d.decode_with(ctx)?)),
            4 => Ok(ScriptWitnessNotValidatingUTXOW(d.decode_with(ctx)?)),
            5 => Ok(MissingTxBodyMetadataHash(d.decode_with(ctx)?)),
            6 => Ok(MissingTxMetadata(d.decode_with(ctx)?)),
            7 => Ok(ConflictingMetadataHash(
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
            )),
            8 => Ok(InvalidMetadata()),
            9 => Ok(ExtraneousScriptWitnessesUTXOW(d.decode_with(ctx)?)),
            10 => Ok(MissingRedeemers(d.decode_with(ctx)?)),
            11 => Ok(MissingRequiredDatums(
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
            )),
            12 => Ok(NotAllowedSupplementalDatums(
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
            )),
            13 => Ok(PPViewHashesDontMatch(
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
            )),
            14 => Ok(UnspendableUTxONoDatumHash(d.decode_with(ctx)?)),
            15 => Ok(ExtraRedeemers(d.decode_with(ctx)?)),
            16 => Ok(MalformedScriptWitnesses(d.decode_with(ctx)?)),
            17 => Ok(MalformedReferenceScripts(d.decode_with(ctx)?)),
            _ => Err(decode::Error::message(format!(
                "unknown error tag while decoding ConwayUtxoWPredFailure: {}",
                error
            ))),
        }
    }
}

impl Encode<()> for ConwayUtxoWPredFailure {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        use ConwayUtxoWPredFailure::*;
        match self {
            ExtraneousScriptWitnessesUTXOW(addrs) => {
                e.array(2)?;
                e.u8(9)?;
                e.encode(addrs)?;
            }
            MissingTxBodyMetadataHash(addr) => {
                e.array(2)?;
                e.u8(5)?;
                e.encode(addr)?;
            }
            NotAllowedSupplementalDatums(unall, accpt) => {
                e.array(3)?;
                e.u8(12)?;
                e.encode(unall)?;
                e.encode(accpt)?;
            }
            PPViewHashesDontMatch(body_hash, pp_hash) => {
                e.array(3)?;
                e.u8(13)?;
                e.encode(body_hash)?;
                e.encode(pp_hash)?;
            }
            ExtraRedeemers(purp) => {
                e.array(2)?;
                e.u8(15)?;
                e.encode(purp)?;
            }
            UtxoFailure(failure) => {
                e.array(2)?;
                e.u8(0)?;
                e.encode(failure)?;
            }
            _ => todo!(),
        }

        Ok(())
    }
}

impl<'b, C> Decode<'b, C> for UtxoFailure {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        d.array()?;

        use UtxoFailure::*;

        match d.u8()? {
            0 => Ok(UtxosFailure(d.decode_with(ctx)?)),
            1 => Ok(BadInputsUTxO(d.decode_with(ctx)?)),
            2 => Ok(OutsideValidityIntervalUTxO(
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
            )),
            3 => Ok(MaxTxSizeUTxO(d.decode_with(ctx)?, d.decode_with(ctx)?)),
            4 => Ok(InputSetEmptyUTxO),
            5 => Ok(FeeTooSmallUTxO(d.decode_with(ctx)?, d.decode_with(ctx)?)),
            6 => Ok(ValueNotConservedUTxO(
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
            )),
            7 => Ok(WrongNetwork(d.decode_with(ctx)?, d.decode_with(ctx)?)),
            12 => Ok(InsufficientCollateral(
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
            )),
            15 => Ok(CollateralContainsNonADA(d.decode_with(ctx)?)),
            18 => Ok(TooManyCollateralInputs(
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
            )),
            19 => Ok(NoCollateralInputs),
            20 => Ok(IncorrectTotalCollateralField(
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
            )),
            21 => Ok(BabbageOutputTooSmallUTxO(d.decode_with(ctx)?)),
            _ => Err(decode::Error::message("Unknown `UtxoFailure` variant")),
        }
    }
}
impl<C> Encode<C> for UtxoFailure {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), encode::Error<W::Error>> {
        todo!("encoder for UtxoFailure");
        Ok(())
    }
}
impl<'b, C> Decode<'b, C> for UtxosFailure {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        d.array()?;

        match d.u8()? {
            0 => Ok(UtxosFailure::ValidationTagMismatch(
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
            )),
            1 => Ok(UtxosFailure::CollectErrors(d.decode_with(ctx)?)),
            _ => Err(decode::Error::message("Unknown `UtxosFailure` variant")),
        }
    }
}

impl Encode<()> for UtxosFailure {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        match self {
            UtxosFailure::ValidationTagMismatch(isv, tmd) => {
                e.array(3)?;
                e.u8(0)?;
                e.encode(isv)?;
                e.encode(tmd)?;
            }
            UtxosFailure::CollectErrors(c) => {
                e.array(2)?;
                e.u8(1)?;
                e.encode(c)?;
            }
        }

        Ok(())
    }
}

impl<'b, C> decode::Decode<'b, C> for CollectError {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut C) -> Result<Self, decode::Error> {
        d.array()?;
        match d.u16()? {
            0 => Ok(Self::NoRedeemer(d.decode()?)),
            1 => Ok(Self::NoWitness(d.decode()?)),
            2 => Ok(Self::NoCostModel(d.decode()?)),
            3 => Ok(Self::BadTranslation(d.decode()?)),
            _ => Err(decode::Error::message("Unknown variant")),
        }
    }
}

impl<C> encode::Encode<C> for CollectError {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut C,
    ) -> Result<(), encode::Error<W::Error>> {
        match self {
            Self::NoRedeemer(item) => {
                e.array(2)?;
                e.u16(0)?;
                e.encode(item)?;
            }
            Self::NoWitness(bytes) => {
                e.array(2)?;
                e.u16(1)?;
                e.encode(bytes)?;
            }
            Self::NoCostModel(language) => {
                e.array(2)?;
                e.u16(2)?;
                e.encode(language)?;
            }
            Self::BadTranslation(error) => {
                e.array(2)?;
                e.u16(3)?;
                e.encode(error)?;
            }
        }
        Ok(())
    }
}

impl<'b, C> Decode<'b, C> for TagMismatchDescription {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        d.array()?;

        match d.u8()? {
            0 => Ok(TagMismatchDescription::PassedUnexpectedly),
            1 => Ok(TagMismatchDescription::FailedUnexpectedly(
                d.decode_with(ctx)?,
            )),
            _ => Err(decode::Error::message(
                "Unknown `TagMismatchDescription` variant",
            )),
        }
    }
}

impl Encode<()> for TagMismatchDescription {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        match self {
            TagMismatchDescription::PassedUnexpectedly => {
                e.array(1)?;
                e.u8(0)?;
            }
            TagMismatchDescription::FailedUnexpectedly(c) => {
                e.array(2)?;
                e.u8(1)?;
                e.encode(c)?;
            }
        }

        Ok(())
    }
}

impl<'b, T0, T1, T2, T3, T4, T5, Ctx> Decode<'b, Ctx> for PlutusPurpose<T0, T1, T2, T3, T4, T5>
where
    T0: Decode<'b, ()>,
    T1: Decode<'b, ()>,
    T2: Decode<'b, ()>,
    T3: Decode<'b, ()>,
    T4: Decode<'b, ()>,
    T5: Decode<'b, ()>,
{
    fn decode(d: &mut Decoder<'b>, _ctx: &mut Ctx) -> Result<Self, decode::Error> {
        d.array()?;
        d.array()?;

        use PlutusPurpose::*;

        match d.u8()? {
            0 => Ok(Spending(d.decode()?)),
            1 => Ok(Minting(d.decode()?)),
            2 => Ok(Certifying(d.decode()?)),
            3 => Ok(Rewarding(d.decode()?)),
            4 => Ok(Voting(d.decode()?)),
            5 => Ok(Proposing(d.decode()?)),
            _ => Err(decode::Error::message("Unknown `PlutusPurpose` variant")),
        }
    }
}

impl<T0, T1, T2, T3, T4, T5, Ctx> Encode<Ctx> for PlutusPurpose<T0, T1, T2, T3, T4, T5>
where
    T0: Encode<()>,
    T1: Encode<()>,
    T2: Encode<()>,
    T3: Encode<()>,
    T4: Encode<()>,
    T5: Encode<()>,
{
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut Ctx,
    ) -> Result<(), encode::Error<W::Error>> {
        e.array(1)?;
        e.array(2)?;
        e.u8(self.ord())?;

        use PlutusPurpose::*;

        match self {
            Spending(x) => e.encode(x)?,
            Minting(x) => e.encode(x)?,
            Certifying(x) => e.encode(x)?,
            Rewarding(x) => e.encode(x)?,
            Voting(x) => e.encode(x)?,
            Proposing(x) => e.encode(x)?,
        };

        Ok(())
    }
}

macro_rules! decode_err {
    ($msg:expr) => {
        return Err(decode::Error::message($msg))
    };
}

impl<'b, C> Decode<'b, C> for ShelleyPoolPredFailure {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        d.array()?;
        let tag = d.u16()?;

        use ShelleyPoolPredFailure::*;
        match tag {
            0 => Ok(StakePoolNotRegisteredOnKeyPOOL(d.decode_with(ctx)?)),
            1 => {
                let gt_expected: EpochNo = d.decode_with(ctx)?;
                let lt_supplied: EpochNo = d.decode_with(ctx)?;
                let lt_expected: EpochNo = d.decode_with(ctx)?;

                Ok(StakePoolRetirementWrongEpochPOOL(
                    Mismatch(lt_supplied.clone(), gt_expected),
                    Mismatch(lt_supplied, lt_expected),
                ))
            }
            3 => Ok(StakePoolCostTooLowPOOL(d.decode_with(ctx)?)),
            4 => {
                let expected: Network = d.decode_with(ctx)?;
                let supplied: Network = d.decode_with(ctx)?;

                Ok(WrongNetworkPOOL(
                    Mismatch(supplied, expected),
                    d.decode_with(ctx)?,
                ))
            }
            5 => Ok(PoolMedataHashTooBig(
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
            )),
            _ => Err(decode::Error::message(format!(
                "unknown error tag while decoding ShelleyPoolPredFailure: {}",
                tag
            ))),
        }
    }
}

impl<'b, T, C> Decode<'b, C> for Mismatch<T>
where
    T: Decode<'b, C>,
{
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        match d.decode_with(ctx) {
            Ok(mis1) => match d.decode_with(ctx) {
                Ok(mis2) => Ok(Mismatch(mis1, mis2)),
                Err(e) => Err(e),
            },
            Err(e) => Err(e),
        }
    }
}

impl<T, C> Encode<C> for Mismatch<T>
where
    T: Encode<C>,
{
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), encode::Error<W::Error>> {
        e.encode_with(&self.0, ctx)?;
        e.encode_with(&self.1, ctx)?;
        Ok(())
    }
}

impl<'b, C> Decode<'b, C> for ConwayCertsPredFailure {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        d.array()?;
        let error = d.u16()?;

        use ConwayCertsPredFailure::*;

        match error {
            0 => Ok(WithdrawalsNotInRewardsCERTS(d.decode_with(ctx)?)),
            1 => Ok(CertFailure(d.decode_with(ctx)?)),
            _ => Err(decode::Error::message(format!(
                "unknown error tag while decoding ConwayCertsPredFailure: {}",
                error
            ))),
        }
    }
}
impl<C> Encode<C> for ConwayCertsPredFailure {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut C,
    ) -> Result<(), encode::Error<W::Error>> {
        use ConwayCertsPredFailure::*;

        match self {
            WithdrawalsNotInRewardsCERTS(addr) => {
                e.array(2)?;
                e.u16(0)?;
                e.encode(addr)?;
            }
            CertFailure(failure) => {
                e.array(2)?;
                e.u16(1)?;
                e.encode(failure)?;
            }
        }

        Ok(())
    }
}

impl<'b, C, K: pallas_codec::minicbor::Decode<'b, C>, V: pallas_codec::minicbor::Decode<'b, C>>
    Decode<'b, C> for OHashMap<K, V>
{
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        let v: Result<Vec<(K, V)>, _> = d.map_iter_with::<C, K, V>(ctx)?.collect();

        Ok(OHashMap(v?))
    }
}

impl<C, K: pallas_codec::minicbor::Encode<()>, V: pallas_codec::minicbor::Encode<()>> Encode<C>
    for OHashMap<K, V>
{
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut C,
    ) -> Result<(), encode::Error<W::Error>> {
        e.map(self.0.len() as u64)?;
        e.encode(&self.0);

        Ok(())
    }
}

impl<'b, C> Decode<'b, C> for ConwayContextError {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        d.array()?;
        let error = d.u16()?;

        use ConwayContextError::*;

        match error {
            8 => Ok(BabbageContextError(d.decode_with(ctx)?)),

            9 => Ok(CertificateNotSupported(d.decode_with(ctx)?)),

            10 => Ok(PlutusPurposeNotSupported(d.decode_with(ctx)?)),
            11 => Ok(CurrentTreasuryFieldNotSupported(d.decode_with(ctx)?)),
            12 => Ok(VotingProceduresFieldNotSupported(d.decode_with(ctx)?)),
            13 => Ok(ProposalProceduresFieldNotSupported(d.decode_with(ctx)?)),
            14 => Ok(TreasuryDonationFieldNotSupported(d.decode_with(ctx)?)),

            _ => Err(decode::Error::message(format!(
                "unknown error tag while decoding CollectError: {}",
                error
            ))),
        }
    }
}

impl<C> Encode<C> for ConwayContextError {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), encode::Error<W::Error>> {
        use ConwayContextError::*;

        match self {
            BabbageContextError(inner) => {
                e.array(2)?;
                e.u16(8)?;
                e.encode_with(inner, ctx)?;
            }
            CertificateNotSupported(cert) => {
                e.array(2)?;
                e.u16(9)?;
                e.encode_with(cert, ctx)?;
            }
            PlutusPurposeNotSupported(purpose) => {
                e.array(2)?;
                e.u16(10)?;
                e.encode_with(purpose, ctx)?;
            }
            CurrentTreasuryFieldNotSupported(field) => {
                e.array(2)?;
                e.u16(11)?;
                e.encode_with(field, ctx)?;
            }
            VotingProceduresFieldNotSupported(field) => {
                e.array(2)?;
                e.u16(12)?;
                e.encode_with(field, ctx)?;
            }
            ProposalProceduresFieldNotSupported(field) => {
                e.array(2)?;
                e.u16(13)?;
                e.encode_with(field, ctx)?;
            }
            TreasuryDonationFieldNotSupported(field) => {
                e.array(2)?;
                e.u16(14)?;
                e.encode_with(field, ctx)?;
            }
        }
        Ok(())
    }
}

impl<'b, C> Decode<'b, C> for BabbageContextError {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        d.array()?;
        let error = d.u16()?;

        use BabbageContextError::*;

        match error {
            0 => Ok(ByronTxOutInContext(d.decode_with(ctx)?)),
            1 => Ok(AlonzoMissingInput(d.decode_with(ctx)?)),
            2 => Ok(RedeemerPointerPointsToNothing(d.decode_with(ctx)?)),
            4 => Ok(InlineDatumsNotSupported(d.decode_with(ctx)?)),
            5 => Ok(ReferenceScriptsNotSupported(d.decode_with(ctx)?)),
            6 => Ok(ReferenceInputsNotSupported(d.decode_with(ctx)?)),
            7 => Ok(AlonzoTimeTranslationPastHorizon(d.decode_with(ctx)?)),
            _ => Err(decode::Error::message(format!(
                "unknown error tag while decoding BabbageContextError: {}",
                error
            ))),
        }
    }
}

impl<C> Encode<C> for BabbageContextError {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), encode::Error<W::Error>> {
        match self {
            BabbageContextError::ByronTxOutInContext(inner) => {
                e.array(2)?;
                e.u16(0)?;
                e.encode_with(inner, ctx)?;
            }
            BabbageContextError::AlonzoMissingInput(inner) => {
                e.array(2)?;
                e.u16(1)?;
                e.encode_with(inner, ctx)?;
            }
            BabbageContextError::RedeemerPointerPointsToNothing(inner) => {
                e.array(2)?;
                e.u16(2)?;
                e.encode_with(inner, ctx)?;
            }
            BabbageContextError::InlineDatumsNotSupported(inner) => {
                e.array(2)?;
                e.u16(4)?;
                e.encode_with(inner, ctx)?;
            }
            BabbageContextError::ReferenceScriptsNotSupported(inner) => {
                e.array(2)?;
                e.u16(5)?;
                e.encode_with(inner, ctx)?;
            }
            BabbageContextError::ReferenceInputsNotSupported(inner) => {
                e.array(2)?;
                e.u16(6)?;
                e.encode_with(inner, ctx)?;
            }
            BabbageContextError::AlonzoTimeTranslationPastHorizon(inner) => {
                e.array(2)?;
                e.u16(7)?;
                e.encode_with(inner, ctx)?;
            }
        }
        Ok(())
    }
}
impl<'b, C> Decode<'b, C> for TxOutSource {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        d.array()?;
        let error = d.u16()?;

        use TxOutSource::*;

        match error {
            0 => Ok(TxOutFromInput(d.decode_with(ctx)?)),
            1 => Ok(TxOutFromOutput(d.decode_with(ctx)?)),

            _ => Err(decode::Error::message(format!(
                "unknown error tag while decoding TxOutSource: {}",
                error
            ))),
        }
    }
}

impl<C> Encode<C> for TxOutSource {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), encode::Error<W::Error>> {
        match self {
            TxOutSource::TxOutFromInput(inner) => {
                e.array(2)?;
                e.u16(0)?;
                e.encode_with(inner, ctx)?;
            }
            TxOutSource::TxOutFromOutput(inner) => {
                e.array(2)?;
                e.u16(1)?;
                e.encode_with(inner, ctx)?;
            }
        }
        Ok(())
    }
}
impl<'b, C> Decode<'b, C> for ConwayPlutusPurpose {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        d.array()?;
        let error = d.u16()?;

        use ConwayPlutusPurpose::*;

        match error {
            0 => Ok(ConwaySpending(d.decode_with(ctx)?)),
            1 => Ok(ConwayMinting(d.decode_with(ctx)?)),
            2 => Ok(ConwayCertifying(d.decode_with(ctx)?)),
            3 => Ok(ConwayRewarding(d.decode_with(ctx)?)),
            4 => Ok(ConwayVoting(d.decode_with(ctx)?)),
            5 => Ok(ConwayProposing(d.decode_with(ctx)?)),
            _ => Err(decode::Error::message(format!(
                "unknown error tag while decoding ConwayPlutusPurpose: {}",
                error
            ))),
        }
    }
}

impl<'b, C> Decode<'b, C> for ConwayCertPredFailure {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        d.array()?;
        let error = d.u16()?;

        use ConwayCertPredFailure::*;

        match error {
            1 => Ok(DelegFailure(d.decode_with(ctx)?)),
            2 => Ok(PoolFailure(d.decode_with(ctx)?)),
            3 => Ok(GovCertFailure(d.decode_with(ctx)?)),
            _ => Err(decode::Error::message(format!(
                "unknown error tag while decoding ConwayCertPredFailure: {}",
                error
            ))),
        }
    }
}

impl<'b, C> Decode<'b, C> for ConwayGovCertPredFailure {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        d.array()?;
        let error = d.u16()?;

        use ConwayGovCertPredFailure::*;

        match error {
            0 => Ok(ConwayDRepAlreadyRegistered(d.decode_with(ctx)?)),
            1 => Ok(ConwayDRepNotRegistered(d.decode_with(ctx)?)),
            2 => Ok(ConwayDRepIncorrectDeposit(
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
            )),
            3 => Ok(ConwayCommitteeHasPreviouslyResigned(d.decode_with(ctx)?)),
            4 => Ok(ConwayDRepIncorrectRefund(
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
            )),
            5 => Ok(ConwayCommitteeIsUnknown(d.decode_with(ctx)?)),
            _ => Err(decode::Error::message(format!(
                "unknown error tag while decoding ConwayGovCertPredFailure: {}",
                error
            ))),
        }
    }
}

impl<'b, C> Decode<'b, C> for ConwayDelegPredFailure {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        d.array()?;
        let error = d.u16()?;

        use ConwayDelegPredFailure::*;

        match error {
            1 => Ok(IncorrectDepositDELEG(d.decode_with(ctx)?)),
            2 => Ok(StakeKeyRegisteredDELEG(d.decode_with(ctx)?)),
            3 => Ok(StakeKeyNotRegisteredDELEG(d.decode_with(ctx)?)),
            4 => Ok(StakeKeyHasNonZeroRewardAccountBalanceDELEG(
                d.decode_with(ctx)?,
            )),
            5 => Ok(DelegateeDRepNotRegisteredDELEG(d.decode_with(ctx)?)),
            6 => Ok(DelegateeStakePoolNotRegisteredDELEG(d.decode_with(ctx)?)),
            _ => Err(decode::Error::message(format!(
                "unknown error code while decoding ConwayDelegPredFailure: {}",
                error
            ))),
        }
    }
}

impl<'b, C> Decode<'b, C> for ConwayTxCert {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        let pos = d.position();
        d.array()?;
        let variant = d.u16()?;

        d.set_position(pos);
        let cert: Certificate = d.decode_with(ctx)?;

        match variant {
            // shelley deleg certificates
            0..3 => Ok(ConwayTxCert::ConwayTxCertDeleg(cert)),
            // pool certificates
            3..5 => Ok(ConwayTxCert::ConwayTxCertPool(cert)),
            // conway deleg certificates
            5 => decode_err!("Genesis delegation certificates are no longer supported"),
            6 => decode_err!("MIR certificates are no longer supported"),
            7..14 => Ok(ConwayTxCert::ConwayTxCertDeleg(cert)),
            14..19 => Ok(ConwayTxCert::ConwayTxCertGov(cert)),
            _ => Err(decode::Error::message(format!(
                "unknown certificate variant while decoding ConwayTxCert: {}",
                variant
            ))),
        }
    }
}

impl<C> Encode<C> for ConwayTxCert {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), encode::Error<W::Error>> {
        match self {
            ConwayTxCert::ConwayTxCertDeleg(cert) => {
                e.encode_with(cert, ctx)?;
            }
            ConwayTxCert::ConwayTxCertPool(cert) => {
                e.encode_with(cert, ctx)?;
            }
            ConwayTxCert::ConwayTxCertGov(cert) => {
                e.encode_with(cert, ctx)?;
            }
        }
        Ok(())
    }
}

impl<'b, C> Decode<'b, C> for FailureDescription {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        d.array()?;
        let error = d.u16()?;

        use FailureDescription::*;

        match error {
            1 => Ok(PlutusFailure(d.decode_with(ctx)?, d.decode_with(ctx)?)),
            _ => Err(decode::Error::message(format!(
                "unknown error tag while decoding FailureDescription: {}",
                error
            ))),
        }
    }
}

impl<C> Encode<C> for FailureDescription {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), encode::Error<W::Error>> {
        match self {
            FailureDescription::PlutusFailure(purpose, err) => {
                e.array(3)?;
                e.u16(1)?;
                e.encode_with(purpose, ctx)?;
                e.encode_with(err, ctx)?;
            }
        }
        Ok(())
    }
}
impl<'b, C> Decode<'b, C> for Credential {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        d.array()?;
        let tag = d.u16()?;

        use Credential::*;

        match tag {
            0 => Ok(KeyHashObj(d.decode_with(ctx)?)),
            1 => Ok(ScriptHashObj(d.decode_with(ctx)?)),
            _ => Err(decode::Error::message(format!(
                "unknown tag while decoding Credential: {}",
                tag
            ))),
        }
    }
}

impl<C> Encode<C> for Credential {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), encode::Error<W::Error>> {
        match self {
            Credential::KeyHashObj(hash) => {
                e.array(2)?;
                e.u16(0)?;
                e.encode_with(hash, ctx)?;
            }
            Credential::ScriptHashObj(hash) => {
                e.array(2)?;
                e.u16(1)?;
                e.encode_with(hash, ctx)?;
            }
        }
        Ok(())
    }
}

impl<'b, C> Decode<'b, C> for ShelleyBasedEra {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut C) -> Result<Self, decode::Error> {
        d.array()?;
        let era = d.u16()?;

        use ShelleyBasedEra::*;

        match era {
            1 => Ok(ShelleyBasedEraShelley),
            2 => Ok(ShelleyBasedEraAllegra),
            3 => Ok(ShelleyBasedEraMary),
            4 => Ok(ShelleyBasedEraAlonzo),
            5 => Ok(ShelleyBasedEraBabbage),
            6 => Ok(ShelleyBasedEraConway),
            _ => Err(decode::Error::message(format!(
                "unknown era while decoding ShelleyBasedEra: {}",
                era
            ))),
        }
    }
}

impl<'b, C> Decode<'b, C> for Utxo {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        let tx_vec = d.decode_with(ctx)?;
        Ok(Utxo(tx_vec))
    }
}

#[cfg(test)]
mod tests {
    use pallas_codec::{
        minicbor::{self, encode},
        Fragment,
    };

    use crate::miniprotocols::localtxsubmission::{EraTx, Message, RejectReason};
    use crate::multiplexer::Error;

    #[test]
    fn decode_reject_message() {
        let reason = decode_error("8182068183051a000a9c7c1a000f37b5");
        println!("Reject reason: {:?}", reason);
    }

    #[test]
    fn decode_reject_message_001() {
        let reason = decode_error("81820681820482581cce3e13a895ca723a377b52b880ede8d822b34fc497f7add6df50d8d4581ce1c21d6ecb67e54622923d5db2a600a9a53f6a5eec1878a0118838e0");
        println!("Reject reason: {:?}", reason);
        assert!(true);
    }

    fn decode_error(cbor: &str) -> RejectReason {
        let mut cbor_bytes = hex::decode(cbor).unwrap();
        let mut decoder = minicbor::Decoder::new(&mut cbor_bytes);

        match decoder.decode::<RejectReason>() {
            Ok(reason) => reason,
            Err(e) => panic!("Error decoding: {:?}", e),
        }
    }

    fn try_decode_message<M>(buffer: &mut Vec<u8>) -> Result<Option<M>, Error>
    where
        M: Fragment,
    {
        let mut decoder = minicbor::Decoder::new(buffer);
        let maybe_msg = decoder.decode();

        match maybe_msg {
            Ok(msg) => {
                let pos = decoder.position();
                buffer.drain(0..pos);
                Ok(Some(msg))
            }
            Err(err) if err.is_end_of_input() => Ok(None),
            Err(err) => Err(Error::Decoding(err.to_string())),
        }
    }

    #[test]
    fn decode_reject_string_message() {
        let mut bytes = hex::decode(RAW_REJECT_REPONSE_ERROR_STRING).unwrap();
        let msg_res = try_decode_message::<Message<EraTx, RejectReason>>(&mut bytes);
        println!("Result: {:?}", msg_res);
        assert!(msg_res.is_ok())
    }

    fn decode_reject_reason(reject: &str) {
        let bytes = hex::decode(reject).unwrap();
        let msg_res = try_decode_message::<Message<EraTx, RejectReason>>(&mut bytes.clone());
        println!("Result: {:02x?}", msg_res);
        assert!(msg_res.is_ok());
        let mut datum: Vec<u8> = Vec::new();
        // Encoding back
        encode(msg_res.unwrap().unwrap(), &mut datum).expect("Error encoding");
        assert_eq!(bytes, datum);
    }

    #[test]
    fn round_trip_codec() {
        decode_reject_reason(RAW_REJECT_RESPONSE_CONWAY);
        decode_reject_reason(ISVALID_REJECT_PREVIEW);
        decode_reject_reason(MISSING_METADATA_HASH);
        decode_reject_reason(INPUT_SET_EMPTY_FEE_OUTPUT_SMALL_PREVIEW);
        decode_reject_reason(MAX_TX_SIZE_PREVIEW);
    }

    const RAW_REJECT_RESPONSE: &str =
        "82028182059f820082018200820a81581c3b890fb5449baedf5342a48ee9c9ec6acbc995641be92ad21f08c686\
        8200820183038158202628ce6ff8cc7ff0922072d930e4a693c17f991748dedece0be64819a2f9ef7782582031d\
        54ce8d7e8cb262fc891282f44e9d24c3902dc38fac63fd469e8bf3006376b5820750852fdaf0f2dd724291ce007\
        b8e76d74bcf28076ed0c494cd90c0cfe1c9ca582008201820782000000018200820183048158201a547638b4cf4\
        a3cec386e2f898ac6bc987fadd04277e1d3c8dab5c505a5674e8158201457e4107607f83a80c3c4ffeb70910c2b\
        a3a35cf1699a2a7375f50fcc54a931820082028201830500821a00636185a2581c6f1a1f0c7ccf632cc9ff4b796\
        87ed13ffe5b624cce288b364ebdce50a144414749581b000000032a9f8800581c795ecedb09821cb922c13060c8\
        f6377c3344fa7692551e865d86ac5da158205399c766fb7c494cddb2f7ae53cc01285474388757bc05bd575c14a\
        713a432a901820082028201820085825820497fe6401e25733c073c01164c7f2a1a05de8c95e36580f9d1b05123\
        70040def028258207911ba2b7d91ac56b05ea351282589fe30f4717a707a1b9defaf282afe5ba44200825820791\
        1ba2b7d91ac56b05ea351282589fe30f4717a707a1b9defaf282afe5ba44201825820869bcb6f35e6b7912c25e5\
        cb33fb9906b097980a83f2b8ef40b51c4ef52eccd402825820efc267ad2c15c34a117535eecc877241ed836eb3e\
        643ec90de21ca1b12fd79c20282008202820181148200820283023a000f0f6d1a004944ce820082028201830d3a\
        000f0f6d1a00106253820082028201830182811a02409e10811a024138c01a0255e528ff";

    const RAW_REJECT_REPONSE_ERROR_STRING: &str =
        "6867475972786f4141794e6847514d734151455a412b675a416a734141526b4436426c65635151424751506f47\
        4341614141484b64686b6f3677515a576467595a426c5a3242686b47566e594747515a576467595a426c5a32426\
        86b47566e59474751595a42686b47566e594747515a5446455949426f4141717a364743415a7456454547674144\
        5978555a4166384141526f4141567731474341614141655864526b3239415143476741432f35516141416271654\
        26a63414145424751506f47572f324241496141414f3943426f414130374647443442476741514c67385a4d536f\
        42476741444c6f415a4161554247674143326e675a412b675a7a775942476741424f6a515949426d6f385267674\
        751506f47434161414145367241455a34554d454751506f43686f414177495a474a77424767414441686b596e41\
        456141414d6766426b423251456141414d7741426b422f77455a7a504d5949426e395142676747662f564743415\
        a5742345949426c4173786767476741424b74385949426f4141762b5547674147366e67593341414241526f4141\
        512b534753326e4141455a3672735949426f4141762b5547674147366e67593341414241526f4141762b5547674\
        147366e67593341414241526f4145624973476741462f64344141686f414446424f4758635342426f4148577232\
        47674142516c73454767414544475941424141614141465071786767476741444932455a4179774241526d67336\
        86767476741445058595949426c353942676747582b344743415a7156305949426c3939786767475a5771474341\
        6141694f737a416f61413354326b786c4b48776f61416c466568426d417377714347674149466c41614364577a5\
        1466b452f466b452b514541414449794d6a49794d6a49794d6a49794d6a49794d6a49794d6a49794d69496c4d7a\
        41554d6a49794d6a49794d6a49794d6a49794d6a49794d6a49794d6a49794d6a49794d6a49794d6c4d7a41774d3\
        3447041424142435a47526b706d59475a6d3464544d774d7a4e773575744d4451774e5144306741425341414649\
        414a494141564d774a444e77356d425341555947674469514151715a6753475a455a75764e30356763414247366\
        3774f414154413041454d44514167564d774a444e77356d59475245536d5a675941416941454a6d41475a754141\
        435341434d44674146494141424e494151564d774a444e784a75744d4451774e5146674168557a416b4142457a4\
        d7949694d33456d62677a4e77526d34497a4174414f4144414341424d33414762676a4d433041344152494e4150\
        4d3342414241416d626741426b67416a413041654d4451423033576d426f417362725441304162457a496a4d6a4\
        131496c4d7a41794142464b41715a6d42775a75764d446b4145414d556f69594152676441416d366b4145414933\
        5747426f5a4742735947786762474273594777414a6761674b473634774e4147464d794d774d77415253695a475\
        26b706d424d5a75504e31786762474275414562726a41324d44634145544e783575754d44594149335847427341\
        435947344535676241416d426d41344a6d5a6d52455247526b5a754a4d33416d6267544170414e4e316f414a6d3\
        44d7a6345414b4149414759464943616d5a676347526b706d42575a75504e317867646d4234414562726a41374d\
        44774145544e783575754d447341493358474232414359486746686764674443627141425241424e31435141426\
        75a674f6d3630774d77465141546461594759444a75744d444d4268544d774d5449794d6c4d774a544e78357575\
        4d4455774e67416a646359477067624141695a75504e317867616742473634774e5141544132416d4d445541457\
        74d6747784d33426d6267674154646159475143356d344533576d426b4175627254417941594541457a63435a67\
        54414647426941305a6754414347426941304c47426d4145594651414a75714d4334774c77475464575a4742635\
        94635675941416d4261594677414a675841416d59453575744d43734163416f33566d5267566d425959466f414a\
        67564742574143594659414a6d4249627254416f414641484d7949794d6a4a544d774b7a49794d6c4d7a41754d3\
        3447041424142435a47526b706d59474a6d34644941414149556f435a75764e30344168756e41424d4451414977\
        4b7741546455414f4a6b5a47536d5a67596d626830674167416853674a6d3638335467434736634145774e41416\
        a417241424e3151413567596742474251414362716741524143466a49794d6a4979557a4d43387a634f6b414141\
        454a6b5a47526b706d59475a6d346449414141495449794d6a4a544d774e7a4e77365141514151734a6d3656494\
        1414145774f67416a417841424e3151414a676141416978676241424742614143627167415441774142457a644b\
        6b414542555947514152675567416d366f4145774c4441744d43344145774b7a417441464e315a6b5a47536d5a6\
        75747626830674167416859564d7a41734d33486d3634774c51415142684d4330774c6a41764148466a41764143\
        4d43594145335641416d526756474259414359464a675667426d3634774a77437a416e414b4d77496a646159457\
        741494168675441416d424b414359456f434275734d434941493357474243414559454a675167416d4243594434\
        4252675067416d4138414359446f414a674f41416d41324143594451414a674d67416d417741435944414168674\
        c6741696b7773536d5a67466741696b41414a6d59434a6d3638774454415341424e3149417875744d425577456a\
        6457594370674a414170414145526d59434941514149415970514d33537041414759414a757041434d774154645\
        341454151726f45695141694d6a4d774241417a6463594277414a75754d413477447741544150414249694d7a4d\
        4151414a4941416a4d7a414641435341416461627177415141794d4149335567416b52455a674645536d5a67446\
        74169414b4b6d5a67476d62727a414a4d4134414541595441454d424577446741524d414977447741514156567a\
        3658726756584f6b536d5a67436d626941416b67414259544d414d41494145774153496c4d7a41464d334467424\
        a41414359417741496d59415a6d3445414a49414977427741534d6a4143497a414341434142497741694d774167\
        416741566330726f56644552674247366f4146566338474432486d6632486d665145442f32486d6657427950377\
        9303042345a5a53547a68596162482b3653316176373668545570616c644439705748524546425245482f32486d\
        66574279694c7235587846304c3437704c363870616e5568337443312f32484c7a313042425436456b544546425\
        245466651555242583035475650385a412b5562414256704b5a4c365955776241574e466546324b414142594848\
        6b6743687a624c72495933354279415a653538786c3365776836586b464d693035332b4b2f59655a3959484b303\
        465644c505031447a4441647969454d6e77445879736a4d4769693351475346574e62722f476773764b4d416141\
        58764a4d502f59655a3842414145412f3968356e3968356e352f59655a2f59655a2f59655a39594942364f54504\
        845657a426d5249524448705765462b4d69394961367935426b564665434675786155714d522f77442f32486d66\
        32486d66324871665742776d474f6c4d32775a354c7757756d78374869774978394c66304956736254505575593\
        04c652f39683667502b68514b4641476774734d63445965352f59655a2f59";

    const RAW_REJECT_RESPONSE_CONWAY: &str =
        "82028182068a82018209d9010281581ca55f409501bf65805bb0dc76f6f9ae90b61e19ed870bc0025681360882\
         01830cd901028158203e8c4b1d396bb8132e5097f5a2f012d97900cbc496a3745db4226cea4cb66465d9010280\
         8201820f818200008201830d815820b8f025288ba73aed0fe31fad243c58bef276caf20e70ade9d343bbed62b5\
         fdc08158200f9a8c36fd5205f371efa8a251e0c27c6d944afa837ac2a7ae0776c51a6372cd82018200830700d9\
         01028458390170e60f3b5ea7153e0acc7a803e4401d44b8ed1bae1c7baaad1a62a721e78aae7c90cc36d624f7b\
         3bb6d86b52696dc84e490f343eba89005f583901b7469fffd8657fdc71bfcc2368abf070025f8ed3b2d07edf42\
         11383c9e2efeca24440f4ef0d0718ed066b0d0928af76584eb87a3b7fe2549583901c7f913cb1a0d62a1dbd9eb\
         0c5fb4d5b9ff1ed370fbb6b2dbe98c4b82d3e62702c687b1d0d010136220d0a389b590f2e98399502a2bfc4b2b\
         583901f1e126304308006938d2e8571842ff87302fff95a037b3fd838451b8b3c9396d0680d912487139cb7fc8\
         5aa279ea70e8cdacee4c6cae40fd82018200830600821a0198de8ca1581c787f0c946b98153500edc0a753e654\
         57250544da8486b17c85708135a15818506572666563744c6567656e64617279446572705365616c0182018200\
         8201d9010283825820459763315cb9af2ecd9003a4236aacae4ec4777df7f4f757b5b0187a32eca90700825820\
         df92937f762ae2f0afcb9829c3ef514635ac0d9975750225519cdb071e1cff9201825820ff4f36b81327cfb4b1\
         7c958b790447c4734fe39c8741dd1f38ace0fd54fcf2fc0182018200811382018200830c001a000fbd72820182\
         00830282811a044f777c811a044f858c1a04ace388";

    const ISVALID_REJECT_PREVIEW: &str = "8202818206818201820082008300f48100";

    const MISSING_METADATA_HASH: &str =
        "82028182068182018205582059182929bdbb6e212a80e65564a1c21a3ffae38dc99b9dc2b6f4184b12dd2b8c";

    const INPUT_SET_EMPTY_FEE_OUTPUT_SMALL_PREVIEW: &str =
        "8202818206858201820082158182825839004464b02c100eb32bc337ffbe0ce79fcf80cced5beb1b672ed8a58d\
         776ed48e025b63487772f02996258396adcfaacf402de624980949eaa9011a000e88be82018200830600018201\
         820083051a00027e3d00820182008104820182008302828081011a041f9b1b";

    const MAX_TX_SIZE_PREVIEW: &str = "82028182068182018200830319405f194000";
}
