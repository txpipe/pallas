use super::*;
use pallas_codec::minicbor::{decode, encode, Decode, Decoder, Encode, Encoder};

impl<'b, C> Encode<C> for MultiassetA {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut C,
    ) -> Result<(), encode::Error<W::Error>> {
        todo!()
    }
}

impl<'b, C> Decode<'b, C> for MultiassetA {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut C) -> Result<Self, decode::Error> {
        match d.datatype()? {
            minicbor::data::Type::U8 => {
                let x = d.u8()?;
                Ok(Self::U8(x))
            }
            minicbor::data::Type::U16 => {
                let x = d.u16()?;
                Ok(Self::U16(x))
            }
            minicbor::data::Type::U32 => {
                let x = d.u32()?;
                Ok(Self::U32(x))
            }
            minicbor::data::Type::U64 => {
                let x = d.u64()?;
                Ok(Self::U64(x))
            }
            minicbor::data::Type::Bytes => {
                let x = d.decode()?;
                Ok(Self::Bytes(x))
            }
            _ => Err(decode::Error::message("invalid tag")),
        }
    }
}

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
                println!("encode GetUTxOByAddress");
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
            BlockQuery::GetUTxOByTxIn(_) => {
                e.array(2)?;
                e.u16(15)?;
                e.encode(2)?;
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
            BlockQuery::GetStakeSnapshots(x) => {
                e.array(2)?;
                e.u16(20)?;
                e.encode(x)?;
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
        println!("decode BlockQuery");
        d.array()?;

        match d.u16()? {
            0 => Ok(Self::GetLedgerTip),
            1 => Ok(Self::GetEpochNo),
            // 2 => Ok(Self::GetNonMyopicMemberRewards(())),
            3 => Ok(Self::GetCurrentPParams),
            4 => Ok(Self::GetProposedPParamsUpdates),
            5 => Ok(Self::GetStakeDistribution),
            6 => {
                println!("decode GetUTxOByAddress");
                let value = d.decode()?;
                Ok(Self::GetUTxOByAddress(value))
            }
            // 7 => Ok(Self::GetUTxOWhole),
            // 8 => Ok(Self::DebugEpochState),
            // 9 => Ok(Self::GetCBOR(())),
            // 10 => Ok(Self::GetFilteredDelegationsAndRewardAccounts(())),
            11 => Ok(Self::GetGenesisConfig),
            // 12 => Ok(Self::DebugNewEpochState),
            13 => Ok(Self::DebugChainDepState),
            14 => Ok(Self::GetRewardProvenance),
            // 15 => Ok(Self::GetUTxOByTxIn(())),
            16 => Ok(Self::GetStakePools),
            // 17 => Ok(Self::GetStakePoolParams(())),
            18 => Ok(Self::GetRewardInfoPools),
            // 19 => Ok(Self::GetPoolState(())),
            // 20 => Ok(Self::GetStakeSnapshots(())),
            // 21 => Ok(Self::GetPoolDistr(())),
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
