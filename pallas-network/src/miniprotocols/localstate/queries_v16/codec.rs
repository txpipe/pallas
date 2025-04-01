use super::*;
use pallas_codec::minicbor::{data::Tag, decode, encode, Decode, Decoder, Encode, Encoder};

impl Encode<()> for BlockQuery {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        match self {
            BlockQuery::GetLedgerTip => {
                e.array(1)?;
                e.u16(0)?;
            }
            BlockQuery::GetEpochNo => {
                e.array(1)?;
                e.u16(1)?;
            }
            BlockQuery::GetNonMyopicMemberRewards(x) => {
                e.array(2)?;
                e.u16(2)?;
                e.encode(x)?;
            }
            BlockQuery::GetCurrentPParams => {
                e.array(1)?;
                e.u16(3)?;
            }
            BlockQuery::GetProposedPParamsUpdates => {
                e.array(1)?;
                e.u16(4)?;
            }
            BlockQuery::GetStakeDistribution => {
                e.array(1)?;
                e.u16(5)?;
            }
            BlockQuery::GetUTxOByAddress(addrs) => {
                e.array(2)?;
                e.u16(6)?;
                e.encode(addrs)?;
            }
            BlockQuery::GetUTxOWhole => {
                e.encode((7,))?;
            }
            BlockQuery::DebugEpochState => {
                e.array(1)?;
                e.u16(8)?;
            }
            BlockQuery::GetCBOR(x) => {
                e.array(2)?;
                e.u16(9)?;
                e.encode(x)?;
            }
            BlockQuery::GetFilteredDelegationsAndRewardAccounts(x) => {
                e.array(2)?;
                e.u16(10)?;
                e.encode(x)?;
            }
            BlockQuery::GetGenesisConfig => {
                e.array(1)?;
                e.u16(11)?;
            }
            BlockQuery::DebugNewEpochState => {
                e.array(1)?;
                e.u16(12)?;
            }
            BlockQuery::DebugChainDepState => {
                e.array(1)?;
                e.u16(13)?;
            }
            BlockQuery::GetRewardProvenance => {
                e.array(1)?;
                e.u16(14)?;
            }
            BlockQuery::GetUTxOByTxIn(txin) => {
                e.array(2)?;
                e.u16(15)?;
                e.encode(txin)?;
            }
            BlockQuery::GetStakePools => {
                e.array(1)?;
                e.u16(16)?;
            }
            BlockQuery::GetStakePoolParams(x) => {
                e.array(2)?;
                e.u16(17)?;
                e.encode(x)?;
            }
            BlockQuery::GetRewardInfoPools => {
                e.array(1)?;
                e.u16(18)?;
            }
            BlockQuery::GetPoolState(x) => {
                e.array(2)?;
                e.u16(19)?;
                e.encode(x)?;
            }
            BlockQuery::GetStakeSnapshots(pools) => {
                e.array(2)?;
                e.u16(20)?;
                e.encode(pools)?;
            }
            BlockQuery::GetPoolDistr(x) => {
                e.array(2)?;
                e.u16(21)?;
                e.encode(x)?;
            }
            BlockQuery::GetStakeDelegDeposits(x) => {
                e.array(2)?;
                e.u16(22)?;
                e.encode(x)?;
            }
            BlockQuery::GetConstitution => {
                e.array(1)?;
                e.u16(23)?;
            }
            BlockQuery::GetGovState => {
                e.array(1)?;
                e.u16(24)?;
            }
            BlockQuery::GetDRepState(x) => {
                e.array(2)?;
                e.u16(25)?;
                e.encode(x)?;
            }
            BlockQuery::GetDRepStakeDistr(dreps) => {
                e.array(2)?;
                e.u16(26)?;
                e.encode(dreps)?;
            }
            BlockQuery::GetCommitteeMembersState(set1, set2, status) => {
                e.array(4)?;
                e.u16(27)?;
                e.encode(set1)?;
                e.encode(set2)?;
                e.encode(status)?;
            }
            BlockQuery::GetFilteredVoteDelegatees(addrs) => {
                e.array(2)?;
                e.u16(28)?;
                e.encode(addrs)?;
            }
            BlockQuery::GetAccountState => {
                e.array(1)?;
                e.u16(29)?;
            }
            BlockQuery::GetSPOStakeDistr(pools) => {
                e.array(2)?;
                e.u16(30)?;
                e.encode(pools)?;
            }
            BlockQuery::GetProposals(proposals) => {
                e.array(2)?;
                e.u16(31)?;
                e.encode(proposals)?;
            }
            BlockQuery::GetRatifyState => {
                e.array(1)?;
                e.u16(32)?;
            }
            BlockQuery::GetFuturePParams => {
                e.array(1)?;
                e.u16(33)?;
            }
            BlockQuery::GetBigLedgerPeerSnapshot => {
                e.array(1)?;
                e.u16(34)?;
            }
        }
        Ok(())
    }
}

impl<'b> Decode<'b, ()> for BlockQuery {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        d.array()?;

        match d.u16()? {
            0 => Ok(Self::GetLedgerTip),
            1 => Ok(Self::GetEpochNo),
            2 => Ok(Self::GetNonMyopicMemberRewards(d.decode()?)),
            3 => Ok(Self::GetCurrentPParams),
            4 => Ok(Self::GetProposedPParamsUpdates),
            5 => Ok(Self::GetStakeDistribution),
            6 => Ok(Self::GetUTxOByAddress(d.decode()?)),
            7 => Ok(Self::GetUTxOWhole),
            8 => Ok(Self::DebugEpochState),
            9 => Ok(Self::GetCBOR(d.decode()?)),
            10 => Ok(Self::GetFilteredDelegationsAndRewardAccounts(d.decode()?)),
            11 => Ok(Self::GetGenesisConfig),
            12 => Ok(Self::DebugNewEpochState),
            13 => Ok(Self::DebugChainDepState),
            14 => Ok(Self::GetRewardProvenance),
            15 => Ok(Self::GetUTxOByTxIn(d.decode()?)),
            16 => Ok(Self::GetStakePools),
            17 => Ok(Self::GetStakePoolParams(d.decode()?)),
            18 => Ok(Self::GetRewardInfoPools),
            19 => Ok(Self::GetPoolState(d.decode()?)),
            20 => Ok(Self::GetStakeSnapshots(d.decode()?)),
            21 => Ok(Self::GetPoolDistr(d.decode()?)),
            22 => Ok(Self::GetStakeDelegDeposits(d.decode()?)),
            23 => Ok(Self::GetConstitution),
            24 => Ok(Self::GetGovState),
            25 => Ok(Self::GetDRepState(d.decode()?)),
            26 => Ok(Self::GetDRepStakeDistr(d.decode()?)),
            27 => Ok(Self::GetCommitteeMembersState(
                d.decode()?,
                d.decode()?,
                d.decode()?,
            )),
            28 => Ok(Self::GetFilteredVoteDelegatees(d.decode()?)),
            29 => Ok(Self::GetAccountState),
            30 => Ok(Self::GetSPOStakeDistr(d.decode()?)),
            31 => Ok(Self::GetProposals(d.decode()?)),
            32 => Ok(Self::GetRatifyState),
            33 => Ok(Self::GetFuturePParams),
            34 => Ok(Self::GetBigLedgerPeerSnapshot),
            _ => unreachable!(),
        }
    }
}

impl Encode<()> for HardForkQuery {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        match self {
            HardForkQuery::GetInterpreter => {
                e.encode((0,))?;
            }
            HardForkQuery::GetCurrentEra => {
                e.encode((1,))?;
            }
        }

        Ok(())
    }
}

impl<'b> Decode<'b, ()> for HardForkQuery {
    fn decode(d: &mut Decoder<'b>, _: &mut ()) -> Result<Self, decode::Error> {
        d.array()?;
        let tag = d.u16()?;
        match tag {
            0 => Ok(Self::GetInterpreter),
            1 => Ok(Self::GetCurrentEra),
            _ => Err(decode::Error::message("invalid tag")),
        }
    }
}

impl Encode<()> for LedgerQuery {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        match self {
            LedgerQuery::BlockQuery(era, q) => {
                e.encode((0, (era, q)))?;
            }
            LedgerQuery::HardForkQuery(q) => {
                e.encode((2, q))?;
            }
        }

        Ok(())
    }
}

impl<'b> Decode<'b, ()> for LedgerQuery {
    fn decode(d: &mut Decoder<'b>, _: &mut ()) -> Result<Self, decode::Error> {
        d.array()?;
        let tag = d.u16()?;
        match tag {
            0 => {
                let (era, q) = d.decode()?;
                Ok(Self::BlockQuery(era, q))
            }
            2 => {
                let q = d.decode()?;
                Ok(Self::HardForkQuery(q))
            }
            _ => Err(decode::Error::message("invalid tag")),
        }
    }
}

impl Encode<()> for Request {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        match self {
            Self::LedgerQuery(q) => {
                e.encode((0, q))?;
                Ok(())
            }
            Self::GetSystemStart => {
                e.array(1)?;
                e.u16(1)?;
                Ok(())
            }
            Self::GetChainBlockNo => {
                e.array(1)?;
                e.u16(2)?;
                Ok(())
            }
            Self::GetChainPoint => {
                e.array(1)?;
                e.u16(3)?;
                Ok(())
            }
        }
    }
}

impl<'b> Decode<'b, ()> for Request {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        d.array()?;
        let tag = d.u16()?;

        match tag {
            0 => Ok(Self::LedgerQuery(d.decode()?)),
            1 => Ok(Self::GetSystemStart),
            2 => Ok(Self::GetChainBlockNo),
            3 => Ok(Self::GetChainPoint),
            _ => Err(decode::Error::message("invalid tag")),
        }
    }
}

impl<'b, C> minicbor::decode::Decode<'b, C> for Value {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        match d.datatype()? {
            minicbor::data::Type::U8 => Ok(Value::Coin(d.decode_with(ctx)?)),
            minicbor::data::Type::U16 => Ok(Value::Coin(d.decode_with(ctx)?)),
            minicbor::data::Type::U32 => Ok(Value::Coin(d.decode_with(ctx)?)),
            minicbor::data::Type::U64 => Ok(Value::Coin(d.decode_with(ctx)?)),
            minicbor::data::Type::Array => {
                d.array()?;
                let coin = d.decode_with(ctx)?;
                let multiasset = d.decode_with(ctx)?;
                Ok(Value::Multiasset(coin, multiasset))
            }
            _ => Err(minicbor::decode::Error::message(
                "unknown cbor data type for Value enum",
            )),
        }
    }
}

impl<C> minicbor::encode::Encode<C> for Value {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            Value::Coin(coin) => {
                e.encode_with(coin, ctx)?;
            }
            Value::Multiasset(coin, other) => {
                e.array(2)?;
                e.encode_with(coin, ctx)?;
                e.encode_with(other, ctx)?;
            }
        };

        Ok(())
    }
}

impl<'b, C> minicbor::decode::Decode<'b, C> for RationalNumber {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        d.tag()?;
        d.array()?;

        Ok(RationalNumber {
            numerator: d.decode_with(ctx)?,
            denominator: d.decode_with(ctx)?,
        })
    }
}

impl<C> minicbor::encode::Encode<C> for RationalNumber {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.tag(Tag::new(30))?;
        e.array(2)?;
        e.encode_with(self.numerator, ctx)?;
        e.encode_with(self.denominator, ctx)?;

        Ok(())
    }
}

impl<'b, C> minicbor::decode::Decode<'b, C> for TransactionOutput {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        match d.datatype()? {
            minicbor::data::Type::Map => Ok(TransactionOutput::Current(d.decode_with(ctx)?)),
            minicbor::data::Type::Array => Ok(TransactionOutput::Legacy(d.decode_with(ctx)?)),
            _ => Err(minicbor::decode::Error::message(
                "unknown cbor data type for TransactionOutput enum",
            )),
        }
    }
}

impl<C> minicbor::encode::Encode<C> for TransactionOutput {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            TransactionOutput::Current(map) => {
                e.encode_with(map, ctx)?;
            }
            TransactionOutput::Legacy(array) => {
                e.encode_with(array, ctx)?;
            }
        };

        Ok(())
    }
}

impl<'b, S, T, C> minicbor::decode::Decode<'b, C> for Either<S, T>
where
    S: minicbor::Decode<'b, C> + Ord,
    T: minicbor::Decode<'b, C> + Ord,
{
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        d.array()?;
        match d.u8()? {
            0 => Ok(Either::Left(d.decode_with(ctx)?)),
            1 => Ok(Either::Right(d.decode_with(ctx)?)),
            _ => Err(minicbor::decode::Error::message(
                "unknown cbor variant for `Either` enum",
            )),
        }
    }
}

impl<S, T, C> minicbor::encode::Encode<C> for Either<S, T>
where
    S: Clone + minicbor::Encode<C>,
    T: Clone + minicbor::Encode<C>,
{
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.array(2)?;
        match self {
            Either::Left(x) => {
                e.u8(0)?;
                e.encode_with(x, ctx)?;
            }
            Either::Right(x) => {
                e.u8(1)?;
                e.encode_with(x, ctx)?;
            }
        }

        Ok(())
    }
}

impl<'b, C> minicbor::decode::Decode<'b, C> for DRep {
    fn decode(
        d: &mut minicbor::Decoder<'b>,
        _ctx: &mut C,
    ) -> Result<Self, minicbor::decode::Error> {
        d.array()?;
        match d.u16()? {
            0 => Ok(Self::KeyHash(d.decode()?)),
            1 => Ok(Self::ScriptHash(d.decode()?)),
            2 => Ok(Self::AlwaysAbstain),
            3 => Ok(Self::AlwaysNoConfidence),
            _ => unreachable!(),
        }
    }
}

impl<C> minicbor::encode::Encode<C> for DRep {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        _ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            DRep::KeyHash(bytes) => {
                e.array(2)?;
                e.u16(0)?;
                e.encode(bytes)?;
            }
            DRep::ScriptHash(bytes) => {
                e.array(2)?;
                e.u16(1)?;
                e.encode(bytes)?;
            }
            DRep::AlwaysAbstain => {
                e.array(1)?;
                e.u16(2)?;
            }
            DRep::AlwaysNoConfidence => {
                e.array(1)?;
                e.u16(3)?;
            }
        }
        Ok(())
    }
}

impl<'b, C> minicbor::decode::Decode<'b, C> for CommitteeAuthorization {
    fn decode(
        d: &mut minicbor::Decoder<'b>,
        _ctx: &mut C,
    ) -> Result<Self, minicbor::decode::Error> {
        d.array()?;
        match d.u16()? {
            0 => Ok(Self::HotCredential(d.decode()?)),
            1 => Ok(Self::MemberResigned(d.decode()?)),
            _ => unreachable!(),
        }
    }
}

impl<C> minicbor::encode::Encode<C> for CommitteeAuthorization {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        _ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            CommitteeAuthorization::HotCredential(credential) => {
                e.array(2)?;
                e.u16(0)?;
                e.encode(credential)?;
            }
            CommitteeAuthorization::MemberResigned(anchor) => {
                e.array(2)?;
                e.u16(1)?;
                e.encode(anchor)?;
            }
        }
        Ok(())
    }
}

impl<'b, C> minicbor::decode::Decode<'b, C> for FuturePParams {
    fn decode(
        d: &mut minicbor::Decoder<'b>,
        _ctx: &mut C,
    ) -> Result<Self, minicbor::decode::Error> {
        d.array()?;
        match d.u16()? {
            0 => Ok(FuturePParams::NoPParamsUpdate),
            1 => Ok(FuturePParams::DefinitePParamsUpdate(d.decode()?)),
            2 => Ok(FuturePParams::PotentialPParamsUpdate(d.decode()?)),
            _ => unreachable!(),
        }
    }
}

impl<C> minicbor::encode::Encode<C> for FuturePParams {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        _ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            FuturePParams::NoPParamsUpdate => {
                e.array(1)?;
                e.u16(0)?;
            }
            FuturePParams::DefinitePParamsUpdate(param) => {
                e.array(2)?;
                e.u16(1)?;
                e.encode(param)?;
            }
            FuturePParams::PotentialPParamsUpdate(maybe_param) => {
                e.array(2)?;
                e.u16(2)?;
                e.encode(maybe_param)?;
            }
        }
        Ok(())
    }
}

impl<'b, C> minicbor::decode::Decode<'b, C> for GovAction {
    fn decode(
        d: &mut minicbor::Decoder<'b>,
        _ctx: &mut C,
    ) -> Result<Self, minicbor::decode::Error> {
        d.array()?;
        match d.u16()? {
            0 => Ok(Self::ParameterChange(d.decode()?, d.decode()?, d.decode()?)),
            1 => Ok(Self::HardForkInitiation(d.decode()?, d.decode()?)),
            2 => Ok(Self::TreasuryWithdrawals(d.decode()?, d.decode()?)),
            3 => Ok(Self::NoConfidence(d.decode()?)),
            4 => Ok(Self::UpdateCommittee(
                d.decode()?,
                d.decode()?,
                d.decode()?,
                d.decode()?,
            )),
            5 => Ok(Self::NewConstitution(d.decode()?, d.decode()?)),
            6 => Ok(Self::InfoAction),
            _ => unreachable!(),
        }
    }
}

impl<C> minicbor::encode::Encode<C> for GovAction {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        _ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            GovAction::ParameterChange(sm_id, params, hash) => {
                e.array(4)?;
                e.u16(0)?;
                e.encode(sm_id)?;
                e.encode(params)?;
                e.encode(hash)?;
            }
            GovAction::HardForkInitiation(sm_id, version) => {
                e.array(3)?;
                e.u16(1)?;
                e.encode(sm_id)?;
                e.encode(version)?;
            }
            GovAction::TreasuryWithdrawals(withdrawals, hash) => {
                e.array(3)?;
                e.u16(2)?;
                e.encode(withdrawals)?;
                e.encode(hash)?;
            }
            GovAction::NoConfidence(sm_id) => {
                e.array(2)?;
                e.u16(3)?;
                e.encode(sm_id)?;
            }
            GovAction::UpdateCommittee(sm_id, removed, added, threshold) => {
                e.array(5)?;
                e.u16(4)?;
                e.encode(sm_id)?;
                e.encode(removed)?;
                e.encode(added)?;
                e.encode(threshold)?;
            }
            GovAction::NewConstitution(sm_id, constitution) => {
                e.array(3)?;
                e.u16(5)?;
                e.encode(sm_id)?;
                e.encode(constitution)?;
            }
            GovAction::InfoAction => {
                e.array(1)?;
                e.u16(6)?;
            }
        }
        Ok(())
    }
}

impl<'b, C> minicbor::decode::Decode<'b, C> for HotCredAuthStatus {
    fn decode(
        d: &mut minicbor::Decoder<'b>,
        _ctx: &mut C,
    ) -> Result<Self, minicbor::decode::Error> {
        d.array()?;
        match d.u16()? {
            0 => Ok(Self::MemberAuthorized(d.decode()?)),
            1 => Ok(Self::MemberNotAuthorized),
            2 => Ok(Self::MemberResigned(d.decode()?)),
            _ => Err(minicbor::decode::Error::message(
                "Unknown variant for HotCredAuthStatus",
            )),
        }
    }
}

impl<C> minicbor::encode::Encode<C> for HotCredAuthStatus {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        _ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            HotCredAuthStatus::MemberAuthorized(credential) => {
                e.array(2)?;
                e.u16(0)?;
                e.encode(credential)?;
            }
            HotCredAuthStatus::MemberNotAuthorized => {
                e.array(1)?;
                e.u16(1)?;
            }
            HotCredAuthStatus::MemberResigned(anchor) => {
                e.array(2)?;
                e.u16(2)?;
                e.encode(anchor)?;
            }
        }
        Ok(())
    }
}

impl<'b, C> minicbor::decode::Decode<'b, C> for NextEpochChange {
    fn decode(
        d: &mut minicbor::Decoder<'b>,
        _ctx: &mut C,
    ) -> Result<Self, minicbor::decode::Error> {
        d.array()?;
        match d.u16()? {
            0 => Ok(Self::ToBeEnacted),
            1 => Ok(Self::ToBeRemoved),
            2 => Ok(Self::NoChangeExpected),
            3 => Ok(Self::ToBeExpired),
            4 => Ok(Self::TermAdjusted(d.decode()?)),
            _ => unreachable!(),
        }
    }
}

impl<C> minicbor::encode::Encode<C> for NextEpochChange {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        _ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            Self::ToBeEnacted => {
                e.array(1)?;
                e.u16(0)?;
            }
            Self::ToBeRemoved => {
                e.array(1)?;
                e.u16(1)?;
            }
            Self::NoChangeExpected => {
                e.array(1)?;
                e.u16(2)?;
            }
            Self::ToBeExpired => {
                e.array(1)?;
                e.u16(3)?;
            }
            Self::TermAdjusted(epoch) => {
                e.array(2)?;
                e.u16(4)?;
                e.encode(epoch)?;
            }
        }
        Ok(())
    }
}

impl<'b, C> minicbor::Decode<'b, C> for CostModels {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let models: KeyValuePairs<u64, CostModel> = d.decode_with(ctx)?;

        let mut plutus_v1 = None;
        let mut plutus_v2 = None;
        let mut plutus_v3 = None;
        let mut unknown: Vec<(u64, CostModel)> = Vec::new();

        for (k, v) in models.iter() {
            match k {
                0 => plutus_v1 = Some(v.clone()),
                1 => plutus_v2 = Some(v.clone()),
                2 => plutus_v3 = Some(v.clone()),
                _ => unknown.push((*k, v.clone())),
            }
        }

        Ok(Self {
            plutus_v1,
            plutus_v2,
            plutus_v3,
            unknown: unknown.into(),
        })
    }
}

#[cfg(test)]
pub mod tests {
    use pallas_codec::minicbor;

    /// Decode/encode roundtrip tests for the localstate example queries/results.
    #[test]
    #[cfg(feature = "blueprint")]
    fn test_api_example_roundtrip() {
        use crate::miniprotocols::localstate::{
            queries_v16::{Request, SystemStart},
            Message,
        };
        use pallas_codec::utils::AnyCbor;

        // TODO: scan for examples
        let examples = [(
            include_str!(
                "../../../../../cardano-blueprint/src/api/examples/getSystemStart/query.cbor"
            ),
            include_str!(
                "../../../../../cardano-blueprint/src/api/examples/getSystemStart/result.cbor"
            ),
        )];
        // TODO: DRY with other decode/encode roundtrips
        for (idx, (query_str, result_str)) in examples.iter().enumerate() {
            println!("Roundtrip query {idx}");
            roundtrips_with(query_str, |q| match q {
                Message::Query(cbor) => {
                    let request = minicbor::decode(&cbor[..]).unwrap_or_else(|e| {
                        panic!("error decoding cbor from query message: {e:?}")
                    });
                    match request {
                        Request::GetSystemStart => {
                            return Message::Query(AnyCbor::from_encode(request))
                        }
                        _ => panic!("unexpected query type"),
                    }
                }
                _ => panic!("unexpected message type"),
            });

            println!("Roundtrip result {idx}");
            roundtrips_with(result_str, |q| match q {
                Message::Result(cbor) => {
                    return minicbor::decode::<SystemStart>(&cbor[..]).unwrap_or_else(|e| {
                        panic!("error decoding cbor from query message: {e:?}")
                    });
                }
                _ => panic!("unexpected message type"),
            });
        }
    }

    // TODO: DRY with other decode/encode roundtripss
    /// Decode a value of type T, transform it to U and encode that again to form a roundtrip.
    fn roundtrips_with<T, U>(message_str: &str, transform: fn(T) -> U)
    where
        T: for<'b> minicbor::Decode<'b, ()> + std::fmt::Debug,
        U: std::fmt::Debug + minicbor::Encode<()>,
    {
        use pallas_codec::minicbor;

        let bytes = hex::decode(message_str).unwrap_or_else(|e| panic!("bad message file: {e:?}"));

        let value: T =
            minicbor::decode(&bytes[..]).unwrap_or_else(|e| panic!("error decoding cbor: {e:?}"));
        println!("Decoded value: {:#?}", value);

        let result: U = transform(value);
        println!("Transformed to: {:#?}", result);

        let bytes2 =
            minicbor::to_vec(result).unwrap_or_else(|e| panic!("error encoding cbor: {e:?}"));

        assert!(
            bytes.eq(&bytes2),
            "re-encoded bytes didn't match original file"
        );
    }
}
