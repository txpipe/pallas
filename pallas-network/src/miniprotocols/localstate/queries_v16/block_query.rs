//! Iterative operations for the unary GetCBOR query wrapper.
use super::BlockQuery;
impl BlockQuery {
    /// Extracts the wrapped query, returning the original value on a mismatch.
    pub fn into_cbor(mut self) -> Result<Box<Self>, Self> {
        match &mut self {
            Self::GetCBOR(x) => Ok(std::mem::replace(x, Box::new(Self::GetLedgerTip))),
            _ => Err(self),
        }
    }
}
impl Clone for BlockQuery {
    fn clone(&self) -> Self {
        let mut leaf = self;
        let mut depth = 0;
        while let Self::GetCBOR(x) = leaf {
            leaf = x;
            depth += 1;
        }
        let mut copy = match leaf {
            Self::GetLedgerTip => Self::GetLedgerTip,
            Self::GetEpochNo => Self::GetEpochNo,
            Self::GetNonMyopicMemberRewards(x0) => Self::GetNonMyopicMemberRewards(x0.clone()),
            Self::GetCurrentPParams => Self::GetCurrentPParams,
            Self::GetProposedPParamsUpdates => Self::GetProposedPParamsUpdates,
            Self::GetStakeDistribution => Self::GetStakeDistribution,
            Self::GetUTxOByAddress(x0) => Self::GetUTxOByAddress(x0.clone()),
            Self::GetUTxOWhole => Self::GetUTxOWhole,
            Self::DebugEpochState => Self::DebugEpochState,
            Self::GetFilteredDelegationsAndRewardAccounts(x0) => {
                Self::GetFilteredDelegationsAndRewardAccounts(x0.clone())
            }
            Self::GetGenesisConfig => Self::GetGenesisConfig,
            Self::DebugNewEpochState => Self::DebugNewEpochState,
            Self::DebugChainDepState => Self::DebugChainDepState,
            Self::GetRewardProvenance => Self::GetRewardProvenance,
            Self::GetUTxOByTxIn(x0) => Self::GetUTxOByTxIn(x0.clone()),
            Self::GetStakePools => Self::GetStakePools,
            Self::GetStakePoolParams(x0) => Self::GetStakePoolParams(x0.clone()),
            Self::GetRewardInfoPools => Self::GetRewardInfoPools,
            Self::GetPoolState(x0) => Self::GetPoolState(x0.clone()),
            Self::GetStakeSnapshots(x0) => Self::GetStakeSnapshots(x0.clone()),
            Self::GetPoolDistr(x0) => Self::GetPoolDistr(x0.clone()),
            Self::GetStakeDelegDeposits(x0) => Self::GetStakeDelegDeposits(x0.clone()),
            Self::GetConstitution => Self::GetConstitution,
            Self::GetGovState => Self::GetGovState,
            Self::GetDRepState(x0) => Self::GetDRepState(x0.clone()),
            Self::GetDRepStakeDistr(x0) => Self::GetDRepStakeDistr(x0.clone()),
            Self::GetCommitteeMembersState(x0, x1, x2) => {
                Self::GetCommitteeMembersState(x0.clone(), x1.clone(), x2.clone())
            }
            Self::GetFilteredVoteDelegatees(x0) => Self::GetFilteredVoteDelegatees(x0.clone()),
            Self::GetAccountState => Self::GetAccountState,
            Self::GetSPOStakeDistr(x0) => Self::GetSPOStakeDistr(x0.clone()),
            Self::GetProposals(x0) => Self::GetProposals(x0.clone()),
            Self::GetRatifyState => Self::GetRatifyState,
            Self::GetFuturePParams => Self::GetFuturePParams,
            Self::GetBigLedgerPeerSnapshot => Self::GetBigLedgerPeerSnapshot,
            Self::GetLedgerPeerSnapshot(x0) => Self::GetLedgerPeerSnapshot(*x0),
            Self::GetPoolDistr2(x0) => Self::GetPoolDistr2(x0.clone()),
            Self::GetStakeDistribution2 => Self::GetStakeDistribution2,
            Self::GetDRepsDelegations(x0) => Self::GetDRepsDelegations(x0.clone()),
            Self::GetCBOR(_) => unreachable!("wrappers were peeled"),
        };
        for _ in 0..depth {
            copy = Self::GetCBOR(Box::new(copy));
        }
        copy
    }
}
impl PartialEq for BlockQuery {
    fn eq(&self, other: &Self) -> bool {
        let (mut a, mut b) = (self, other);
        while let (Self::GetCBOR(x), Self::GetCBOR(y)) = (a, b) {
            a = x;
            b = y;
        }
        match (a, b) {
            (Self::GetLedgerTip, Self::GetLedgerTip) => true,
            (Self::GetEpochNo, Self::GetEpochNo) => true,
            (Self::GetNonMyopicMemberRewards(a0), Self::GetNonMyopicMemberRewards(b0)) => a0 == b0,
            (Self::GetCurrentPParams, Self::GetCurrentPParams) => true,
            (Self::GetProposedPParamsUpdates, Self::GetProposedPParamsUpdates) => true,
            (Self::GetStakeDistribution, Self::GetStakeDistribution) => true,
            (Self::GetUTxOByAddress(a0), Self::GetUTxOByAddress(b0)) => a0 == b0,
            (Self::GetUTxOWhole, Self::GetUTxOWhole) => true,
            (Self::DebugEpochState, Self::DebugEpochState) => true,
            (
                Self::GetFilteredDelegationsAndRewardAccounts(a0),
                Self::GetFilteredDelegationsAndRewardAccounts(b0),
            ) => a0 == b0,
            (Self::GetGenesisConfig, Self::GetGenesisConfig) => true,
            (Self::DebugNewEpochState, Self::DebugNewEpochState) => true,
            (Self::DebugChainDepState, Self::DebugChainDepState) => true,
            (Self::GetRewardProvenance, Self::GetRewardProvenance) => true,
            (Self::GetUTxOByTxIn(a0), Self::GetUTxOByTxIn(b0)) => a0 == b0,
            (Self::GetStakePools, Self::GetStakePools) => true,
            (Self::GetStakePoolParams(a0), Self::GetStakePoolParams(b0)) => a0 == b0,
            (Self::GetRewardInfoPools, Self::GetRewardInfoPools) => true,
            (Self::GetPoolState(a0), Self::GetPoolState(b0)) => a0 == b0,
            (Self::GetStakeSnapshots(a0), Self::GetStakeSnapshots(b0)) => a0 == b0,
            (Self::GetPoolDistr(a0), Self::GetPoolDistr(b0)) => a0 == b0,
            (Self::GetStakeDelegDeposits(a0), Self::GetStakeDelegDeposits(b0)) => a0 == b0,
            (Self::GetConstitution, Self::GetConstitution) => true,
            (Self::GetGovState, Self::GetGovState) => true,
            (Self::GetDRepState(a0), Self::GetDRepState(b0)) => a0 == b0,
            (Self::GetDRepStakeDistr(a0), Self::GetDRepStakeDistr(b0)) => a0 == b0,
            (
                Self::GetCommitteeMembersState(a0, a1, a2),
                Self::GetCommitteeMembersState(b0, b1, b2),
            ) => a0 == b0 && a1 == b1 && a2 == b2,
            (Self::GetFilteredVoteDelegatees(a0), Self::GetFilteredVoteDelegatees(b0)) => a0 == b0,
            (Self::GetAccountState, Self::GetAccountState) => true,
            (Self::GetSPOStakeDistr(a0), Self::GetSPOStakeDistr(b0)) => a0 == b0,
            (Self::GetProposals(a0), Self::GetProposals(b0)) => a0 == b0,
            (Self::GetRatifyState, Self::GetRatifyState) => true,
            (Self::GetFuturePParams, Self::GetFuturePParams) => true,
            (Self::GetBigLedgerPeerSnapshot, Self::GetBigLedgerPeerSnapshot) => true,
            (Self::GetLedgerPeerSnapshot(a0), Self::GetLedgerPeerSnapshot(b0)) => a0 == b0,
            (Self::GetPoolDistr2(a0), Self::GetPoolDistr2(b0)) => a0 == b0,
            (Self::GetStakeDistribution2, Self::GetStakeDistribution2) => true,
            (Self::GetDRepsDelegations(a0), Self::GetDRepsDelegations(b0)) => a0 == b0,
            _ => false,
        }
    }
}
