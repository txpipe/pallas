use pallas_codec::minicbor::{decode, encode, Decode, Decoder, Encode, Encoder};

use super::Query;

// https://github.com/input-output-hk/ouroboros-consensus/blob/main/ouroboros-consensus-cardano/src/shelley/Ouroboros/Consensus/Shelley/Ledger/Query.hs
#[derive(Debug, Clone)]
#[repr(u8)]
pub enum BlockQuery {
    GetLedgerTip,
    GetEpochNo,
    // GetNonMyopicMemberRewards(()),
    // GetCurrentPParams,
    // GetProposedPParamsUpdates,
    // GetStakeDistribution,
    // GetUTxOByAddress(()),
    // GetUTxOWhole,
    // DebugEpochState,
    // GetCBOR(()),
    // GetFilteredDelegationsAndRewardAccounts(()),
    // GetGenesisConfig,
    // DebugNewEpochState,
    // DebugChainDepState,
    // GetRewardProvenance,
    // GetUTxOByTxIn(()),
    // GetStakePools,
    // GetStakePoolParams(()),
    // GetRewardInfoPools,
    // GetPoolState(()),
    // GetStakeSnapshots(()),
    // GetPoolDistr(()),
    // GetStakeDelegDeposits(()),
    // GetConstitutionHash,
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
                Ok(())
            }
            BlockQuery::GetEpochNo => {
                e.array(1)?;
                e.u16(1)?;
                Ok(())
            }
        }
    }
}

impl<'b> Decode<'b, ()> for BlockQuery {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        d.array()?;

        match d.u16()? {
            0 => Ok(Self::GetLedgerTip),
            1 => Ok(Self::GetEpochNo),
            _ => todo!(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Request {
    BlockQuery(BlockQuery),
    GetSystemStart,
    GetChainBlockNo,
    GetChainPoint,
}

impl Encode<()> for Request {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        match self {
            Self::BlockQuery(q) => {
                e.array(2)?;
                e.u16(0)?;
                e.encode(q)?;

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
        let size = match d.array()? {
            Some(l) => l,
            None => return Err(decode::Error::message("unexpected indefinite len list")),
        };

        let tag = d.u16()?;

        match (size, tag) {
            (2, 0) => Ok(Self::BlockQuery(d.decode()?)), // decode block query
            (1, 1) => Ok(Self::GetSystemStart),
            (1, 2) => Ok(Self::GetChainBlockNo),
            (1, 3) => Ok(Self::GetChainPoint),
            _ => {
                return Err(decode::Error::message(
                    "invalid (size, tag) for lsq request",
                ))
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct GenericResponse(Vec<u8>);

impl Encode<()> for GenericResponse {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        e.writer_mut()
            .write_all(&self.0)
            .map_err(|e| encode::Error::write(e))
    }
}

impl<'b> Decode<'b, ()> for GenericResponse {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        let start = d.position();
        d.skip()?;
        let end = d.position();
        let slice = &d.input()[start..end];
        let vec = slice.to_vec();
        Ok(GenericResponse(vec))
    }
}

/// Queries available as of N2C V16
#[derive(Debug, Clone)]
pub struct QueryV16 {}

impl Query for QueryV16 {
    type Request = Request;
    type Response = GenericResponse;
}
