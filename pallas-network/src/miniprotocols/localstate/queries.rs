use pallas_codec::minicbor::{decode, encode, Decode, Decoder, Encode, Encoder};

// https://github.com/input-output-hk/ouroboros-consensus/blob/main/ouroboros-consensus-cardano/src/shelley/Ouroboros/Consensus/Shelley/Ledger/Query.hs
#[derive(Debug, Clone, PartialEq)]
#[repr(u16)]
pub enum BlockQuery {
    GetLedgerTip,
    GetEpochNo,
    // GetNonMyopicMemberRewards(()),
    GetCurrentPParams,
    GetProposedPParamsUpdates,
    GetStakeDistribution,
    // GetUTxOByAddress(()),
    // GetUTxOWhole, (Response too large for now)
    // DebugEpochState, (Response too large for now)
    // GetCBOR(()),
    // GetFilteredDelegationsAndRewardAccounts(()),
    GetGenesisConfig,
    // DebugNewEpochState, (Response too large for now)
    DebugChainDepState,
    GetRewardProvenance,
    // GetUTxOByTxIn(()),
    GetStakePools,
    // GetStakePoolParams(()),
    GetRewardInfoPools,
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
        e.array(2)?;
        e.u16(0)?;
        e.array(2)?;
        /*
            TODO: Think this is era or something? First fetch era with
            [3, [0, [2, [1]]]], then use it here?
        */
        e.u16(5)?;
        match self {
            BlockQuery::GetLedgerTip => {
                e.array(1)?;
                e.u16(0)?;
            }
            BlockQuery::GetEpochNo => {
                e.array(1)?;
                e.u16(1)?;
            }
            // BlockQuery::GetNonMyopicMemberRewards(()) => {
            //     e.array(X)?;
            //     e.u16(2)?;
            // }
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
            // BlockQuery::GetUTxOByAddress(()) => {
            //     e.array(X)?;
            //     e.u16(6)?;
            // }
            // BlockQuery::GetUTxOWhole => {
            //     e.array(1)?;
            //     e.u16(7)?;
            // }
            // BlockQuery::DebugEpochState => {
            //     e.array(1)?;
            //     e.u16(8)?;
            // }
            // BlockQuery::GetCBOR(()) => {
            //     e.array(X)?;
            //     e.u16(9)?;
            // }
            // BlockQuery::GetFilteredDelegationsAndRewardAccounts(()) => {
            //     e.array(X)?;
            //     e.u16(10)?;
            // }
            BlockQuery::GetGenesisConfig => {
                e.array(1)?;
                e.u16(11)?;
            }
            // BlockQuery::DebugNewEpochState => {
            //     e.array(1)?;
            //     e.u16(12)?;
            // }
            BlockQuery::DebugChainDepState => {
                e.array(1)?;
                e.u16(13)?;
            }
            BlockQuery::GetRewardProvenance => {
                e.array(1)?;
                e.u16(14)?;
            }
            // BlockQuery::GetUTxOByTxIn(()) => {
            //     e.array(X)?;
            //     e.u16(15)?;
            // }
            BlockQuery::GetStakePools => {
                e.array(1)?;
                e.u16(16)?;
            }
            // BlockQuery::GetStakePoolParams(()) => {
            //     e.array(X)?;
            //     e.u16(17)?;
            // }
            BlockQuery::GetRewardInfoPools => {
                e.array(1)?;
                e.u16(18)?;
            } /* BlockQuery::GetPoolState(()) => {
               *     e.array(X)?;
               *     e.u16(19)?;
               * } */

              /* BlockQuery::GetStakeSnapshots(()) => {
               *     e.array(X)?;
               *     e.u16(20)?;
               * } */

              /* BlockQuery::GetPoolDistr(()) => {
               *     e.array(X)?;
               *     e.u16(21)?;
               * } */

              /* BlockQuery::GetStakeDelegDeposits(()) => {
               *     e.array(X)?;
               *     e.u16(22)?;
               * } */

              /* BlockQuery::GetConstitutionHash => {
               *     e.array(1)?;
               *     e.u16(23)?;
               * } */
        }
        Ok(())
    }
}

impl<'b> Decode<'b, ()> for BlockQuery {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        d.array()?;
        d.u16()?;
        d.array()?;
        d.u16()?;
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

#[derive(Debug, Clone, PartialEq)]
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
        d.array()?;
        let tag = d.u16()?;

        match tag {
            0 => {
                let inner = d.decode()?;
                Ok(Self::BlockQuery(inner))
            }
            1 => Ok(Self::GetSystemStart),
            2 => Ok(Self::GetChainBlockNo),
            3 => Ok(Self::GetChainPoint),
            _ => Err(decode::Error::message("invalid tag")),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BlockQueryResponse {
    LedgerTip(Vec<u8>),
    EpochNo(EpochNo),
    CurrentPParams(Vec<u8>),
    ProposedPParamsUpdates(Vec<u8>),
    StakeDistribution(Vec<u8>),
    GenesisConfig(Vec<u8>),
    DebugChainDepState(Vec<u8>),
    RewardProvenance(Vec<u8>),
    StakePools(Vec<u8>),
    RewardInfoPools(Vec<u8>),
}

impl Encode<()> for BlockQueryResponse {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        match self {
            Self::LedgerTip(bytes) => {
                e.array(2)?;
                e.u16(0)?;
                e.bytes(bytes)?;
                Ok(())
            }
            Self::EpochNo(bytes) => {
                e.array(2)?;
                e.u16(1)?;
                // e.bytes(bytes)?;
                Ok(())
            }
            Self::CurrentPParams(bytes) => {
                e.array(2)?;
                e.u16(3)?;
                e.bytes(bytes)?;
                Ok(())
            }
            Self::ProposedPParamsUpdates(bytes) => {
                e.array(2)?;
                e.u16(4)?;
                e.bytes(bytes)?;
                Ok(())
            }
            Self::StakeDistribution(bytes) => {
                e.array(2)?;
                e.u16(5)?;
                e.bytes(bytes)?;
                Ok(())
            }
            Self::GenesisConfig(bytes) => {
                e.array(2)?;
                e.u16(11)?;
                e.bytes(bytes)?;
                Ok(())
            }
            Self::DebugChainDepState(bytes) => {
                e.array(2)?;
                e.u16(13)?;
                e.bytes(bytes)?;
                Ok(())
            }
            Self::RewardProvenance(bytes) => {
                e.array(2)?;
                e.u16(14)?;
                e.bytes(bytes)?;
                Ok(())
            }
            Self::StakePools(bytes) => {
                e.array(2)?;
                e.u16(16)?;
                e.bytes(bytes)?;
                Ok(())
            }
            Self::RewardInfoPools(bytes) => {
                e.array(2)?;
                e.u16(17)?;
                e.bytes(bytes)?;
                Ok(())
            }
        }
    }
}

impl<'b> Decode<'b, ()> for BlockQueryResponse {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        d.array()?;
        let tag = d.u16()?;

        match tag {
            0 => Ok(Self::LedgerTip(d.bytes()?.to_vec())),
            1 => Ok(Self::EpochNo(EpochNo(d.bytes()?.to_vec()))),
            3 => Ok(Self::CurrentPParams(d.bytes()?.to_vec())),
            4 => Ok(Self::ProposedPParamsUpdates(d.bytes()?.to_vec())),
            5 => Ok(Self::StakeDistribution(d.bytes()?.to_vec())),
            11 => Ok(Self::GenesisConfig(d.bytes()?.to_vec())),
            13 => Ok(Self::DebugChainDepState(d.bytes()?.to_vec())),
            14 => Ok(Self::RewardProvenance(d.bytes()?.to_vec())),
            16 => Ok(Self::StakePools(d.bytes()?.to_vec())),
            17 => Ok(Self::RewardInfoPools(d.bytes()?.to_vec())),
            _ => Err(decode::Error::message(
                "invalid (size, tag) for lsq response",
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EpochNo(pub Vec<u8>);

impl EpochNo {
    pub fn to_vec(&self) -> Vec<u8> {
        self.0.clone()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Response {
    BlockQuery(BlockQueryResponse),
    SystemStart(Vec<u8>),
    ChainBlockNo(Vec<u8>),
    ChainPoint(Vec<u8>),
    Generic(Vec<u8>),
}

impl Encode<()> for Response {
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
            Self::SystemStart(bytes) => {
                e.array(1)?;
                e.u16(1)?;
                e.bytes(bytes)?;
                Ok(())
            }
            Self::ChainBlockNo(bytes) => {
                e.array(1)?;
                e.u16(2)?;
                e.bytes(bytes)?;
                Ok(())
            }
            Self::ChainPoint(bytes) => {
                e.array(1)?;
                e.u16(3)?;
                e.bytes(bytes)?;
                Ok(())
            }
            Response::Generic(_) => todo!(),
        }
    }
}

impl<'b> Decode<'b, ()> for Response {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        d.array()?;
        let tag = d.u16()?;

        match tag {
            0 => {
                let inner = d.decode()?;
                Ok(Self::BlockQuery(inner))
            }
            _ => Err(decode::Error::message("unknown tag")),
        }
    }
}
