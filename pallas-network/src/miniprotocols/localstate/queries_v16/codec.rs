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
            BlockQuery::GetConstitutionHash => {
                e.array(1)?;
                e.u16(23)?;
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
            // 2 => Ok(Self::GetNonMyopicMemberRewards(())),
            3 => Ok(Self::GetCurrentPParams),
            4 => Ok(Self::GetProposedPParamsUpdates),
            5 => Ok(Self::GetStakeDistribution),
            6 => Ok(Self::GetUTxOByAddress(d.decode()?)),
            7 => Ok(Self::GetUTxOWhole),
            // 8 => Ok(Self::DebugEpochState),
            9 => Ok(Self::GetCBOR(d.decode()?)),
            10 => Ok(Self::GetFilteredDelegationsAndRewardAccounts(d.decode()?)),
            11 => Ok(Self::GetGenesisConfig),
            // 12 => Ok(Self::DebugNewEpochState),
            13 => Ok(Self::DebugChainDepState),
            14 => Ok(Self::GetRewardProvenance),
            15 => Ok(Self::GetUTxOByTxIn(d.decode()?)),
            16 => Ok(Self::GetStakePools),
            // 17 => Ok(Self::GetStakePoolParams(())),
            18 => Ok(Self::GetRewardInfoPools),
            19 => Ok(Self::GetPoolState(d.decode()?)),
            20 => Ok(Self::GetStakeSnapshots(d.decode()?)),
            21 => Ok(Self::GetPoolDistr(d.decode()?)),
            // 22 => Ok(Self::GetStakeDelegDeposits(())),
            // 23 => Ok(Self::GetConstitutionHash),
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
