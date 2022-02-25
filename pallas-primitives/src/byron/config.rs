//! Parsing of Byron configuration data

use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenesisFile {
    pub avvm_distr: HashMap<String, String>,
    pub block_version_data: BlockVersionData,
    // ftsSeed
    // protocolConsts
    pub start_time: u64,
    // bootStakeholders
    // heavyDelegation
    // nonAvvmBalances
    // vssCerts
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockVersionData {
    //heavyDelThd
    //maxBlockSize
    //maxHeaderSize
    //maxProposalSize
    //maxTxSize
    //mpcThd
    //scriptVersion
    //slotDuration
    //softforkRule
    pub tx_fee_policy: TxFeePolicy,
    //unlockStakeEpoch
    //updateImplicit
    //updateProposalThd
    //updateVoteThd
}

#[derive(Debug, Deserialize)]
pub struct TxFeePolicy {
    pub multiplier: String,
    pub summand: String,
}

#[cfg(test)]
mod tests {
    use super::GenesisFile;

    #[test]
    fn config_parses_correctly() {
        let json = include_str!("test_data/genesis.json");
        let model: GenesisFile = serde_json::from_str(json).unwrap();

        assert_eq!(model.avvm_distr.len(), 100)
    }
}
