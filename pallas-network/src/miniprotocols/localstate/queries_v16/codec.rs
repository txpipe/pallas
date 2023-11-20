use super::*;
use pallas_codec::minicbor::{decode, encode, Decode, Decoder, Encode, Encoder};

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
            BlockQuery::GetUTxOByAddress(x) => {
                e.array(2)?;
                e.u16(6)?;
                e.encode(x)?;
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
        d.array()?;

        match d.u16()? {
            0 => Ok(Self::GetLedgerTip),
            1 => Ok(Self::GetEpochNo),
            // 2 => Ok(Self::GetNonMyopicMemberRewards(())),
            3 => Ok(Self::GetCurrentPParams),
            4 => Ok(Self::GetProposedPParamsUpdates),
            5 => Ok(Self::GetStakeDistribution),
            // 6 => Ok(Self::GetUTxOByAddress(())),
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
    fn decode(_d: &mut Decoder<'b>, _: &mut ()) -> Result<Self, decode::Error> {
        todo!()
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
    fn decode(_d: &mut Decoder<'b>, _: &mut ()) -> Result<Self, decode::Error> {
        todo!()
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
