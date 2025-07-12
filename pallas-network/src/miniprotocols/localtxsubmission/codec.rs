use pallas_codec::minicbor::data::{IanaTag, Tag};
use pallas_codec::minicbor::{decode, encode, Decode, Decoder, Encode, Encoder};

use crate::miniprotocols::localstate::queries_v16::{
    BigInt, BoundedBytes, Constr, DatumOption, PlutusData,
};
use crate::miniprotocols::localtxsubmission::primitives::Voter;
use crate::miniprotocols::localtxsubmission::{
    ConwayLedgerFailure, EpochNo, EraTx, Message, Mismatch, Network, PlutusPurpose, SMaybe,
    ShelleyPoolPredFailure,
};

use std::ops::Deref;
use std::str::from_utf8;

use super::primitives::{Certificate, NativeScript, PseudoScript};
use super::{ApplyTxError, ConwayTxCert, OHashMap, TxValidationError, Utxo, VotingProcedure};

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

impl<'b, C, Tx: Decode<'b, C>, Reject: Decode<'b, C> + From<String>> Decode<'b, C>
    for Message<Tx, Reject>
{
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
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
            0 => Ok(Message::SubmitTx(d.decode_with(ctx)?)),
            1 => Ok(Message::AcceptTx),
            2 => Ok(Message::RejectTx(d.decode_with(ctx)?)),
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

impl<'b, C> Decode<'b, C> for TxValidationError {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        d.array()?;
        d.array()?;
        let era = d.decode_with(ctx)?;
        let error = d.decode_with(ctx)?;
        Ok(TxValidationError::ShelleyTxValidationError { error, era })
    }
}

impl<C> Encode<C> for TxValidationError {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), encode::Error<W::Error>> {
        match self {
            TxValidationError::ShelleyTxValidationError { error, era } => {
                e.array(2)?;
                e.encode_with(era, ctx)?;
                e.encode_with(error, ctx)?;
            }
            TxValidationError::ByronTxValidationError { error: _ } => todo!(),
            TxValidationError::Plutus(_) => todo!(),
        }
        Ok(())
    }
}

impl<'b, C> Decode<'b, C> for ApplyTxError {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        let errors = d.array_iter_with::<C, ConwayLedgerFailure>(ctx)?.collect();

        match errors {
            Ok(errors) => Ok(ApplyTxError(errors)),
            Err(error) => Err(error),
        }
    }
}

impl<C> Encode<C> for ApplyTxError {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), encode::Error<W::Error>> {
        e.array(self.0.len() as u64)?;
        for error in &self.0 {
            e.encode_with(error, ctx)?;
        }
        Ok(())
    }
}

impl<'b, T0, T1, T2, T3, T4, T5, Ctx> Decode<'b, Ctx> for PlutusPurpose<T0, T1, T2, T3, T4, T5>
where
    T0: Decode<'b, Ctx>,
    T1: Decode<'b, Ctx>,
    T2: Decode<'b, Ctx>,
    T3: Decode<'b, Ctx>,
    T4: Decode<'b, Ctx>,
    T5: Decode<'b, Ctx>,
{
    fn decode(d: &mut Decoder<'b>, ctx: &mut Ctx) -> Result<Self, decode::Error> {
        d.array()?;

        use PlutusPurpose::*;

        match d.u8()? {
            0 => Ok(Spending(d.decode_with(ctx)?)),
            1 => Ok(Minting(d.decode_with(ctx)?)),
            2 => Ok(Certifying(d.decode_with(ctx)?)),
            3 => Ok(Rewarding(d.decode_with(ctx)?)),
            4 => Ok(Voting(d.decode_with(ctx)?)),
            5 => Ok(Proposing(d.decode_with(ctx)?)),
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
                "unknown error tag while decoding ShelleyPoolPredFailure: {tag}"
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
        e.encode(&self.0)?;

        Ok(())
    }
}

impl<'b, C> Decode<'b, C> for ConwayTxCert {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        let pos = d.position();
        d.array()?;
        let variant = d.u16()?;

        d.set_position(pos);
        let cert: Certificate = d.decode_with(ctx)?;

        use ConwayTxCert::*;
        match variant {
            // shelley deleg certificates
            0..3 => Ok(Deleg(cert)),
            // pool certificates
            3..5 => Ok(Pool(cert)),
            // conway deleg certificates
            5 => decode_err!("Genesis delegation certificates are no longer supported"),
            6 => decode_err!("MIR certificates are no longer supported"),
            7..14 => Ok(Deleg(cert)),
            14..19 => Ok(Gov(cert)),
            _ => Err(decode::Error::message(format!(
                "unknown certificate variant while decoding ConwayTxCert: {variant}"
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
        use ConwayTxCert::*;
        match self {
            Deleg(cert) => {
                e.encode_with(cert, ctx)?;
            }
            Pool(cert) => {
                e.encode_with(cert, ctx)?;
            }
            Gov(cert) => {
                e.encode_with(cert, ctx)?;
            }
        }
        Ok(())
    }
}

impl<'b, C> Decode<'b, C> for Utxo {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        let tx_vec = d.decode_with(ctx)?;
        Ok(Utxo(tx_vec))
    }
}

impl<C> Encode<C> for Utxo {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), encode::Error<W::Error>> {
        e.encode_with(&self.0, ctx)?;
        Ok(())
    }
}

impl<'b, C> Decode<'b, C> for VotingProcedure {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        d.array()?;

        Ok(Self {
            vote: d.decode_with(ctx)?,
            anchor: d.decode_with(ctx)?,
        })
    }
}

impl<C> Encode<C> for VotingProcedure {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), encode::Error<W::Error>> {
        e.array(2)?;

        e.encode_with(&self.vote, ctx)?;
        e.encode_with(&self.anchor, ctx)?;

        Ok(())
    }
}

impl<'b, C> Decode<'b, C> for Voter {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        d.array()?;
        let variant = d.u16()?;

        use Voter::*;

        match variant {
            0 => Ok(ConstitutionalCommitteeKey(d.decode_with(ctx)?)),
            1 => Ok(ConstitutionalCommitteeScript(d.decode_with(ctx)?)),
            2 => Ok(DRepKey(d.decode_with(ctx)?)),
            3 => Ok(DRepScript(d.decode_with(ctx)?)),
            4 => Ok(StakePoolKey(d.decode_with(ctx)?)),
            _ => Err(decode::Error::message("invalid variant id for Voter")),
        }
    }
}

impl<C> Encode<C> for Voter {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), encode::Error<W::Error>> {
        e.array(2)?;

        use Voter::*;

        match self {
            ConstitutionalCommitteeKey(h) => {
                e.encode_with(0, ctx)?;
                e.encode_with(h, ctx)?;

                Ok(())
            }
            ConstitutionalCommitteeScript(h) => {
                e.encode_with(1, ctx)?;
                e.encode_with(h, ctx)?;

                Ok(())
            }
            DRepKey(h) => {
                e.encode_with(2, ctx)?;
                e.encode_with(h, ctx)?;

                Ok(())
            }
            DRepScript(h) => {
                e.encode_with(3, ctx)?;
                e.encode_with(h, ctx)?;

                Ok(())
            }
            StakePoolKey(h) => {
                e.encode_with(4, ctx)?;
                e.encode_with(h, ctx)?;

                Ok(())
            }
        }
    }
}

impl<'b, C> Decode<'b, C> for Certificate {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        d.array()?;
        let variant = d.u16()?;

        match variant {
            0 => {
                let a = d.decode_with(ctx)?;
                Ok(Certificate::StakeRegistration(a))
            }
            1 => {
                let a = d.decode_with(ctx)?;
                Ok(Certificate::StakeDeregistration(a))
            }
            2 => {
                let a = d.decode_with(ctx)?;
                let b = d.decode_with(ctx)?;
                Ok(Certificate::StakeDelegation(a, b))
            }
            3 => {
                let operator = d.decode_with(ctx)?;
                let vrf_keyhash = d.decode_with(ctx)?;
                let pledge = d.decode_with(ctx)?;
                let cost = d.decode_with(ctx)?;
                let margin = d.decode_with(ctx)?;
                let reward_account = d.decode_with(ctx)?;
                let pool_owners = d.decode_with(ctx)?;
                let relays = d.decode_with(ctx)?;
                let pool_metadata = d.decode_with(ctx)?;

                Ok(Certificate::PoolRegistration {
                    operator,
                    vrf_keyhash,
                    pledge,
                    cost,
                    margin,
                    reward_account,
                    pool_owners,
                    relays,
                    pool_metadata,
                })
            }
            4 => {
                let a = d.decode_with(ctx)?;
                let b = d.decode_with(ctx)?;
                Ok(Certificate::PoolRetirement(a, b))
            }

            7 => {
                let a = d.decode_with(ctx)?;
                let b = d.decode_with(ctx)?;
                Ok(Certificate::Reg(a, b))
            }
            8 => {
                let a = d.decode_with(ctx)?;
                let b = d.decode_with(ctx)?;
                Ok(Certificate::UnReg(a, b))
            }
            9 => {
                let a = d.decode_with(ctx)?;
                let b = d.decode_with(ctx)?;
                Ok(Certificate::VoteDeleg(a, b))
            }
            10 => {
                let a = d.decode_with(ctx)?;
                let b = d.decode_with(ctx)?;
                let c = d.decode_with(ctx)?;
                Ok(Certificate::StakeVoteDeleg(a, b, c))
            }
            11 => {
                let a = d.decode_with(ctx)?;
                let b = d.decode_with(ctx)?;
                let c = d.decode_with(ctx)?;
                Ok(Certificate::StakeRegDeleg(a, b, c))
            }
            12 => {
                let a = d.decode_with(ctx)?;
                let b = d.decode_with(ctx)?;
                let c = d.decode_with(ctx)?;
                Ok(Certificate::VoteRegDeleg(a, b, c))
            }
            13 => {
                let a = d.decode_with(ctx)?;
                let b = d.decode_with(ctx)?;
                let c = d.decode_with(ctx)?;
                let d = d.decode_with(ctx)?;
                Ok(Certificate::StakeVoteRegDeleg(a, b, c, d))
            }
            14 => {
                let a = d.decode_with(ctx)?;
                let b = d.decode_with(ctx)?;
                Ok(Certificate::AuthCommitteeHot(a, b))
            }
            15 => {
                let a = d.decode_with(ctx)?;
                let b = d.decode_with(ctx)?;
                Ok(Certificate::ResignCommitteeCold(a, b))
            }
            16 => {
                let a = d.decode_with(ctx)?;
                let b = d.decode_with(ctx)?;
                let c = d.decode_with(ctx)?;
                Ok(Certificate::RegDRepCert(a, b, c))
            }
            17 => {
                let a = d.decode_with(ctx)?;
                let b = d.decode_with(ctx)?;
                Ok(Certificate::UnRegDRepCert(a, b))
            }
            18 => {
                let a = d.decode_with(ctx)?;
                let b = d.decode_with(ctx)?;
                Ok(Certificate::UpdateDRepCert(a, b))
            }
            _ => Err(decode::Error::message("unknown variant id for certificate")),
        }
    }
}

impl<C> Encode<C> for Certificate {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), encode::Error<W::Error>> {
        match self {
            Certificate::StakeRegistration(a) => {
                e.array(2)?;
                e.u16(0)?;
                e.encode_with(a, ctx)?;
            }
            Certificate::StakeDeregistration(a) => {
                e.array(2)?;
                e.u16(1)?;
                e.encode_with(a, ctx)?;
            }
            Certificate::StakeDelegation(a, b) => {
                e.array(3)?;
                e.u16(2)?;
                e.encode_with(a, ctx)?;
                e.encode_with(b, ctx)?;
            }
            Certificate::PoolRegistration {
                operator,
                vrf_keyhash,
                pledge,
                cost,
                margin,
                reward_account,
                pool_owners,
                relays,
                pool_metadata,
            } => {
                e.array(10)?;
                e.u16(3)?;

                e.encode_with(operator, ctx)?;
                e.encode_with(vrf_keyhash, ctx)?;
                e.encode_with(pledge, ctx)?;
                e.encode_with(cost, ctx)?;
                e.encode_with(margin, ctx)?;
                e.encode_with(reward_account, ctx)?;
                e.encode_with(pool_owners, ctx)?;
                e.encode_with(relays, ctx)?;
                e.encode_with(pool_metadata, ctx)?;
            }
            Certificate::PoolRetirement(a, b) => {
                e.array(3)?;
                e.u16(4)?;
                e.encode_with(a, ctx)?;
                e.encode_with(b, ctx)?;
            }
            // 5 and 6 removed in conway
            Certificate::Reg(a, b) => {
                e.array(3)?;
                e.u16(7)?;
                e.encode_with(a, ctx)?;
                e.encode_with(b, ctx)?;
            }
            Certificate::UnReg(a, b) => {
                e.array(3)?;
                e.u16(8)?;
                e.encode_with(a, ctx)?;
                e.encode_with(b, ctx)?;
            }
            Certificate::VoteDeleg(a, b) => {
                e.array(3)?;
                e.u16(9)?;
                e.encode_with(a, ctx)?;
                e.encode_with(b, ctx)?;
            }
            Certificate::StakeVoteDeleg(a, b, c) => {
                e.array(4)?;
                e.u16(10)?;
                e.encode_with(a, ctx)?;
                e.encode_with(b, ctx)?;
                e.encode_with(c, ctx)?;
            }
            Certificate::StakeRegDeleg(a, b, c) => {
                e.array(4)?;
                e.u16(11)?;
                e.encode_with(a, ctx)?;
                e.encode_with(b, ctx)?;
                e.encode_with(c, ctx)?;
            }
            Certificate::VoteRegDeleg(a, b, c) => {
                e.array(4)?;
                e.u16(12)?;
                e.encode_with(a, ctx)?;
                e.encode_with(b, ctx)?;
                e.encode_with(c, ctx)?;
            }
            Certificate::StakeVoteRegDeleg(a, b, c, d) => {
                e.array(5)?;
                e.u16(13)?;
                e.encode_with(a, ctx)?;
                e.encode_with(b, ctx)?;
                e.encode_with(c, ctx)?;
                e.encode_with(d, ctx)?;
            }
            Certificate::AuthCommitteeHot(a, b) => {
                e.array(3)?;
                e.u16(14)?;
                e.encode_with(a, ctx)?;
                e.encode_with(b, ctx)?;
            }
            Certificate::ResignCommitteeCold(a, b) => {
                e.array(3)?;
                e.u16(15)?;
                e.encode_with(a, ctx)?;
                e.encode_with(b, ctx)?;
            }
            Certificate::RegDRepCert(a, b, c) => {
                e.array(4)?;
                e.u16(16)?;
                e.encode_with(a, ctx)?;
                e.encode_with(b, ctx)?;
                e.encode_with(c, ctx)?;
            }
            Certificate::UnRegDRepCert(a, b) => {
                e.array(3)?;
                e.u16(17)?;
                e.encode_with(a, ctx)?;
                e.encode_with(b, ctx)?;
            }
            Certificate::UpdateDRepCert(a, b) => {
                e.array(3)?;
                e.u16(18)?;
                e.encode_with(a, ctx)?;
                e.encode_with(b, ctx)?;
            }
        }

        Ok(())
    }
}

impl<'b, C> Decode<'b, C> for DatumOption {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        d.array()?;

        match d.u8()? {
            0 => Ok(Self::Hash(d.decode_with(ctx)?)),
            1 => Ok(Self::Data(d.decode_with(ctx)?)),
            _ => Err(decode::Error::message(
                "invalid variant for datum option enum",
            )),
        }
    }
}

impl<C> Encode<C> for DatumOption {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), encode::Error<W::Error>> {
        match self {
            Self::Hash(x) => e.encode_with((0, x), ctx)?,
            Self::Data(x) => e.encode_with((1, x), ctx)?,
        };

        Ok(())
    }
}

impl<'b, C> Decode<'b, C> for PlutusData {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        let type_ = d.datatype()?;

        use pallas_codec::minicbor::data::Type::*;

        match type_ {
            Tag => {
                let mut probe = d.probe();
                let tag = probe.tag()?;

                if tag == IanaTag::PosBignum.tag() || tag == IanaTag::NegBignum.tag() {
                    Ok(Self::BigInt(d.decode_with(ctx)?))
                } else {
                    match tag.as_u64() {
                        (121..=127) | (1280..=1400) | 102 => Ok(Self::Constr(d.decode_with(ctx)?)),
                        _ => Err(decode::Error::message("unknown tag for plutus data tag")),
                    }
                }
            }
            U8 | U16 | U32 | U64 | I8 | I16 | I32 | I64 | Int => {
                Ok(Self::BigInt(d.decode_with(ctx)?))
            }
            Map | MapIndef => Ok(Self::Map(d.decode_with(ctx)?)),
            Bytes => Ok(Self::BoundedBytes(d.decode_with(ctx)?)),
            BytesIndef => {
                let mut full = Vec::new();

                for slice in d.bytes_iter()? {
                    full.extend(slice?);
                }

                Ok(Self::BoundedBytes(BoundedBytes::from(full)))
            }
            Array | ArrayIndef => Ok(Self::Array(d.decode_with(ctx)?)),

            any => Err(decode::Error::message(format!(
                "bad cbor data type ({any:?}) for plutus data"
            ))),
        }
    }
}

impl<C> Encode<C> for PlutusData {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), encode::Error<W::Error>> {
        match self {
            Self::Constr(a) => {
                e.encode_with(a, ctx)?;
            }
            Self::Map(a) => {
                // we use definite array to match the approach used by haskell's plutus
                // implementation https://github.com/input-output-hk/plutus/blob/9538fc9829426b2ecb0628d352e2d7af96ec8204/plutus-core/plutus-core/src/PlutusCore/Data.hs#L152
                e.map(a.len().try_into().unwrap())?;
                for (k, v) in a.iter() {
                    k.encode(e, ctx)?;
                    v.encode(e, ctx)?;
                }
            }
            Self::BigInt(a) => {
                e.encode_with(a, ctx)?;
            }
            Self::BoundedBytes(a) => {
                e.encode_with(a, ctx)?;
            }
            Self::Array(a) => {
                e.encode_with(a, ctx)?;
            }
        };

        Ok(())
    }
}

impl<'b, C> Decode<'b, C> for BigInt {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        let datatype = d.datatype()?;

        use pallas_codec::minicbor::data::Type::*;

        match datatype {
            U8 | U16 | U32 | U64 | I8 | I16 | I32 | I64 | Int => Ok(Self::Int(d.decode_with(ctx)?)),
            Tag => {
                let tag = d.tag()?;
                if tag == IanaTag::PosBignum.tag() {
                    Ok(Self::BigUInt(d.decode_with(ctx)?))
                } else if tag == IanaTag::NegBignum.tag() {
                    Ok(Self::BigNInt(d.decode_with(ctx)?))
                } else {
                    Err(decode::Error::message("invalid cbor tag for big int"))
                }
            }
            _ => Err(decode::Error::message("invalid cbor data type for big int")),
        }
    }
}

impl<C> Encode<C> for BigInt {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), encode::Error<W::Error>> {
        match self {
            BigInt::Int(x) => {
                e.encode_with(x, ctx)?;
            }
            BigInt::BigUInt(x) => {
                e.tag(IanaTag::PosBignum)?;
                e.encode_with(x, ctx)?;
            }
            BigInt::BigNInt(x) => {
                e.tag(IanaTag::NegBignum)?;
                e.encode_with(x, ctx)?;
            }
        };

        Ok(())
    }
}

impl<C> Encode<C> for BoundedBytes {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _: &mut C,
    ) -> Result<(), encode::Error<W::Error>> {
        // we match the haskell implementation by encoding bytestrings longer than 64
        // bytes as indefinite lists of bytes
        const CHUNK_SIZE: usize = 64;
        let bs: &Vec<u8> = self.deref();
        if bs.len() <= 64 {
            e.bytes(bs)?;
        } else {
            e.begin_bytes()?;
            for b in bs.chunks(CHUNK_SIZE) {
                e.bytes(b)?;
            }
            e.end()?;
        }
        Ok(())
    }
}

impl<'b, C> Decode<'b, C> for BoundedBytes {
    fn decode(d: &mut Decoder<'b>, _: &mut C) -> Result<Self, decode::Error> {
        let mut res = Vec::new();
        for chunk in d.bytes_iter()? {
            let bs = chunk?;
            res.extend_from_slice(bs);
        }
        Ok(BoundedBytes::from(res))
    }
}

impl<'b, C, A> Decode<'b, C> for Constr<A>
where
    A: decode::Decode<'b, C>,
{
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        let tag = d.tag()?;
        let x = tag.as_u64();
        match x {
            121..=127 | 1280..=1400 => Ok(Constr {
                tag: x,
                fields: d.decode_with(ctx)?,
                any_constructor: None,
            }),
            102 => {
                d.array()?;

                Ok(Constr {
                    tag: x,
                    any_constructor: Some(d.decode_with(ctx)?),
                    fields: d.decode_with(ctx)?,
                })
            }
            _ => Err(decode::Error::message("bad tag code for plutus data")),
        }
    }
}

impl<C, A> Encode<C> for Constr<A>
where
    A: Encode<C>,
{
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), encode::Error<W::Error>> {
        e.tag(Tag::new(self.tag))?;

        match self.tag {
            102 => {
                let x = (self.any_constructor.unwrap_or_default(), &self.fields);
                e.encode_with(x, ctx)?;
                Ok(())
            }
            _ => {
                e.encode_with(&self.fields, ctx)?;
                Ok(())
            }
        }
    }
}

impl<'b, C> Decode<'b, C> for NativeScript {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        d.array()?;
        let variant = d.u32()?;

        match variant {
            0 => Ok(NativeScript::ScriptPubkey(d.decode_with(ctx)?)),
            1 => Ok(NativeScript::ScriptAll(d.decode_with(ctx)?)),
            2 => Ok(NativeScript::ScriptAny(d.decode_with(ctx)?)),
            3 => Ok(NativeScript::ScriptNOfK(
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
            )),
            4 => Ok(NativeScript::InvalidBefore(d.decode_with(ctx)?)),
            5 => Ok(NativeScript::InvalidHereafter(d.decode_with(ctx)?)),
            _ => Err(decode::Error::message(
                "unknown variant id for native script",
            )),
        }
    }
}

impl<C> Encode<C> for NativeScript {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), encode::Error<W::Error>> {
        match self {
            NativeScript::ScriptPubkey(v) => {
                e.array(2)?;
                e.encode_with(0, ctx)?;
                e.encode_with(v, ctx)?;
            }
            NativeScript::ScriptAll(v) => {
                e.array(2)?;
                e.encode_with(1, ctx)?;
                e.encode_with(v, ctx)?;
            }
            NativeScript::ScriptAny(v) => {
                e.array(2)?;
                e.encode_with(2, ctx)?;
                e.encode_with(v, ctx)?;
            }
            NativeScript::ScriptNOfK(a, b) => {
                e.array(3)?;
                e.encode_with(3, ctx)?;
                e.encode_with(a, ctx)?;
                e.encode_with(b, ctx)?;
            }
            NativeScript::InvalidBefore(v) => {
                e.array(2)?;
                e.encode_with(4, ctx)?;
                e.encode_with(v, ctx)?;
            }
            NativeScript::InvalidHereafter(v) => {
                e.array(2)?;
                e.encode_with(5, ctx)?;
                e.encode_with(v, ctx)?;
            }
        }

        Ok(())
    }
}

impl<'b, C, T> Decode<'b, C> for PseudoScript<T>
where
    T: Decode<'b, ()>,
{
    fn decode(d: &mut Decoder<'b>, _ctx: &mut C) -> Result<Self, decode::Error> {
        d.array()?;

        match d.u8()? {
            0 => Ok(Self::NativeScript(d.decode()?)),
            1 => Ok(Self::PlutusV1Script(d.decode()?)),
            2 => Ok(Self::PlutusV2Script(d.decode()?)),
            3 => Ok(Self::PlutusV3Script(d.decode()?)),
            x => Err(decode::Error::message(format!(
                "invalid variant for script enum: {x}"
            ))),
        }
    }
}

impl<C, T> Encode<C> for PseudoScript<T>
where
    T: Encode<C>,
{
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), encode::Error<W::Error>> {
        match self {
            Self::NativeScript(x) => e.encode_with((0, x), ctx)?,
            Self::PlutusV1Script(x) => e.encode_with((1, x), ctx)?,
            Self::PlutusV2Script(x) => e.encode_with((2, x), ctx)?,
            Self::PlutusV3Script(x) => e.encode_with((3, x), ctx)?,
        };

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use pallas_codec::minicbor::{self, encode};

    use crate::miniprotocols::localtxsubmission::TxValidationError;
    use crate::multiplexer::Error;

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_219() {
        assert_reject_reason("8182068282018200820981a300581d604a6c28fd47292afd87445491396f3cf832b96ef92387e29ae6bf480301821b1dde223cadb71881a1581c24d2406d2646270b076898ef0b2ea9c0c9932cdea771f5931ac7aca6a141300103d8184682008303008083060000");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_218() {
        assert_reject_reason("818206838201820082008201818203820dd9010282841a0001522e581de02dbd94e1f494ac6fcab4f9fa08b88fbd1e6e635e3227e00f8a583223810682782a68747470733a2f2f6f69754534506c76747732344a545541737037675476522e7965557339652e636f6d58201c5d6370de0af731d2559556e027f947f14cf732ab8dce7b3c81115aa2f7e954841a0008a703581df13df154a06758fa4fc55d2bae5362491e56ca0a05e6e47f03bbdf1ff38400825820d4a5ed72cbf6d4c67bc702ff0471e208e07f19ecbf2cdeec1e942bb417f61bbf01b818011a000e34dd020203000400051a000acbc20702080209d81e821b0494a3bf2307660718280ad81e821a063a31211a12a05f200bd81e82185718fa1019864c111a00063eb01382d81e821b1d011a10a23289691901f4d81e821b3b1cc3c5552d25571a0007a12014821b789e161d00e15aba1b0770585a0313c63a16021700181985d81e821b000001fede89c7731b00000246139ca800d81e821a0ecca0d11b00000002540be400d81e821b000018c0130c8ae31b00002d79883d2000d81e82190d91191388d81e821b2526d898fb47df151b8ac7230489e80000181a8ad81e821a05db57f31a0ee6b280d81e821a0004ced31a001312d0d81e821b133450b26afe91b91b4563918244f40000d81e821a000104891a000186a0d81e821b00000007e2c612eb1b000000174876e800d81e821b000004667a0c5c131b000009184e72a000d81e821b00000008639b7c991b0000000ba43b7400d81e821b0009501824ebcfb11b002386f26fc10000d81e820101d81e821b001dbdafc28c03791b00b1a2bc2ec50000181b01181c01181d02181e1a000a71011820001821d81e821b07746537b09ae5f91b00de0b6b3a764000581c179302091e603ddab40b1b9fa906d33c957f41a6599647b0eb0567fd82782768747470733a2f2f796948754a436341644158614270414275364a584d4767476b466f2e636f6d5820288c6a98a2257268744893e50335b869628460f0b2fea4c4f22df6562c073ce38203820c841a0001b4cd581de010018d016988527f63562e07653ef02a601e3be0762476786249dcb58302a0f682783968747470733a2f2f39326a365464422d6a726d38303253536852764375536b4e624a4c6a5a32446c456958704c7973447746574b682e636f6d5820297232fdf51c9a4dc84c607f875e8e684a834df609840af36139cc0807cf656482028200a1581de0c5b5e5e636f860e651e88369e39524cb550ad70614c3a84a27cd0ca11a000bec00");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_217() {
        assert_reject_reason("81820684820483581c9cb9f43fb9d49ffdb693ed2ad8b1d0b3c0da464a10ec44ac9c617607581ce59bb42034a8c6d939c556fb77cf7d94141c3aa731fb27cb567ef03b581cf5dce4ae6c05dcea8d7658f0184023620f432291fc4b6576d1432d538201820082008201838201581c0759d2e3fc024d430e269d6976cc9a88fd46f3eaffa167cca31c191c8203820dd9010282841a0005441c581de0c3e366917bc518e74a56527fb7893593ee6e28a6b38daf453d3384118504825820287e69fd76ddb7dd779559180dc345088cc8ab13f1daaaca27a92f9de959a9b600d90102828201581c8221dd7d109e7dcf7bc0c0ba63f708d64895c0ffcf218cb4ddf9ec748200581c607e756078c327e4f2638c273b77c4fec96879c4a00026b5de6aebc2a0d81e821a003221dd1a017d784082783768747470733a2f2f4f722d7249334a4a56306549785935455970785058385162695872456e452e6c2d5a44636142487648734c2e636f6d5820071ccf84eaf8b7f596d08aefa074314f0b10d8ac9811cf9b8ed7118736cb8adc841a0004fb4f581de007c235c0c49465a5fbaafad39090cad63ffa21789b93ea07cfd8c1138203f682783b68747470733a2f2f2e63536b4f493346426f30702d334570684d7a646872305837686441504968665a762d68534b353769756767724c2e2e636f6d58201ba841e51248bfe589d21daebecac4940b8d83b996a0b04be73050250c04cffd82008205841a00028c23581df19dce6190eb0fd97f7aa6901d03667359199299eea0b852be2e8a3e9c8302a1581df135ea087b0a324782f835523142c2ce952f972f2f2d71705b747cfa441a0008f0a9581cd82e1532f9e710df3d95c83b4ba81daca4b9671629539b6356261ddc82783d68747470733a2f2f73743753393447424d576e76564e70476b52454b774e62587836755a306d70586c2d6c7a78654b7243754b6238364f6b782e636f6d5820253c46155d4ad2fae14720663cbb6d48630339934c26c92d8fc8518d6396529d8207613b8203820582828202581c53b39e1fe952387d6e8aa0a463e3afb001a7bd7cd7c00a083599bebd825820ec72eeaab502da23031433a6e1c61bf6f4bed26ebcf13b809792a27fed80747900828203581c191e4585270fc406546c8e7bc6e4d796aa4e84f83799620defd1f453825820596dbfeff44a7db9fc68c9040c7e6ba475ebaee9b0f843801fa3a35ac22932bb02");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_216() {
        assert_reject_reason("81820683820182065820af8209e1859bc59559c3919d91bb73c42604365495e1aa8451daca3b34350bbc830603038201820082008201828203820dd9010282841a0008cb45581de184333f59851fb2e4c63ec29f2cb3c2c1b39643faaac361ce12a3bafa8203f682782968747470733a2f2f71626630495635426f645a2d455449416f566f61355035394e70446d472e636f6d58204077fdc4646c452e55eb5661a1127ae83075c95cdaedb20d65ef7647847e4f06841a00045dfc581df1bdf39c0a800b2538f916cd9ef2d10ac4679795981ca8ab564331b0498302a3581df023aea0c7537e1dea1f6f6bb79bacf0bc585ce6565eaa5b6442c9a8491a000956bb581de006b6522904f69e257b7734bbfba50cd4198c491d00f73f3e378184381a000ae646581de098fccb290ce2cd87f770c7b38b8b411e40c27a3149cba598cdba3db319dc2ef682782268747470733a2f2f75492e464d79684a4b65786d423442395752527a2e732e636f6d5820ae6fdb17b0d53c6badb4bce9eb3a49ca62b01620d25a0f9a8b4bfbe55ed0e3db820382098304581cf365458d1e40cb5febced799c68186eb4799dde57f731a72d429bd9e00");
    }
    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_215() {
        assert_reject_reason("8182068282018200820da38258202c15bf61c964a9df3ed7d81c91288ab0bbaf1c99ee4c2a034f8f990fcc21f7c301a40058391023f55cbfd0c4a56434184f76dc089425f46f28bed49fcb4081c65a6a11e53f2f6740e5d2b2733cf05dd9d7601efd548f702fd6d6ed9b25e401821b5471c015cfa67113a1581c43ab475bd3e21354e1d9f203a5cfb188e96fe20bc02a9039bd91bb59a141361b54f14ea71d8ca6000282005820ee996e189a39f01986e32a60ba3959251def909d1e2ab1c12c2bc4db8ef7fd0903d818458200820404825820a49f8b1710b49d2f5edce0ca4ecf7a4912c4aac10479e7337eae873b65540d3f02a400585082d818584683581ca9905dda942884233cde4913ff1c4fb8a7aa73d04aa95de0b0e86797a10158225820696d746363786171656564666871656a67666b666568626a6b6179777a6f6679021a05cbba1b01821b1c64ea8204b41692a1581c245d5a7a06fe18358242e81281cd5ba9e6abe4efc54e7b659f25abaea14381e2461b0f3b88e5e33da0d3028201d81858449fd87a9f80d87a9f04ff9f413e204483a99079ff41e4d87e9f426edaffff9f44be0ae63bffd87a9fa144ec728f344159d87a9f01425f0e0301ff9f420ce40423ff40ffff03d818584882008202838200581cba632d3c49cba8249749dec9d6b58f6fdde83f7e7bc8a42cc05d73428200581c013829f553810e3ffade14b8ccdbb543d118ceac6597d86075c7930b820280825820f76e5c76172ab74c8ba5f21af5796acc5305b56b3bd6ac22c82e835464faa37b018258391108bd797544805839ff8531da72ca35c563b2a43ffda597cbdd697de355bf61b29287b14f91d52ee4968aa809eb35e88383688af558e02370821b6be775c9070e7010a1581ce37f460f169d19a41c8909b8905c3193aea61b88a6d480493935b907a15213a85ffcc659b6ad15d083cc1b8eff43085c1b40e527d7f4fda95c83051a000cd8291a000dd3b0");
    }
    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_214() {
        assert_reject_reason("818206838201820f808203820983828200581c8f21d3b6395247a1ac6a7f0b8334d9a6b176554811d6ae95d3b6fc9382582002d71e2cff34e26848125fda975b8bc8b12d4c314eb7418d15d09963c824634801828202581cf110de76a242c714151a9e5fe5041beba36634d54b5531ad5117598b8258203b366f327e5e4ab533d3f63c2f54a136536c763ad33a77a4d37b99198a2c4d1d00828203581cc19b599ec4d8bc9334078392b3e1c97e31470cb46203c4e062ad8d938258205c3ae7c952294f1ee36cc4495fe896bbda210b08b48561fa9093797baba912df008201820082008300f4820184830164f3be84b3415c8301624f1341a08301612f41de8301614741fa");
    }
    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_213() {
        assert_reject_reason("818206848306030182038207a38201581c955de8462bff0cb8f0a608081cef0ff3bb4e9948c87f32ebaf18f87b008201581ca56f175bb79337e6f3051440143875a9fa0807ee1f0fa83da263e91e038200581c6965437edf99effe8fb1a8480976f27b90ee45cb992ff5102113cdb00083051a000a99701a00037d2282018211d9010280");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_212() {
        assert_reject_reason("8182068382018200820f8202a3581c01245af2aa18f14248934987875c6c2bb3001c6f077232e5bae148d3a14703ff16af1f1ba302581c467f58932b54910584a0e8ea25a225e06a14530b2e96e938c53a3f22a1430794f81b515e043538e443c9581c5e347134badc15e715a9b5478969104fb3b1f0d578cf4748e96c1d06a15818252a1ff59d88f8eec49f91854c7c89e13e1ce9641c751b6c0183060200820482581cb787e8f8d6b8d64d53bfb0b46df329caf389b85fb5ecbf4907911dc6581c301b69a55b727a4812e5ae97a245766c93af1d21eacebde79a06d6da");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_211() {
        assert_reject_reason("818206818203820f8302a2581df0eb7a7477f5ae057e20d2af59323f70aa3dcff44fb7c11f98dc282fa11a000efea6581de122b54ce8531a96cccc039f3df2f0a2e5d71305e15fe32eb0d425de481a0006caea581cca5359a1d21971169ad0bb3c8f919a19aab4a91d6e8b9c35bb4a2d74");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_210() {
        assert_reject_reason("81820681820483581c92c20b176496806c97b2be6ef847a257eaaf8d6c4329a9e8afbd45c3581c3a1a420e7d0ce87b06c8e354227ecf3f0d1c7cad1f77ea4c23debce6581ced28532926a06baafbb9032aea8eacfe174138df760adba4fdbe18d3");
    }
    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_209() {
        assert_reject_reason("8182068182028200a2581df0a3657b46043adb954f3b12f92e3a72bea4e9bb0f388558a7c4a692fb19e276581de065006c807ecacbcf35a35d23119639745490762385c5aabe26011d1f1a000d496d");
    }
    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_208() {
        assert_reject_reason("818206828201820f8282050182010282018108");
    }
    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_207() {
        assert_reject_reason("8182068182018200820082018182038208820760");
    }
    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_206() {
        assert_reject_reason("818206828201820082008201818203820882048200825820ddab1290bd66e98c22e381c93f62bbe78f185c0a45e5a8b76b7c8d25156bd42f0083062020");
    }
    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_205() {
        assert_reject_reason("8182068283051a000cb6d81a00079024820182008200820181820382088206d90102818258209ccd57b4b05da96b598c307b2ac43c8d3d4a7e3d334acfbb485e576377b7a2bd01");
    }
    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_204() {
        assert_reject_reason("81820681820182008200820181820382088204820100");
    }
    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_203() {
        assert_reject_reason("818206818201820a818282028a03581c1c2289c4feaf4fc5179b02a86fcd9fd680a34d8cb8abdfaea538544f58208875c00c208849b51c02b6ecbea3a8446f363bc37946bdfe8b18fd0c20d105ca1a00016149196e27d81e820001581de1ae08cddae4cafeab700dbf49ac37c9e6f593af1a7a5d82472a778635d9010281581c047a69ae900728df1ae9e6af92b8c1ba196ace9b0ac60ff60f93db208182027762436a544c74573465573654326679396f38732e636f6df6581c183c873908d3749511232309412d676f8acabbf3e7e3be4f07a3f55c");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_202() {
        assert_reject_reason("81820682820764f48e92ae820182008200820181820382088201825820f68d0be332597839c20ecb8e58bc761a0da7a5dde853efdb680891c206f1933e01");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_201() {
        assert_reject_reason("8182068283051a000306b31a0007c5238201820a81828202840c8201581c81af975bf4f79d99167a89c95d8952e4213306aa6f19598abd3b1faa8200581cb6eddcd3a43ece25fc9b9b6f116762fa0fb9628f74f0a2e366fe27dc193b78581ca568181d119a8b367c19cddcbdcfac49f34faf6e4e5ffc475194c41c");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_200() {
        assert_reject_reason("818206828207608201820a818282028a03581c1de626154df3da6a4d21ce8a2fcca1f9cc6bc26108be05df7edbc1c7582098b7e962db1c8a74c6aafc3e3f2897390aa6f458f68e85eefb83591398c08e3719f35f1a000488c2d81e820001581de0324bcf8c928ef3799319d420a7cb3e1133baaaf5b35a84803ac0af36d9010280818400014400000000500000000001000000000000000000000082782c68747470733a2f2f73562d4c4a6f776d4d35754164314f35324173654f4a65754578497048757a6f2e636f6d5820753e219e507c9002a740ad1b66506a17013a73eff9e85f1e534b26e806271511581c4326932d8e34e6bc68eae123635b212cf13a446c3733ed756793ef46");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_199() {
        assert_reject_reason("818206818201820a818282028a03581ca4f2612d7ed982e37a696185a4e691c4481727796d652a28958ad7ff582011aa2d184886edccab0415970cce3d85beb77ff7f4b1b222a16331e31dd4eeaa1a0009216e1a000ee410d81e820101581df1f70405628ddca50a1f99868d60e999c53dfeeb56c150bece655c2ee6d901028081840001f65000000000000000000100000000000000f6581c3f88188ed6a744013cd53bc529ea10c936e2054f0b7c441ad7df1fbf");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_198() {
        assert_reject_reason("818206828201820a818282028a03581c80a90e16b70c97aab1ae259952c9b62c16daf1e78e3d1a541c1946cb5820cab47b532930c904b4bffb726d82dc6f3c005c089ccfb94b0f92cab9d0f5e5a41a0006e9921a000b8771d81e820001581de18ec04b21fafcf4fff3920fcea386b0dd7679ec970624f1e97f091597d90102808082782768747470733a2f2f687762334971644b305155417a75706166304563763742787566702e636f6d58207279b517debcb6a4eebc5080a11730ad30a330754a549c3fa7756841abcc8350581cf53c6c5d66f5589c4fa2b540a6ada621dafe320b73954172075bef7683051a000b39581a0009366f");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_197() {
        assert_reject_reason("818206818201820082008201818203820ca18200581cd8ff887f4509aed556a37b202bb943a2ab60cb7d0399a27bbc45e188a18258203c606408e208a504419dc3f96424ce40a8ce52af320be014d6c0aac8e9ac872701820182782668747470733a2f2f7369566530664a646377644d66677a4e482e746f7659724b2d722e636f6d5820baa959a417bb888188abaf0176088153571a4c9575f2d8017e6d7ee0db6b9268");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_196() {
        assert_reject_reason("818206828201820082008201818201581c41180946cf45db921ead40042f9f61b40174c78d2b580d1d958662e1820760");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_195() {
        assert_reject_reason("818206818201820a81828202850d8201581cac4d33158bca5793e520077c533bf2f71d768563be379a138e5931ba581cf1a9a66ac70aaa565db26b6574f7e18f2d66f1b4e9d54880d7f4767981031a0005b708581c45930f9886b7f7aeb351d5ac0553b1c9481a60371a3f800df94a531b");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_194() {
        assert_reject_reason("818206828201820082008201818203820dd9010281841a000551cc581df002210d84e38df21c1911ee6b1d3c426fa51cdde90d27020cd09781838400f6b4020103010400051a0008e9dd0700080109d81e821b066ddf58c681099f19c3500bd81e82030a101a000a044d12a01382d81e821b42ce89f7007f1e631b000000174876e800d81e821b01f0d7309c44d70b1a3b9aca0015821b253c80586e74cdf81b252a70d41abb80331700181801181985d81e821a2547d11b1b00000003a3529440d81e821a58847fcd1b00000004a817c800d81e820405d81e821b5a791b1afa9dcba91b8ac7230489e80000d81e821a01c3a4511a02625a00181a8ad81e821a003184f71a004c4b40d81e82182b1832d81e821b0000000545cbf2111b0000000ba43b7400d81e8219ab9d1a000186a0d81e821902611903e8d81e821b000008c1767328391b000009184e72a000d81e821a1c16dd9d1a2540be40d81e821b0000001c942edfdf1b000000e8d4a51000d81e821902151903e8d81e821b000000205bbb713f1b000000e8d4a51000181b00181c00181d01182001581ce116fbcb42485f10245d30d41a1c883a4c1578ef60abe9ebfa92a734826f68747470733a2f2f3831442e636f6d58201467b507e5cc77d34b39ed70f84a409829c4242416b0e0e81d318d96b42bc32f82028201820382038201581cee408ed9115ca8d2a53cdfd3b992c1f90fbcdac25e58114384dd602c");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_193() {
        assert_reject_reason("818206818201820a81828202840a8200581caeb0f447a1f725c355fc00ec4f7c0403c88d425cf07e00b3b5d38d27581c4bdd0816568c294d612bea1270d7a030910b7357118f44c72936b40c8102581c4c10c7d7849a3ccb83f67a314f896bd2a813ae4da355788ffcb0886e");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_192() {
        assert_reject_reason("818206818201820082008201818203820ca18201581c341d184318368cf092a45bd1bbdc95314dc51471faa30974746b6433a1825820a1849f6725c74f39ca578fccfc77edd4a766d16ee384b88d5eb0f276f431f8e200820282783968747470733a2f2f4a7553554e50754371715a3373615768546a3548573244386476345262554d6d5746465069735572535266425a2e636f6d5820452c407bff5b262c808f7b79fe54f62019502db3c216c7def60f5a2edaca3baa");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_191() {
        assert_reject_reason("818206828201820a808201820a81828202840a8200581ccf4465f4a38f0aec1ab8e0e277e15196bdffe860c20f7dd72ff7d3af581c5ac1ab82d424c285803ebb2514d446ddcf719f10d915282ec975d5708103581c00722c401011cf8d6238e24f4a8acf69e2332b3202343868cfe138f3");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_190() {
        assert_reject_reason("818206818201820a81828202840a8201581c906df4134412665e4497a4eea61d48b5943a044876f9c6a21b1cf9d9581c22ccc62fe40972b18080a29d4cc5e0a2cd8c9fc8b0357985dead1eda8200581c2955b1e4d2a07a58c65cbd41f2577483ea11dbd0bf86fb3107eb3bca581c1c250c81784f9fd0f9961d0f45057439e09dda2acf7907afbb40963d");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_189() {
        assert_reject_reason("818206818201820A818282028A03581C2A681808F6BE19C7739C8F783A8921848F27C150E385A02574EEB9ED5820C09995AAE09B9263CC1711F4F5FF7EEB6101D2397CB6CE40E5DCABF1A7FD2C781A0002EAF21A000AD050D81E821B094B0240888FB69D1B0DE0B6B3A7640000581DF03A8AF08F3B154B44014EBAC498266034ADE5E13A87D78861C7241EC1D9010281581CB1EA9FAC3FC079D7893C0D2F148EE556A4D2B386763CC9C1D823AC0E8082782A68747470733A2F2F4942494D6F6B6B6D3134744854494F416C526A587A4461475735704A41662E636F6D40581CA2849CB0CD15EE77454A167B9E1BCE6B7C391B4F238B2F491941367C");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_188() {
        assert_reject_reason("8182068283060100820182008200820181820200");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_187() {
        assert_reject_reason("8182068283051a00091e931a000eb3f08201820a8182820283098201581c5fd668394939623245d79251daa8c04e42490e0de4a381723fecd2d98102581c3c4b2b6523721d8641f3aa8518e18dc878bfe60c92f8b285c096fc17");
    }
    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_186() {
        assert_reject_reason("818206828201820a818282028a03581c7e3aeb4729a3208c5d975222258989d3aaf5f9da2bb000a3e583a6765820204eea807f7f5d27a978c9fa9f11fe04b8da9883b1d8ffb98b81e84c02c8d1a41a000b25461a000ccf6ed81e821a03eb67891a05f5e100581df189d3eaa3e4906c9263f2d084c1da057f22a0e437854533b5745546e7d9010281581ce4b01cfda2ea80ec0625442f291c4aae19676cfddbb4ac4919c7975881830101781a76443639507573614c2d7446714851457175686d6c722e636f6df6581c084eb420a78b892f508319124153ffb4a645396ffe205c0b18613c5b82018108");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_185() {
        assert_reject_reason("818206818201820a81828202840c8201581c341cd31b1f30e83174fa4d205ca838e54cf2f476e20d8ccffff4841b81021a000ad6ed581c4e7344b532a775e7ac92c0d88098b150c8731bb57108a380101a2fb8");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_184() {
        assert_reject_reason("8182068282018200830700d9010281582050cb466b5b3c81e7084afacdc12cbbb50b46800c44d7a4d499f35e232c010000820760");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_183() {
        assert_reject_reason("81820682820482581c83d8c5807ff38c8b7def8444331edc22fb182981165278d03b1b1f54581c4405bee0ae85172d1180db72367a6ca2c7661aba7bf074394f7e1ef28201820a81828202840b8201581cfa0f89e674dde29b4d0e20d0c7c4907d9706b883cd42869e3c23895e581c856a65fbec4dccbf292f8a73ecaf7dc171070ef034031acec132fae51a00072c9e581c5ed7775e8584241f3c4eccbf64157ce6a53fbc9bdf0e6ea0cedd350f");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_182() {
        assert_reject_reason("8182068182018200820b81830020a3005839309500e1e7eec2d5f5bf4bb8bd74b95ebac9d64066231566cc85ad16716a2e265adec399ed7c89796bd2f609973aa3ebeb5b014f20a409db2c018200a1581c17f91be389ca984b2bc0b2ab0c3da658c3f57340835ba7e17d7c4e6da14944772a3b5ac030a3bb01028201d81843429bb1");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_181() {
        assert_reject_reason("81820682820181088201820082008201818200820283028201581c2591d1074e690d3de72f36675e7cbe2522a586be6721223d8560acaa581c6acfa295b29b744c8851ed5ecc76957a7c6a6978b3cf4829fb5b0719");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_180() {
        assert_reject_reason("818206818203820f8400825820336b4b0f4c5365e5b37be569de49575a57d609c6cb4aecf171f7a23432f04eba00b300190dd202000301051a000bed1e061a000bfeea070109d81e821b25177e18dfde0c9d1901f40ad81e821a06dc164f1a0bebc2000bd81e821a007635d31a00989680111a0005ceed12a3009f20012020200120012001200000012020002000002001000020010000200001010001200020010000202001200120200120010020010001200001002001010000202020200000000120012001012000000101012020002000010000012020202000012001010000202001200101200101010001010101010020002001200020010001000001010100000100200020010020200120002020202020010001010000012020202001ff019f01000101200000012000010020202000200001202001000000012000202000200020200101012020202000200100000020000120010100002020000120000001202000200120000001010100200001010120012020000001002001010120010020002020202020202000202000200020010001012001202020010120012020002020200100002000012001002000010020000020000101000000000120202001010001200101010020202020012000ff184981001382d81e821b40fed985eb16551f01d81e821b24e56d7c2a1679771b00005af3107a400014821b6d7181ae7ac2633e1b64dd1ec019aded9b15821b4ab8641fd0d82a441b193b11f9ab8f26d41600181b00181d01181f194a55182001581c12c0a1d1c1fa4396f443d7050f32eb42030158b6f75e40b02975ffa9");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_179() {
        assert_reject_reason("8182068282038208841a00090e9a581df0e78bdfa8eaf0bc0412d5ce6fa5050da2c792ec1bad79b24c7430926c8301825820f6b26befcb54c116bb8bef495e001941a8446ba6b9572c4eb089e0bbdd05d9b20082000082782368747470733a2f2f784a6c747a67536e7253456b574534635656386b5a55452e636f6d58207da1f5b700c97251a8f42790e3314fc1edaa62f5c2016ede5a4adccd88b20eb98201820082008300f58100");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_178() {
        assert_reject_reason("818206828203840a80820b0182060082018200830701d9010281585782d818584d83581c1e1114c2ac4d81eefb7afe4aa117c4af5f6504f81f24b2d5d2fbeba2a201582258207679646279627a6a6d6f6f6d6772716767777378786f766c6e6b6a6f6e71657702451a27322807001ac60d45b6");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_177() {
        assert_reject_reason("81820682820382018400f6b6001a00022ae0011a000a3ce0020003010400061a00083a5f080009d81e821b0c11b56147dae0cb1927100ad81e820101101a000d41d7111a000ecbca12a1009f20202020000000002001200101012000200120002020202001002001002000200001202020010101202001202020010120010000000120200100000101202020000001010101010001010101200100000020002020010000000100200120010120002020000001010001010101200101002000200020200001012001010020002001000020012001010100010120012001200100000101002020200101002001010101010000ff1382d81e821b6abe176d5740d5cf1a000186a0d81e821b1789daf406045bd31b0000b5e620f4800014821b66cbc44675c425ca1b05bc40eb0fb2800015821b2343804bd7c7b79e1b236d1d5fefb20e021601181985d81e820405d81e820d14d81e821a07a607491a0ee6b280d81e821a0045a1391a005f5e10d81e821b007593b40311f5631b00b1a2bc2ec50000181a8ad81e821b002bd9124643ee991b016345785d8a0000d81e821b000005398b7fec411b000009184e72a000d81e821a0001675f1a000186a0d81e821b000045455b00ce531b00005af3107a4000d81e821b0000ecd2a88218171b0001c6bf52634000d81e821a1f623d251a9502f900d81e821b00c44b1f68c4fafb1b01bc16d674ec8000d81e821b00000041460759151b000000e8d4a51000d81e8219083b19186ad81e821a0001c5e91a000f4240181b00181c001820011821d81e821b0f40deea408eca990af682076101");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_176() {
        assert_reject_reason("818206828201820082008201818203820a8201581c105a8f1bb56444cacc86378c95421aceeb326b0fb7743e493eb82fd582018202d9010281581cac0e08646d4eb33beeb1ab8bf1d6438571e337411348767be4e0cd29");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_175() {
        assert_reject_reason("8182068182018200820da1825820e948cca2d38a91eb7821e7145fd28723ff81a6c9c151f2433b113b5ef9e1797b00a400581d60b9e083591f3387d1a7368ea6dd06deb04a62c89b5792049aadc8584f018200a1581c95a53ddd88b9375ca07ad82625b2b7df5777793ae66fc4f3a4ea675da14b5b7e30bfa26dc5e4c2bfd01b7a1e983d526ad767028200582032ee5dcecd32b31bf6d19f02525a9a185bb82bc8f4e62c8cca467fef7f5cef5703d8184d82008303008183030181820180");
    }
    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_174() {
        assert_reject_reason("81820681820382018400825820e8bacac5fb4f42f4b84281ce79d76dae7db9c93ef4039d283fee5960709b292301b819001a000756a2011a0006c54102000301051a0009b93e061a000ed0cc0701080109d81e821b15fea75e1f6157071a017d78400ad81e821913571927100bd81e821b00000001e1e7e52f1b00000002540be400101a000135f8111a000c916912a01382d81e821b4359d921d162a4c51a00989680d81e821b2983fad8705711031b000000746a52880014821b13c89bd92afbdc801b601f64bbaa49049915821b0fda90b3a2cd25691b3aa14698c028db581700181800181a8ad81e821b0b598ae4c8117c1d1b0de0b6b3a7640000d81e820001d81e821b000002daf5008a6f1b0000048c27395000d81e821902e71903e8d81e821b0b0b56cd601b6e931b0de0b6b3a7640000d81e821b016bbb54e3f0aee51b06f05b59d3b20000d81e821a0297c6ab1a02faf080d81e821a000164551a000186a0d81e821b0000651b800584db1b0001c6bf52634000d81e821b006bb1d63ad8b4f91b00b1a2bc2ec50000181b00181c01181d001820001821d81e821b0a1fbd559d4a5b3b192710581c6f3480898deba05a5c02adc31f8fbd84705fa004d50288c556160b58");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_173() {
        assert_reject_reason("8182068282018210d9010281581c7f39f0d1ecb8bf1eddf17bc521c55e94b0cfc396b82efa9788cd8ccf82018200820da182582037f1810ce372719570e0108f80603614dae041ebedc5a05bd74e4b5a607a358d00835839216d97224b575192f75368169c84fb500a61587ad3e032fdd72bc3194c51aa1dac2bf0a5b25e62f883367e98219bf02f142542c3e5e92136b2821b1bbe871285013339a1581c245d5a7a06fe18358242e81281cd5ba9e6abe4efc54e7b659f25abaea141361b4c88e5d72095a6bd582080ac191df663b4620225143270ee2a3d744f335bfde40d1e70fdffdeed58d276");
    }
    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_172() {
        assert_reject_reason("818206828201820a8182820283088201581ceddd45b25f2175ee7e8abc9201e7d8581d09ae40185d5144103bcfbe1a000a3c8e581cb72b24f3d614d491a5eb89d1a9741066b9c2c1716dca8223a49a143a82018204d9010281581cb0245e91834b8652f4ae894df94a17ee5c527fef5ef36ef8c7149359");
    }
    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_171() {
        assert_reject_reason("8182068282018200820981a300585082d818584683581ceb901070bedb9d79546a390130049ae65e82f921c0655dc8499216dba10158225820964122d74d5e5401f33c39bb4fd8fdf066507b870e328b3bb43c18ca8333155e021a0126e8b2018200a1581c2db8410d969b6ad6b6969703c77ebf6c44061aa51c5d6ceba46557e2a1569c8ebdafef13ec500c680727747c5e43bce1898c42fd0103d8184a82024746010000222601820481581c550c056d2d108b8f16f537ec37d885aaf43f86d8f417223d3ed17b7f");
    }
    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_101() {
        assert_reject_reason("818206818201820180");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_102() {
        assert_reject_reason("818206828207608201820a80");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_103() {
        assert_reject_reason("818206828207613c82028201820283031a0007af381a0006280e");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_104() {
        assert_reject_reason("818206828207611482018209d9010280");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_105() {
        assert_reject_reason("8182068182028201820383041a0003fa7d1a000da9e6");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_106() {
        assert_reject_reason("81820682820764f3b39f9d82038302581de1e2c57a0bbc8741292fb36ac36d39b0e836668eac5482c8002e0c711601");
    }
    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_107() {
        assert_reject_reason("818206828201820ed9010280820763eaa08d");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_108() {
        assert_reject_reason("8182068182028201820182011a00096f70");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_109() {
        assert_reject_reason(
            "818206818202820182028305581c56b1a90a80c582324368cdcd4ce38583be778146df34a1762f577c7801",
        )
        ;
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_110() {
        assert_reject_reason("8182068182018204d9010280");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_111() {
        assert_reject_reason(
            "818206828207614a82038210581df17f87a318935cca1e258e34cca32f57c870284b890c51224358580ab9",
        )
        ;
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_112() {
        assert_reject_reason(
            "818206828207614a82038210581df17f87a318935cca1e258e34cca32f57c870284b890c51224358580ab9",
        )
        ;
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_113() {
        assert_reject_reason("8182068182028201820383021a000800941a00011418");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_114() {
        assert_reject_reason("8182068282028201820182041a0001aac0820760");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_115() {
        assert_reject_reason("8182068182018200820a80");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_116() {
        assert_reject_reason(
            "818206818203821181581de1f5812d8f9567fe793f9eb3e991770c459fd51ff61caabc2a6a7ac6f1",
        );
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_117() {
        assert_reject_reason(
            "81820682820481581c5d2f917f14cbe3c625beb0edfe0a273e3277cc9ac50a48faab4fbd9b8203820f8106",
        )
        ;
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_118() {
        assert_reject_reason("818206818203820f8203825820d88109884d7c59fa05bc51044a54fddc1069e73ef37bbc2a28e77c5962497d4100");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_119() {
        assert_reject_reason("818206818201820ed90102818258206cc8d75e9e8d22059b8b6eed6df1fa8987bddf68cd20041bf0457ed74986236c01");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_120() {
        assert_reject_reason("8182068282028200a08202820182028200581c0c71f8ef3bec68c77322583ba2c5c4b77cefd3a977007d7b33548f55");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_121() {
        assert_reject_reason(
            "8182068182018206582047ba0a25a6ad292b706c0580868b1c51f0bb3448b9c0646d225d9841e0cdf822",
        );
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_122() {
        assert_reject_reason("818206818201830bd90102815820d26b6f6d6f7768ef88a33f8d57e5cc29aef4a2470b2dd81a36dcd6ec0b06502cd9010280");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_123() {
        assert_reject_reason("8182068282018200830c3a000b914f1a000833cf83060020");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_124() {
        assert_reject_reason("8182068282018200810483060120");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_125() {
        assert_reject_reason(
            "81820681820382018302a0581ca89c807fd5c717bd0bee39a11f1d95d2bbf07eb0ac9b08f0fe9f06db",
        );
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_126() {
        assert_reject_reason("8182068282028200a08201820083051a000b9e821a0002c879");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_127() {
        assert_reject_reason("8182068182018200821101");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_128() {
        assert_reject_reason("81820681820182008302828101810101");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_129() {
        assert_reject_reason("818206818203840a81825820c7e49d82f19cbfb163051fe45139a5109db567846441353c3f3d588bc93ea9b001820001820401");
    }
    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_130() {
        assert_reject_reason("81820682820481581c4e81ef7fcdadc89994db280188d2cc7fdc640ad02eba10f8aaa0b2058203820f8302a0f6");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_131() {
        assert_reject_reason("81820682830601008203840a80820101820800");
    }
    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_132() {
        assert_reject_reason("81820682830600008202820182028305581c1e3cb8338ed459d0838f3eab8cb8e81d7f17f53cd75ddfe616023bbb20");
    }
    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_133() {
        assert_reject_reason("818206818201820083100001");
    }
    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_134() {
        assert_reject_reason("8182068282028201820284010100008203830419bd9e1a0004fb26");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_135() {
        assert_reject_reason("81820681820382018106");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_136() {
        assert_reject_reason("8182068283051a000a9a031a000913d58202820182028401010101");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_137() {
        assert_reject_reason("8182068282018202d9010281581cc9e37a63758d2384f2925923bd991c4d99b6b98a5f91389a2640e2b38201820083141a000248841901ec");
    }
    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_138() {
        assert_reject_reason(
            "818206828207608203820e818201581cfa0620a416f795433029d1a790492c223bd89f019798363aedb20593",
        )
        ;
    }
    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_139() {
        assert_reject_reason("8182068182018307582075f4ce81e57f41b7d6615642f94b567e4deebc09c8e909933c9e16ae26d99e3058204e5cf3f9e298b0d20ee3e3d51080b7ee955d5f82482c4597d5dc035429bf7944");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_140() {
        assert_reject_reason("8182068282028200a08203820e828204581cbbba874ad0c11f0fdea1aaf867b19e60d0511467216815110ec4891b8200581c4417f73277a013e135f7a19b7a3064d7c310882fd3cb5c282193ed85");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_141() {
        assert_reject_reason("8182068283051a0004c6d61a000da4358203820f8302a1581de1e02410af4d130ae2b0059a25a8eb9979ca7a0343a4f942d92aa1d1f51a00075d0d581c4d23e66b7cbbb736c4aff3fffee66102dd34e9eda2c2320bbc1c2719");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_142() {
        assert_reject_reason("818206828201820a81828203581df051b629fe6c8efcc05db7c67a87e222f29e169b10c7d3c4e4e7f8e960581c42c451efd6e783c03a427068162a4477fdba6602f784234cad5d3aac83051a000378761a0002964c");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_143() {
        assert_reject_reason("818206828203820f8504825820a9befe0c443c7d8b9c9a6ffcbcb2e221bd126f418ad67d39936c33d8199de3e101d9010280a0d81e821b0001bb33992a3a051b000470de4df8200083060001");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_144() {
        assert_reject_reason("818206828203820f8504825820a9befe0c443c7d8b9c9a6ffcbcb2e221bd126f418ad67d39936c33d8199de3e101d9010280a0d81e821b0001bb33992a3a051b000470de4df8200083060001");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_145() {
        assert_reject_reason("8182068282038207a18201581c0304b9a8d416cb28dc1cd1aeab86512be01bce92df92cae3e26d688e01820182008200820180");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_146() {
        assert_reject_reason("818206818201820083030020");
    }
    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_147() {
        assert_reject_reason("818206818201820f81820300");
    }
    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_148() {
        assert_reject_reason("8182068182018200821580");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_149() {
        assert_reject_reason("818206828306002082018200830701d9010280");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_150() {
        assert_reject_reason("8182068282028200a08201820a818282048200581ca9bc226d80d1e428c1a433ae831776c713c64de990da648c20c5dbfd581cdcf581cc56e6ba56e0d8941b609cd4a3e25adb7117ceed57d3f3a268");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_151() {
        assert_reject_reason("81820682830620018201820a81828201581c83cf7460f5b4c00c1867be4d4d27390a462b0bd597ce6db40f17be2a581c72886bee067a5d32dd70963fec2b42a6b26034ece1761a36abb35d5f");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_152() {
        assert_reject_reason("81820682820182008216818258207aca4ab9ab63c38ca5f63e24e9dd5a4a8a4625af0cb7d8c531e6e6c68d0fd32101820764f3b08894");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_153() {
        assert_reject_reason("8182068282018200830700d9010281582b82d818582183581cb608f4d5e7a0eba5f8856688bf941ece797df055a7e9a6a9d00f756ba0001a26be366283060100");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_154() {
        assert_reject_reason("8182068282018200820da082028200a0");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_155() {
        assert_reject_reason("8182068182018200820b80");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_156() {
        assert_reject_reason("8182068182018200830700d9010281583931ae7dc8ffb58f253f0b5043b464de6024f3d95f3019f53365075e79f14cf7b79a07f705949cd021d2b4cca776b260c414268f9ef07699ace9");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_157() {
        assert_reject_reason("818206818201820a81828200825820b9fed253815750ad373c5bbd6f3f4dff5aea37f97c3ccae2688539f8b5df06e600581ceeb9940a7f5dc1d62a4a6c2434ee8295596795e65c978fca82f1714d");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_158() {
        assert_reject_reason("8182068282018200830e821b0ad6606cd7fd494c1b38e410ee453aef01821b1a4d09fe8eea73271b1c2de7334d864ba683051a0009351319e6f9");
    }
    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_159() {
        assert_reject_reason("818206818201820083120001");
    }
    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_160() {
        assert_reject_reason("8182068182018200830800d9010281581de0f033030aaf54f96e59ceb93486b0d3d27e1e0a429505f4ae0015f085");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_161() {
        assert_reject_reason("818206828201820082008300f582018183016040820481581c686b39d5d0b4fc372fd59ef84bd0e42e416d52035628ca3c622f7741");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_162() {
        assert_reject_reason("81820682820182008201d9010280820760");
    }
    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_163() {
        assert_reject_reason("818206828201820a8182820282008201581c7a5a6c64d0ee3451ca615220c09793f573382a2dfedb17b5e52ba9d7581ca7aed4982dcdf3d5456835b4fd534eeb04a03460fdcf8a2aba1b994c83060100");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_164() {
        assert_reject_reason("8182068283051a000ac66d1a00099e50820382018400825820bea0058162d89d716fabdad5d5348a951512639e0f19abfc3592af9d666e13dc01b2001a0007f273011a00030ead020003010400051a00043501061a0005f3bf07010bd81e821a00ecd98b1a05f5e100101a00077d151382d81e821b5260d8618ad516db1a00989680d81e821b075c4fb9b087c9211b000071afd498d00014821b4dc6369ed33858f01b7099161740c966e61700181985d81e821a00211c971a004c4b40d81e821b09aba6b7a91ba4bb1b0de0b6b3a7640000d81e821a015ee8af1a017d7840d81e820001d81e821b000f663c07bae39d1b0011c37937e08000181d01181e1a000ba3d6181f1a000adf8b182000581cd2b52f1c99cbc45f441c8b7853c60dcbf4c637ce5268982de1841986");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_165() {
        assert_reject_reason("81820682820481581c44bf89b534d14268692ebb5128dc106fdb1da9ec8ccf88f54574ee2882038208841a000ec2b8581df10e53a9b14eaff458b5916eab0a38801b38a71524b739a737b55b209a8504825820acdb5224782959c8854ffba6a7135a5f54bc4704d7b478d604d5bfeef320766c01d9010280a18200581c2e59179c73f58e833e8ec4572dd495199f664823cd6cbaf18857adfd00d81e821b5b2d2a1156963df71b8ac7230489e8000082782068747470733a2f2f57774970335271505a77434673715853616145792e636f6d5820bee3107a03e3aa281ae0ed7ef52a599f4a2285beda7bd3f1dabb855de9a1a4ca");
    }
    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_166() {
        assert_reject_reason("8182068282018200830320208201820a80");
    }
    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_167() {
        assert_reject_reason("8182068283051a0009b79e19e18782018200820981a400583901d5c082ed40772151110ec630b065cdfdd1a9ff90cddfd405933a46f126b31c9d9fa88e3c51117d5be6ab5d39e4fe98d2f16a6ffa815e9956018200a1581cb0c53e2bf180858da4b64eb5598c5615bba7d723d2b604a83b7f9165a1413402028201d8184aa1d87a9f01ff436e503503d818458200820501");
    }
    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_168() {
        assert_reject_reason("8182068182018200820a81a300583900ef90c4d00722c0905782c358fec66912883ae8cba431fdaba309b5a3d66acdb00c1f608e3fb7ab04c0b639c0868d515d1d00ab707ecfc9c9018200a1581c95224ea4aa18008b51dfcfd549a7ac425dca0eb4b5b53a71afb4993da14d7ce55707d25257cf6a32623f411b1c391a4dc69c17ea03d818458200820180");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_169() {
        assert_reject_reason("818206818201820082158182825839205adff1cda68d1b9a24fccb1e4d711422620e3bc9222369d24c713fc1d96d6f2aa1c5b4674868c1ed1e004b3682e9adb14893e86dfaf61bc28200a1581c105a8f1bb56444cacc86378c95421aceeb326b0fb7743e493eb82fd5a14366c852011a000592b5");
    }
    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_170() {
        assert_reject_reason("81820682820481581cc2cd1b23da4e22b78593f6b0143ec11fe7d2835a765003d7031b2a338201820082158182a400581d608c3584f6a21f0d4c6f733108674e94ec8e18c87dcd9dfc31de35315a018200a1581c467f58932b54910584a0e8ea25a225e06a14530b2e96e938c53a3f22a15610859c66a0b709beb36300bfacf62d21ba1d8133a6fb1b0b418997d0e006b6028201d818585ba2a5d87a9f23447dc5a0d744a2a62e60ff0221a144a1a04a6c4263804172d87c9f2242e3c204ffd87a9f43c99e4a0204ffd8799f0140ffd87a809f4208b6ffa0d8799f44cdef56c3d87a9f40ff439708a3ffd87e9f9f42e6bfffff03d8184b82014847010000222200111a0004d440");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_multi() {
        let arr = ["8182068282018200820f8201a1581c4d50a11e297e7783383bf06dd6e4e481230323bd96cd8b8d9ee3888da1581c10e6342524c8c9b78548f0a3e8888d357607c9349c3c3303fc05d6791b2e2d78035124a72e83060020",
        "818206828207608201820a8182820284108201581cefe4d0f0f0e7c3d094a8f82da3f1bc936df2a549f6699a495476aa251a0002d54f826d68747470733a2f2f6d2e636f6d58203538017e30a9b33df0f6c862b36224539a9f4601d1a263c618106b8979340653581cf2a5a950c6227c0b91bab344894b86b7ab64557d25d29b8a983ffe4f",
        "81820682820382018305f682826d68747470733a2f2f632e636f6d5820725d17d3574085148b727a9e99359ad39231f10e0100f6b65511d2ed2cc4cb08581cc90a357c9c6125643b432f55f65324187dfd5c59ec2a8e5cba87d0ae8203820e828201581cd1e9a7a603336b66e2615a566971de182d60d27143ba65d8a4decebf8203581ca627d673bc5f8d41ef9e75183e8150e53ae084e6c7db111459d68841",
        "818206828201820a818282028304581c49d466acdca7a3c14460095c45c5f536a10f9c299bfb8a148168d70700581ce6b340c20d7d7aeb4ee65f419d3aee4d86df60d4355a11656f6f7c6082018203d9010281581cbf7575e25268f9870d41729abefb2d1a5effccddf1145bbc7384ac26",
        ];

        for cbor in arr.iter() {
            assert_reject_reason(cbor);
        }
    }
    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_multi2() {
        let arr = ["8182068182018200820b81832020a400583921a677aaadacc59f6d9f46406ce95146b452d2614b99ce63a3a430e97c0b9aaf8a6f6f04e7681386cc576b15d9eb0611567383aa9a61b14ced018200a1581ce06ef7446717a34713b439be251dfb47a626ad0927a5de02413bd20da152093636acec0931425f01473689b25aec1ca1010282005820a7d3156a5767b105889271f033a759005c86d9d7b265c29e7154ba603fbc6dcf03d818458200820401",
    "81820682820482581c62c23c7cde24465feb5a2be46d16c183f69f7448dd4e7a6d84eb0ccd581ca38f5da7db261349b9ae58c298f5694c358ad85bc0cb86257d5e6f938203820f8504825820a3c2937bfdd9709d0101527bca443e591b900f184eda9c59f90f83519c559fe801d90102818201581c72d570efe1fed269a417655dcbf46e78a875f3abbef9744c0203e279a0d81e821a0311f2791a07735940",
    "81820682830620208201820a818282058419de8c581de1e5e98b6d2bd7891d634d581c53adbec4f33118d7bcd0e284d77fc97f850482582064227aff4eff0e942fe703304b631f2eed10b6ca6c1474428371fc8e142b9c9501d9010280a0d81e821b0000001731d628391b0000001d1a94a20082782c68747470733a2f2f4444363865474f6a504c384f306459554250642e7362765649433538326a71342e636f6d5820b953af1c13a4028c7e02880489f189c35d337e385598deb1a7a318675ed2b929581cdabde9891773f9c6877753b3705c79b0c2916edbde53afe158714ffa",    
    ];

        for cbor in arr.iter() {
            assert_reject_reason(cbor);
        }
    }
    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0001_ConwayTreasuryValueMismatch_Coin_Coin() {
        assert_reject_reason("8182068183051a000de7561a00080fd6");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0002_ConwayMempoolFailure_ConwayGovFailure_ExpirationEpochTooSmall_List() {
        assert_reject_reason("8182068282076082038207a0");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0003_ConwayUtxowFailure_MissingTxBodyMetadataHash_AuxiliaryDataHash_AuxiliaryDataHash(
    ) {
        assert_reject_reason(
            "818206818201820558200e13ba83be25492abf84e10545393932480e8ad43dacf8a3d93dff388cce84ed",
        );
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0004_ConwayWdrlNotDelegatedToDRep_KeyHash_KeyHash() {
        assert_reject_reason(
            "81820681820481581c22782faa6bd0c54048b6176eb0cc2f4aa6c56818b3b9075e480e4cbf",
        );
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0005_ConwayTxRefScriptsSizeTooBig() {
        assert_reject_reason("8182068183060001");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0006_ConwayUtxowFailure_MalformedScriptWitnesses_List() {
        assert_reject_reason("8182068182018210d9010280");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0007_ConwayWdrlNotDelegatedToDRep_KeyHash_KeyHash_ConwayTreasuryValueMismatch() {
        assert_reject_reason("81820682820481581cab4f400015b95d3b7c45a285fe08da9e4cc110b06105788819890a7283051a0001abb81a0007fc34");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0008_ConwayCertsFailure_CertFailure_GovCertFailure_ConwayCommitteeHasPreviouslyResigned(
    ) {
        assert_reject_reason("8182068282028201820382038200581cde174ee9f903cd93028d16e1bd0df936ddf2a842f2aa414db0598b6782038302581de0c3a48544970283c379904bf33f5ab2b8e1f6fac902a14ddcd18d2bb900");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0009_ConwayTreasuryValueMismatch_Coin_Coin_ConwayTreasuryValueMismatch() {
        assert_reject_reason("8182068283051a0006144d1a0007f68283051a000ab04e1a0003c428");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0010_ConwayMempoolFailure() {
        assert_reject_reason("8182068182076162");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0011_ConwayCertsFailure_WithdrawalsNotInRewardsCERTS_List_RewardAccount() {
        assert_reject_reason("8182068182028200a1581de180c1af75f8e788b08272ee30e8d87bc776e4bfc47adb0da175bf26ac1a000212eb");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0012_ConwayGovFailure_DisallowedProposalDuringBootstrap_ProposalProcedure_ProcDeposit(
    ) {
        assert_reject_reason("818206828203820c841a00043894581de05c60cda4d195859022a5dc288a826c9d413349697e3009dd8163b3358301825820b3a3b00795156a4bd4338afe1d5d1ed55969c088dbc37c134b6346bff9b7210001820b0082783b68747470733a2f2f365859374137397562386e7a755a687a6f73316c4155546a685830416d7a6a715a744837795a4779475a5071694b542e636f6d582049571f726fd12c21b39edd426be658fbc95e5b14cf3b235338573ef9daa1f4c68203830b81581cd3e73bd6d14a851a663cd925ff72ebacad31da7a6cdbddd623087b9780");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0013_ConwayWdrlNotDelegatedToDRep_KeyHash_KeyHash() {
        assert_reject_reason(
            "81820681820481581c3f784466c9efbcbc998ee0121bca9c4dc03dc37b756ed522e4d46ea6",
        );
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0014_ConwayTreasuryValueMismatch_Coin_Coin_ConwayTxRefScriptsSizeTooBig() {
        assert_reject_reason("8182068283051a00059a381a000393c683060101");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0015_ConwayMempoolFailure() {
        assert_reject_reason("8182068182076160");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0016_ConwayTxRefScriptsSizeTooBig_ConwayTreasuryValueMismatch_Coin_Coin() {
        assert_reject_reason("818206828306012083051a000218a31a000594fd");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0017_ConwayTreasuryValueMismatch_Coin_Coin_ConwayWdrlNotDelegatedToDRep() {
        assert_reject_reason("8182068283051a000d755a1a000438a6820482581cf88d2b1e7a199cc2791ecd58b2dba509ebb3213f8a45d76fe6565acf581c9d7fbbc29cea56a0b2cdb7c98f7ebef884ad509b5fecfbdc5d9e0a78");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0018_ConwayTxRefScriptsSizeTooBig_ConwayGovFailure_VotersDoNotExist_DRepVoter() {
        assert_reject_reason("81820682830601008203820e828203581cb921639de9f45aa695050cbb0746979c85e8897e33831a34387222d48204581cd0d83eac6aea2ae34f39c627996457b2eed18ad3283a62b0dbec896a");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0019_ConwayWdrlNotDelegatedToDRep_KeyHash_KeyHash_KeyHash() {
        assert_reject_reason("81820682820482581cc665e067f9c5af2973d41f470db5e85c1f3495958d9ce9f458117a02581cd16ff9615b0f243f73576a54d7b0ee5fd3f1a827899c50191d3cb9b483060120");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0020_ConwayTreasuryValueMismatch_Coin_Coin_ConwayWdrlNotDelegatedToDRep() {
        assert_reject_reason("8182068283051a000aec461a000b26cd820481581c651af173086c113865d55b9abbad7c47aa9569b85f26873ed1b281bd");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0021_ConwayTreasuryValueMismatch_Coin_Coin() {
        assert_reject_reason("8182068183051a000b55c61a000921de");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0022_ConwayMempoolFailure_R_ConwayGovFailure_VotingOnExpiredGovAction() {
        assert_reject_reason("81820682820761528203820981828204581c5e5b6fe689a2a0b842304c912712328aa704b6c49d15e8b60e6d979882582094259fb315f35d28860159dd35231ce60ee3f99d905f9e6519731a505540bb4e01");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0023_ConwayWdrlNotDelegatedToDRep_KeyHash_KeyHash_ConwayUtxowFailure() {
        assert_reject_reason("81820682820481581cbb05dc8589898474b225a13314de415931c384c1013d90c9b10d0f9c8201830d8158208ccf10dc8526e35d4c21bbfa98507c1c9e58cd7d2483a6c502213c3d5fc2f40981582027e04b9c68972e6dc9e8cf34916c338a00a7665df06eb526f333b7af3e908315");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0024_ConwayGovFailure_ConflictingCommitteeUpdate_List_ConwayGovFailure() {
        assert_reject_reason("8182068282038206d901028082038303d901028001");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0025_ConwayWdrlNotDelegatedToDRep_KeyHash_KeyHash_KeyHash() {
        assert_reject_reason("81820681820482581cea98c5db378729e96764fca2d3ca9f2d1973f59e0abecd37833338c0581ceafa36c204c8c0fee37dcbf776c13f31dbe4fd8915cc334beb26e0e9");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0026_ConwayGovFailure_MalformedProposal_HardForkInitiation_SJust() {
        assert_reject_reason("818206828203820183018258203d417a35bce157152945acfe78da6938f3ade318bed839ad70d62cd827abf4470082070182076169");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0027_ConwayWdrlNotDelegatedToDRep_KeyHash_KeyHash_KeyHash() {
        assert_reject_reason("81820682820482581c16e839e30b01738d1115c3efcf1262fac1797d8bd0089de9353aa106581cb5813031a33030f4922a9232e016e25818eefb2e2ccc450cb11f89ec82076138");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0028_ConwayTreasuryValueMismatch_Coin_Coin() {
        assert_reject_reason("8182068183051a0003ec3a1a0003e0c0");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0029_ConwayTreasuryValueMismatch_Coin_Coin_ConwayWdrlNotDelegatedToDRep() {
        assert_reject_reason("8182068283051a000bbfe91a000c125a820482581ca9091c6a554fb48870c42fdb0e367476f4795fea34177de56591bbef581c4ad1ed28f5d0590603d941434b07083a8bcd13cec2d5be9a0b5c8529");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0030_ConwayTxRefScriptsSizeTooBig_ConwayGovFailure_DisallowedVotesDuringBootstrap_DRepVoter(
    ) {
        assert_reject_reason("81820682830601208203820d82828203581ca8d4974095d18fd37c4ba1da80a36851b8345ddaf233a7d75f927e2f825820da98f0e22a86a94d6a7e6f0f526959092f1a6ded895287b15dde2b78d84baf3b00828202581c6e56c2c5a5040fcda445ca0a87436e002e540870fad295f6557077bb8258204a184435fe098d9e62e429475b4e337be0acb369c84bbe39baa74740ef6d69d401");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0031_ConwayUtxowFailure_UtxoFailure_InsufficientCollateral_DeltaCoin() {
        assert_reject_reason("8182068282018200830c1a000d8a871a000b075682018209d9010280");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0032_ConwayTxRefScriptsSizeTooBig_ConwayGovFailure_ExpirationEpochTooSmall_List() {
        assert_reject_reason("818206828306000082038207a18201581cff5c52bb623f42d8d4a48bc9393011167c82650799fe2057d0ffe17800");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0033_ConwayMempoolFailure_ConwayUtxowFailure_MalformedReferenceScripts_List() {
        assert_reject_reason("8182068282076082018211d9010281581cfa0165d3392a8938b5b5a4851d5802f43233ae1f9305b408f39477fd");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0034_ConwayUtxowFailure_InvalidWitnessesUTXOW_VKey_VerKeyEd() {
        assert_reject_reason("8182068282018201815820db765eede4a13a462c48279c62d3b614d3c936e765d8086470fff6f277e2d76e83051a000a786f1a000b2866");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0035_ConwayGovFailure_MalformedProposal_NewConstitution_SJust() {
        assert_reject_reason("81820682820382018305825820b50c881a532f8e51bf3d1b0297acad3066f93e8fc8e16793b07396c01a9d1ad3008282784068747470733a2f2f764875706f33544d47757179795967523271465a545a6c7845615069775875445a7261464c4a49786c584d753134386a514f30332e636f6d5820c6e1e8abededabeba0fb369afd37876919ff1b72c9bd0731304aa3221905e7bb581cbee75c2524e050e9d9c29e1882acc85241563a11d4f4a15158e1093183060100");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0036_ConwayTreasuryValueMismatch_Coin_Coin() {
        assert_reject_reason("8182068183051a0007f26b1a000590a2");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0037_ConwayTreasuryValueMismatch_Coin_Coin_ConwayMempoolFailure() {
        assert_reject_reason("8182068283051a000b3e831a000356b2820760");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0038_ConwayGovFailure_VotersDoNotExist_DRepVoter_KeyHashObj() {
        assert_reject_reason(
            "818206828203820e818202581c6405197a2f6592f55ba348f14d540f35caf3a1dedf1d40cd8e474e04820760",
        )
        ;
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0039_ConwayUtxowFailure_PPViewHashesDontMatch_SJust_SafeHash() {
        assert_reject_reason("818206828201830d815820d15252d17f47ba042f29becbd844d9aabd83b7dafb52bd46589fb10358adb9618158201d97ba11111aee873b749791e741e6f6e9e3d3e7a5e2fc0e532ac6f323a120bf83051a0009c7ac1a000b7da8");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0040_ConwayWdrlNotDelegatedToDRep_KeyHash_KeyHash_KeyHash() {
        assert_reject_reason("81820682820482581c38bb836dc16459b5d268d6c035b49bd8c430d8f617d02af710795df8581cc39e3c910f66ac8ae6a8349e0878d4e529e89f992f24b811bbae222382038200828258204e1a131ff843d622e7bdecddf54b011955d12943181549df5e392faea5d7ae300182582030f363d5469e0099bd16a6f3f41bcbe77c8c8908a5a9ff3f2bc5eee5e62ce26500");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0041_ConwayMempoolFailure() {
        assert_reject_reason("81820681820763e684b9");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0042_ConwayGovFailure_ExpirationEpochTooSmall_List_ConwayWdrlNotDelegatedToDRep() {
        assert_reject_reason("8182068282038207a0820482581c6a6d35b19d4013b919faf9a21cfe32571a540f78ff9d2da0b65d69bf581c8345ef94cf81079de12c4bc2f212f2caeb186eafc7af49d539d561ae");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0043_ConwayCertsFailure_CertFailure_DelegFailure_DelegateeDRepNotRegisteredDELEG()
    {
        assert_reject_reason(
            "8182068182028201820182058200581cb2f0655ce3475b94e5d46d3333f02849a53df7a6fbe82edca31c768d",
        )
        ;
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0044_ConwayTxRefScriptsSizeTooBig() {
        assert_reject_reason("8182068183060100");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0045_ConwayTreasuryValueMismatch_Coin_Coin_ConwayMempoolFailure() {
        assert_reject_reason("8182068283051a0008c44d1a000db83b820760");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0046_ConwayTreasuryValueMismatch_Coin_Coin() {
        assert_reject_reason("8182068183051a000ccad81a00017d31");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0047_ConwayWdrlNotDelegatedToDRep_KeyHash_KeyHash_KeyHash() {
        assert_reject_reason("81820681820482581ca38aef4ba258adf98d062db8af793b3c0a8c8ac825f92ffb81733ac8581c45acc652b46732a00f29f02b674db5011023e47d639247fe68aa40a7");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0048_ConwayUtxowFailure_MissingScriptWitnessesUTXOW_List() {
        assert_reject_reason("8182068182018203d9010280");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0049_ConwayWdrlNotDelegatedToDRep_KeyHash_KeyHash_ConwayMempoolFailure() {
        assert_reject_reason(
            "81820682820481581c97724706756be8c7ac5eeb86bc06402b70e44bb063c117fa678caa9382076148",
        );
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0050_ConwayMempoolFailure_ConwayCertsFailure_WithdrawalsNotInRewardsCERTS_List() {
        assert_reject_reason("818206828207616582028200a0");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0051_ConwayTxRefScriptsSizeTooBig() {
        assert_reject_reason("8182068183060101");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0052_ConwayTxRefScriptsSizeTooBig() {
        assert_reject_reason("8182068183060100");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0053_ConwayMempoolFailure_ConwayWdrlNotDelegatedToDRep_KeyHash_KeyHash() {
        assert_reject_reason("81820682820760820482581cdbcf0991fe989711d07ccfcf0752a3320c12cdb95a0bb6fac43234cc581cb59255ca1a3629862269e016c7be0c6110fc632091ea75aed9d8bba4");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0054_ConwayCertsFailure_WithdrawalsNotInRewardsCERTS_List_RewardAccount() {
        assert_reject_reason("8182068282028200a1581df143b29c77a36b9524cf908490eac5798394492f480ed39c16cd46ba851a0006d02283051a0005df2c19b453");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0055_ConwayUtxowFailure_PPViewHashesDontMatch_SJust_SafeHash() {
        assert_reject_reason(
            "818206818201830d815820722303d3f0c4127f8f7179faeef6c5865c686cd3833a90a2742e1c52e5403e2d80",
        )
        ;
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0056_ConwayUtxowFailure_MissingVKeyWitnessesUTXOW_List_KeyHash() {
        assert_reject_reason(
            "8182068182018202d9010281581c615ba1eac6d914f3e9f0460095bd98ecaf0c54e25039cdfbea8d6783",
        );
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0057_ConwayMempoolFailure_ConwayTreasuryValueMismatch_Coin_Coin() {
        assert_reject_reason("81820682820763e6888d83051a000b1e56191a4b");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0058_ConwayWdrlNotDelegatedToDRep_KeyHash_KeyHash_KeyHash() {
        assert_reject_reason("81820681820482581c3b7c0ec7f1f5093962dc65b03db33b0d90e0dbf6160a988826878b3f581c8952f1ce3cc36baecc375f5b9df12021d76b4ef8b345f0078f207fe4");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0059_ConwayCertsFailure_CertFailure_GovCertFailure_ConwayDRepNotRegistered() {
        assert_reject_reason(
            "8182068182028201820382018201581cce65a879625908607bdef0650cc4e4a651988525e28e93d4973927a3",
        )
        ;
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0060_ConwayMempoolFailure() {
        assert_reject_reason("8182068182076160");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0061_ConwayTxRefScriptsSizeTooBig_ConwayTxRefScriptsSizeTooBig() {
        assert_reject_reason("818206828306010183060001");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0062_ConwayGovFailure_InvalidPrevGovActionId_ProposalProcedure_ProcDeposit() {
        assert_reject_reason("8182068282038208841a000c460e581df0e6870facdc5698dc244cd3045574aec568aafe49b62349d35fc0bcc4830582582093a147effacd3320f62c8b5fb6c5e7ccea7129824c3ce6c43421f80925465c3d008282782168747470733a2f2f506d644d65717034767267704852536e4c454546482e636f6d5820a6a9e86c80d2cafde846a4f53e0d14add8b063c40f7ecdf9e4d6457fbdc2d437581c7502bfb969ceace3373c3cbf2d01c414c5292b9464ccfce774366d3482781c68747470733a2f2f3872574c47567a374e646476765833382e636f6d58200c4714c06ce990a03c87dd5b3be0b2b186a4e58d49601f705ed4668f0815a85583051a000558721a0001c0f3");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0063_ConwayWdrlNotDelegatedToDRep_KeyHash_KeyHash_ConwayTxRefScriptsSizeTooBig() {
        assert_reject_reason(
            "81820682820481581cf2a2d54cff1ec0c393f060fdc2ae9e5a7b71abb4761f04de99aa11c883062000",
        );
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0064_ConwayTreasuryValueMismatch_Coin_Coin_ConwayCertsFailure() {
        assert_reject_reason("8182068283051a0003ff8c1a0001f35282028200a1581df0fbec21d31a02eefc76bfc8f2309199173103c8157c03c8237bdfd8bc198f8e");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0065_ConwayTreasuryValueMismatch_Coin_Coin() {
        assert_reject_reason("8182068183051a000def951a000190ac");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0066_ConwayGovFailure_InvalidPolicyHash_SJust_ScriptHash() {
        assert_reject_reason("818206828203830b81581c1588aa1e2f8ed73cdf28cbf7d06241099f44f3747b50708ecc22799081581cb20a46886f87fd81f4b51689734a786f10e683e65aa6f989dc547739820481581c7fab8efda7ee7d6b04955d250f1473970ece5476871b12de0b9435ab");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0067_ConwayCertsFailure_WithdrawalsNotInRewardsCERTS_List() {
        assert_reject_reason("8182068182028200a0");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0068_ConwayCertsFailure_WithdrawalsNotInRewardsCERTS_List_RewardAccount() {
        assert_reject_reason("8182068182028200a1581de077f4d91b50ac1d97149b599f9df0632a2a492e1e59e20acd72dab75d1a00068b45");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0069_ConwayMempoolFailure() {
        assert_reject_reason("81820681820764f0aab883");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0070_ConwayTxRefScriptsSizeTooBig_ConwayWdrlNotDelegatedToDRep_KeyHash_KeyHash() {
        assert_reject_reason(
            "8182068283060100820481581c5d74ba4656ff6775b653b81cf012a26a9494475ea96bbe74eef53af8",
        );
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0071_ConwayUtxowFailure_MalformedReferenceScripts_List_ScriptHash() {
        assert_reject_reason(
            "8182068182018211d9010281581ce989cde904d0d693b63336252b5452163624ddd3beea935f21757e8b",
        );
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0072_ConwayTreasuryValueMismatch_Coin_Coin_ConwayTreasuryValueMismatch() {
        assert_reject_reason("8182068283051a000110641a00062c9683051a0009d94a1a000f3cea");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0073_ConwayTreasuryValueMismatch_Coin_Coin_ConwayCertsFailure() {
        assert_reject_reason("8182068283051a00013c031a00035c0282028200a0");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0074_ConwayMempoolFailure_ConwayUtxowFailure_ExtraRedeemers_ConwayVoting() {
        assert_reject_reason("81820682820764f0a386828201820f81820401");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0075_ConwayTreasuryValueMismatch_Coin_Coin_ConwayCertsFailure() {
        assert_reject_reason("8182068283051a000255e71a000a07cb82028200a1581de0b49233a1f4271a56406b81e8b1a732ea3d939771c1aefcd46a58e6d91a00027a44");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0076_ConwayMempoolFailure_ConwayUtxowFailure_NotAllowedSupplementalDatums_List() {
        assert_reject_reason("818206828207608201830cd901028158209ee8bb48e4ee4e6af2752bddcec3ec694b31702e40ad7a6f6c7fd67414d06f09d9010280");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0077_ConwayMempoolFailure_SO() {
        assert_reject_reason("818206818207610e");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0078_ConwayMempoolFailure() {
        assert_reject_reason("81820681820760");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0079_ConwayTreasuryValueMismatch_Coin_Coin_ConwayGovFailure() {
        assert_reject_reason("8182068283051a0004964e1a000c916982038207a0");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0080_ConwayMempoolFailure_ConwayTxRefScriptsSizeTooBig() {
        assert_reject_reason("8182068282076083062001");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0081_ConwayMempoolFailure_ConwayCertsFailure_WithdrawalsNotInRewardsCERTS_List() {
        assert_reject_reason("8182068282076082028200a0");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0082_ConwayMempoolFailure_S() {
        assert_reject_reason("8182068182076153");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0083_ConwayGovFailure_ExpirationEpochTooSmall_List_ConwayGovFailure() {
        assert_reject_reason("8182068282038207a08203820e818202581cd6d123ba0dd693a89694142a35714bc5b44f80a23ce66f4674808baf");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0084_ConwayCertsFailure_CertFailure_GovCertFailure_ConwayCommitteeIsUnknown() {
        assert_reject_reason(
            "8182068182028201820382058200581cd86ff1220850c197d0e48a4ac76ec60a9333c5e6d2cff13329931be5",
        )
        ;
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0085_ConwayTreasuryValueMismatch_Coin_Coin() {
        assert_reject_reason("8182068183051a0006297c1a000456b4");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0086_ConwayUtxowFailure_UtxoFailure_OutputTooSmallUTxO_Addr() {
        assert_reject_reason("8182068182018200820981a400583931b2d82464ee0f01a997469f62bfd2f86e2b81e2d6b57f32c76f15ee65470d8980522f46457bd0238fb07a7a86a488a78b595e013f9aea9d5601821b229f5c403dc6c2e3a1581cb0c53e2bf180858da4b64eb5598c5615bba7d723d2b604a83b7f9165a141351b1567a03e35825e8c02820058204cd8ff721542ba2426af9d0fa46638f60559d368b7d2fe4d1651c63e28327f4803d818584e82008303008283030181830301818200581cabde7a6c2f96943f3d0258dbb7ac1fc8769230b7c2b1a42e6aa10d758200581c8eab33aa0947bb8fff0df44cf26fe832ed82105a10ade2cb819ca68b");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0087_ConwayMempoolFailure_ConwayMempoolFailure() {
        assert_reject_reason("8182068282076168820760");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0088_ConwayWdrlNotDelegatedToDRep_KeyHash_KeyHash_KeyHash() {
        assert_reject_reason("81820681820482581cf1d0ecac93ac1a4aa5dad924d9390424114baa9cc07417a67c648f06581c12749381aef4f99e67b5f6237fa797f3e0910a2f0eed50625b8c6753");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0089_ConwayWdrlNotDelegatedToDRep_KeyHash_KeyHash() {
        assert_reject_reason(
            "81820681820481581c5a6cf4280e7f2704e01ba558a5e2b329e8e8c240564ecac077d16b52",
        );
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0090_ConwayGovFailure_GovActionsDoNotExist_GovActionId_TxId() {
        assert_reject_reason("8182068182038200828258206d3a5b92857fd4ec3de160e62f194525286882ef073e27c4aa6e3e21f33e0dc100825820e015c9d455993f68e65ce4201f19ef00e24807b686682f1f0248b50d3369dcdf00");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0091_ConwayGovFailure_InvalidPrevGovActionId_ProposalProcedure_ProcDeposit() {
        assert_reject_reason("8182068182038208841a000c62e0581df11901d1a388e141decd73a9d866e8b345e628a725fc1446692052172c8400f6b818001a00056731011a000f1ff1020003010401051a000d28b2061a0006cc6f0701080009d81e821b0004a0e77a55d1891b000000012a05f2000ad81e821b00033ee519c0683d1b00038d7ea4c68000111a000cad301382d81e821b45cf3fc2ad19fb0b1b016345785d8a0000d81e821b00848874675c52fd1a000186a015821b27aeb4e29a22e1ff1b43ef678ea318e38316011701181801181985d81e820001d81e821b00022b0ac69461e71b0008e1bc9bf04000d81e821b00004f99d4554e731b00005af3107a4000d81e82190a2f191388d81e821b0000c0e0a2bebdf91b00038d7ea4c68000181a8ad81e821b0000000163fdad471b00000002540be400d81e821b00037889c53344c91b00038d7ea4c68000d81e8219aaf119c350d81e821b06991bf4e85f844d1b06f05b59d3b20000d81e821a011e79ff1a05f5e100d81e821a000785071a004c4b40d81e821b05b25cbd3ce69b111b0de0b6b3a7640000d81e821b000000185b7e3c491b00000048c2739500d81e821902931903e8d81e821a0106631f1a05f5e100181c01181e19b84c181f1a000e36061820001821d81e821b189d306e53f85d3d1819f6826e68747470733a2f2f31512e636f6d58202fa17b7ce34cc8670e9448b2b1b39b1d6262b9ec23abea0df5e61b41a88d55d8");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0092_ConwayWdrlNotDelegatedToDRep_KeyHash_KeyHash_KeyHash() {
        assert_reject_reason("81820682820482581c79eafef623f0826d947fefee02b033714530053fe318529dc9f1d92a581cda3bd010fea70e217b9859095390abaffc75e6d31c8e87409bafce5c82018210d9010280");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0093_ConwayGovFailure_ZeroTreasuryWithdrawals_NoConfidence_SJust() {
        assert_reject_reason("818206818203820f8203825820bce7857d66684c376526dd9431a81b9a456fbcc26064c56c43efa05e4e99440101");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0094_ConwayTxRefScriptsSizeTooBig() {
        assert_reject_reason("8182068183060101");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0095_ConwayUtxowFailure_NotAllowedSupplementalDatums_List_SafeHash() {
        assert_reject_reason("818206828201830cd901028158203c44228aea5f895b5acd0a677dabae4923d6ea1dad941c2f04e9e17fe88a98dbd901028158206f13d3fa99ccd62b8fd11c0d15d2a0b0d64d923e867d65bdcb9b740dd101664b820481581c2476e78bd0a862d6ef3de2a178d1f4f079bd2ac0ce4731a8bb3933b9");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0096_ConwayTreasuryValueMismatch_Coin_Coin() {
        assert_reject_reason("8182068183051a000a9c7c1a000f37b5");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0097_ConwayWdrlNotDelegatedToDRep_KeyHash_KeyHash_KeyHash() {
        assert_reject_reason("81820681820482581cce3e13a895ca723a377b52b880ede8d822b34fc497f7add6df50d8d4581ce1c21d6ecb67e54622923d5db2a600a9a53f6a5eec1878a0118838e0");
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0098_ConwayWdrlNotDelegatedToDRep_KeyHash_KeyHash_ConwayMempoolFailure() {
        assert_reject_reason(
            "81820682820481581c4c74aac383eae519dca60a0dade59b7f37bb4fd57ba0cb5a2908f86682076174",
        );
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0099_ConwayWdrlNotDelegatedToDRep_KeyHash_KeyHash() {
        assert_reject_reason(
            "81820681820481581c9b13cae8380b05cf14ce9744ee556118ce6e244500732bf7ccf9d2b6",
        );
    }

    #[cfg(test)]
    #[test]
    #[allow(non_snake_case)]
    fn test_cbor_0100_ConwayUtxowFailure_InvalidMetadata_ConwayGovFailure_DisallowedProposalDuringBootstrap(
    ) {
        assert_reject_reason("81820682820181088203820c841a000236ae581df0552d969928f472c24f005ce4aaeb1a888ecc58f6e75ce0ac65a6be41810682783568747470733a2f2f3975576958314e416b6a6d764e7551775759525131705a75415059416e312d705974514568577249392e636f6d5820f70f8c1b6b57c2d9483bc98f062b088723b3f74e760419ccd4232abba36b75d3");
    }

    #[cfg(test)]
    fn try_decode_error(buffer: &mut Vec<u8>) -> Result<Option<TxValidationError>, Error> {
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

    #[cfg(test)]
    fn assert_reject_reason(reject: &str) {
        let bytes = hex::decode(reject).unwrap();
        let msg_res = try_decode_error(&mut bytes.clone());

        match msg_res {
            Ok(Some(msg)) => {
                let mut datum: Vec<u8> = Vec::new();
                // Encoding back
                encode(msg, &mut datum).expect("Error encoding");
                // assert_eq!(hex::encode(bytes), hex::encode(datum));
            }
            Ok(None) => panic!("Expected error, got None"),
            Err(err) => panic!("Error: {:?}", err),
        }
    }
}
