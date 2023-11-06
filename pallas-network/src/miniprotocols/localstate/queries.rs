use pallas_codec::minicbor::{decode, encode, Decode, Decoder, Encode, Encoder};

use super::Query;

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
            } // BlockQuery::GetPoolState(()) => {
              //     e.array(X)?;
              //     e.u16(19)?;
              // }
              // BlockQuery::GetStakeSnapshots(()) => {
              //     e.array(X)?;
              //     e.u16(20)?;
              // }
              // BlockQuery::GetPoolDistr(()) => {
              //     e.array(X)?;
              //     e.u16(21)?;
              // }
              // BlockQuery::GetStakeDelegDeposits(()) => {
              //     e.array(X)?;
              //     e.u16(22)?;
              // }
              // BlockQuery::GetConstitutionHash => {
              //     e.array(1)?;
              //     e.u16(23)?;
              // }
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
        let size = match d.array()? {
            Some(l) => l,
            None => return Err(decode::Error::message("unexpected indefinite len list")),
        };

        let tag = d.u16()?;

        match (size, tag) {
            (2, 0) => Ok(Self::BlockQuery(d.decode()?)),
            (1, 1) => Ok(Self::GetSystemStart),
            (1, 2) => Ok(Self::GetChainBlockNo),
            (1, 3) => Ok(Self::GetChainPoint),
            _ => Err(decode::Error::message(
                "invalid (size, tag) for lsq request",
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum QueryResponse {
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
    Generic(Vec<u8>), // Server response to unknown query
}

impl QueryResponse {
    fn to_vec(&self) -> Vec<u8> {
        match self {
            QueryResponse::LedgerTip(data) => data.clone(),
            QueryResponse::EpochNo(data) => data.clone().to_vec(),
            QueryResponse::CurrentPParams(data) => data.clone(),
            QueryResponse::ProposedPParamsUpdates(data) => data.clone(),
            QueryResponse::StakeDistribution(data) => data.clone(),
            QueryResponse::GenesisConfig(data) => data.clone(),
            QueryResponse::DebugChainDepState(data) => data.clone(),
            QueryResponse::RewardProvenance(data) => data.clone(),
            QueryResponse::StakePools(data) => data.clone(),
            QueryResponse::RewardInfoPools(data) => data.clone(),
            QueryResponse::Generic(data) => data.clone(),
        }
    }
}

impl Encode<()> for QueryResponse {
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
                e.bytes(&bytes.to_vec())?;
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
            Self::Generic(bytes) => {
                e.array(2)?;
                e.u16(18)?;
                e.bytes(bytes)?;
                Ok(())
            }
        }
    }
}

impl<'b> Decode<'b, ()> for QueryResponse {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        let size = d
            .array()?
            .ok_or_else(|| decode::Error::message("unexpected indefinite len list"))?;

        let tag = d.u16()?;

        match (size, tag) {
            (2, 0) => Ok(Self::LedgerTip(d.bytes()?.to_vec())),
            (2, 1) => Ok(Self::EpochNo(EpochNo(d.bytes()?.to_vec()))),
            (2, 3) => Ok(Self::CurrentPParams(d.bytes()?.to_vec())),
            (2, 4) => Ok(Self::ProposedPParamsUpdates(d.bytes()?.to_vec())),
            (2, 5) => Ok(Self::StakeDistribution(d.bytes()?.to_vec())),
            (2, 11) => Ok(Self::GenesisConfig(d.bytes()?.to_vec())),
            (2, 13) => Ok(Self::DebugChainDepState(d.bytes()?.to_vec())),
            (2, 14) => Ok(Self::RewardProvenance(d.bytes()?.to_vec())),
            (2, 16) => Ok(Self::StakePools(d.bytes()?.to_vec())),
            (2, 17) => Ok(Self::RewardInfoPools(d.bytes()?.to_vec())),
            (2, 18) => Ok(Self::Generic(d.input().to_vec())),
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
        self.0.clone().to_vec()
    }
}

impl Encode<()> for EpochNo {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        e.bytes(&self.0)?;
        Ok(())
    }
}

impl<'b> Decode<'b, ()> for EpochNo {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        Ok(Self(d.bytes()?.to_vec()))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Generic(pub Vec<u8>);

impl Generic {
    pub fn to_vec(&self) -> Vec<u8> {
        self.0.clone()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Response {
    SystemStart(Vec<u8>),
    ChainBlockNo(Vec<u8>),
    ChainPoint(Vec<u8>),
    Query(QueryResponse),
}

impl Response {
    pub fn to_vec(&self) -> Vec<u8> {
        match self {
            // Response::BlockQuery(block_query_response) => block_query_response.to_vec(),
            Response::SystemStart(data) => data.clone(),
            Response::ChainBlockNo(data) => data.clone(),
            Response::ChainPoint(data) => data.clone(),
            Response::Query(query) => query.clone().to_vec(),
        }
    }
}

impl Encode<()> for Response {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        match self {
            Self::Query(q) => {
                e.encode(q)?;
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

impl<'b> Decode<'b, ()> for Response {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        d.set_position(d.position() - 1);
        let label = d.u16()?;
        d.skip()?;
        let vec = d.input().to_vec();

        match label {
            4 => Ok(Self::Query(QueryResponse::Generic(vec))),
            _ => Err(decode::Error::message(
                "invalid (size, tag) for lsq response",
            )),
        }
    }
}

/// Queries available as of N2C V16
#[derive(Debug, Clone)]
pub struct QueryV16 {}

impl Query for QueryV16 {
    type Request = Request;
    type Response = Response;

    fn request_signal(request: Self::Request) -> u16 {
        match request {
            Request::BlockQuery(BlockQuery::GetEpochNo) => 0,
            Request::BlockQuery(BlockQuery::GetLedgerTip) => 1,
            Request::BlockQuery(BlockQuery::GetCurrentPParams) => 2,
            Request::BlockQuery(BlockQuery::GetProposedPParamsUpdates) => 3,
            Request::BlockQuery(BlockQuery::GetStakeDistribution) => 4,
            Request::BlockQuery(BlockQuery::DebugChainDepState) => 5,
            Request::BlockQuery(BlockQuery::GetGenesisConfig) => 6,
            Request::BlockQuery(BlockQuery::GetRewardProvenance) => 7,
            Request::BlockQuery(BlockQuery::GetStakePools) => 8,
            Request::BlockQuery(BlockQuery::GetRewardInfoPools) => 9,
            Request::GetSystemStart => 10,
            Request::GetChainBlockNo => 11,
            Request::GetChainPoint => 12,
        }
    }

    fn map_response(signal: u16, response: Vec<u8>) -> Self::Response {
        match signal {
            0 => Self::Response::Query(QueryResponse::EpochNo(EpochNo(response.clone().to_vec()))),
            8 => Self::Response::Query(QueryResponse::StakePools(response)),
            _ => Self::Response::Query(QueryResponse::Generic(response.clone().to_vec())),
        }
    }

    fn to_vec(response: Self::Response) -> Vec<u8> {
        match response {
            // Response::BlockQuery(block_query_response) => block_query_response.to_vec(),
            Response::SystemStart(data) => data.clone(),
            Response::ChainBlockNo(data) => data.clone(),
            Response::ChainPoint(data) => data.clone(),
            Response::Query(QueryResponse::EpochNo(epoch_no)) => epoch_no.to_vec(),
            Response::Query(QueryResponse::Generic(data)) => data.clone(),
            _ => panic!("unimplemented"),
        }
    }
}
