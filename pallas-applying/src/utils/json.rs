use pallas_primitives::{
    conway::{Nonce, NonceVariant},
    ToCanonicalJson,
};
use serde_json::json;

use crate::MultiEraProtocolParameters;

use super::{gcd, AlonzoProtParams, BabbageProtParams, ByronProtParams, ShelleyProtParams};

/// Implementation of ToCanonicalJson for the multi-era protocol parameters
/// Matches the output produced by Ogmios, documented here:
/// https://ogmios.dev/api/#operation-subscribe-/?QueryLedgerStateProtocolParameters
impl ToCanonicalJson for MultiEraProtocolParameters {
    fn to_json(&self) -> serde_json::Value {
        match self {
            MultiEraProtocolParameters::Byron(byron) => byron.to_json(),
            MultiEraProtocolParameters::Shelley(shelley) => shelley.to_json(),
            MultiEraProtocolParameters::Alonzo(alonzo) => alonzo.to_json(),
            MultiEraProtocolParameters::Babbage(babbage) => babbage.to_json(),
        }
    }
}

// Several quantities are represented as an implicit fraction over 1e15
// source: https://github.com/IntersectMBO/cardano-ledger/blob/28ab3884cac8edbb7270fd4b8628a16429d2ec9e/eras/byron/ledger/impl/src/Cardano/Chain/Common/LovelacePortion.hs#L36
pub const IMPLICIT_LOVELACE_DENOMINATOR: u64 = 10u64.pow(15);
fn to_fraction(numerator: u64, denominator: u64) -> String {
    let gcd = gcd(numerator, denominator);
    let reduced_num = numerator / gcd;
    let reduced_denom = denominator / gcd;
    if reduced_denom == 1 {
        format!("{}", reduced_num)
    } else {
        format!("{}/{}", reduced_num, reduced_denom)
    }
}

// TODO: should Nonce just be a rust enum?
fn to_entropy_string(nonce: &Nonce) -> String {
    match nonce.variant {
        NonceVariant::NeutralNonce => "neutral".to_string(),
        NonceVariant::Nonce => hex::encode(nonce.hash.unwrap()).to_string(),
    }
}

impl ToCanonicalJson for ByronProtParams {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "scriptVersion": self.script_version,
            "slotDuration": self.slot_duration,
            "maxBlockBodySize": {
                "bytes": self.max_block_size,
            },
            "maxBlockHeaderSize": {
                "bytes": self.max_header_size
            },
            "maxTransactionSize": {
                "bytes": self.max_tx_size,
            },
            "maxUpdateProposalSize": {
                "bytes": self.max_proposal_size,
            },
            // These properties are an implicit fraction over 1e15
            "multiPartyComputationThreshold": to_fraction(self.mpc_thd, IMPLICIT_LOVELACE_DENOMINATOR),
            "heavyDelegationThreshold" : to_fraction(self.heavy_del_thd, IMPLICIT_LOVELACE_DENOMINATOR),
            "updateVoteThreshold": to_fraction(self.update_vote_thd, IMPLICIT_LOVELACE_DENOMINATOR),
            "updateProposalThreshold": to_fraction(self.update_proposal_thd, IMPLICIT_LOVELACE_DENOMINATOR),
            "updateProposalTimeToLive": self.update_implicit,
            "softForkInitThreshold": to_fraction(self.soft_fork_rule.0, IMPLICIT_LOVELACE_DENOMINATOR),
            "softForkMinThreshold": to_fraction(self.soft_fork_rule.1, IMPLICIT_LOVELACE_DENOMINATOR),
            "softForkDecrementThreshold": to_fraction(self.soft_fork_rule.2, IMPLICIT_LOVELACE_DENOMINATOR),
            "minFeeConstant": {
                "ada": {
                    "lovelace": self.summand,
                }
            },
            "minFeeCoefficient": self.multiplier,
        })
    }
}

impl ToCanonicalJson for ShelleyProtParams {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "minFeeCoefficient": self.minfee_a,
            "minFeeConstant": {
                "ada": {
                    "lovelace": self.minfee_b
                }
            },
            "maxBlockBodySize": {
                "bytes": self.max_block_body_size
            },
            "maxBlockHeaderSize": {
                "bytes": self.max_block_header_size,
            },
            "maxTransactionSize": {
                "bytes": self.max_transaction_size,
            },
            "stakeCredentialDeposit": {
                "ada": {
                    "lovelace": self.key_deposit,
                }
            },
            "stakePoolDeposit": {
                "ada": {
                    "lovelace": self.pool_deposit,
                }
            },
            "desiredNumberOfStakePools": self.desired_number_of_stake_pools,
            "version": {
                "major": self.protocol_version.0,
                "minor": self.protocol_version.1,
            },
            "minUtxoDepositConstant": {
                "ada": {
                    "lovelace": self.min_utxo_value,
                }
            },
            "minStakePoolCost": {
                "ada": {
                    "lovelace": self.min_pool_cost,
                }
            },
            "monetaryExpansion": to_fraction(self.expansion_rate.numerator, self.expansion_rate.denominator),
            "treasuryExpansion": to_fraction(self.treasury_growth_rate.numerator, self.expansion_rate.denominator),
            "stakePoolRetirementEpochBound": self.maximum_epoch,
            "stakePoolPledgeInfluence": to_fraction(self.pool_pledge_influence.numerator, self.pool_pledge_influence.denominator),
            "federatedBlockProductionRatio": to_fraction(self.decentralization_constant.numerator, self.decentralization_constant.denominator),
            "extraEntropy": to_entropy_string(&self.extra_entropy),
        })
    }
}

impl ToCanonicalJson for AlonzoProtParams {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "minFeeCoefficient": self.minfee_a,
            "minFeeConstant": self.minfee_b,
            "maxBlockBodySize": {
                "bytes": self.max_block_body_size,
            },
            "maxTransactionSize": {
                "bytes": self.max_transaction_size,
            },
            "maxBlockHeaderSize": {
                "bytes": self.max_block_header_size,
            },
            "stakeCredentialDeposit": {
                "ada": {
                    "lovelace": self.key_deposit,
                }
            },
            "stakePoolDeposit": {
                "ada": {
                    "lovelace": self.pool_deposit,
                }
            },
            "desiredNumberOfStakePools": self.desired_number_of_stake_pools,
            "version": {
                "major": self.protocol_version.0,
                "minor": self.protocol_version.1,
            },
            "minStakePoolCost": {
                "ada": {
                    "lovelace": self.min_pool_cost,
                }
            },
            "minUtxoDepositConstant": {
                "ada": {
                    "lovelace": self.ada_per_utxo_byte,
                }
            },
            "plutusCostModels": self.cost_models_for_script_languages,
            "scriptExecutionPrices": {
                "memory": self.execution_costs.mem_price,
                "cpu": self.execution_costs.step_price,
            },
            "maxExecutionUnitsPerTransaction": {
                "memory": self.max_tx_ex_units.mem,
                "cpu": self.max_tx_ex_units.steps,
            },
            "maxExecutionUnitsPerBlock": {
                "memory": self.max_block_ex_units.mem,
                "cpu": self.max_block_ex_units.steps,
            },
            "maxValueSize": {
                "bytes": self.max_value_size,
            },
            "collateralPercentage": self.collateral_percentage,
            "maxCollateralInputs": self.max_collateral_inputs,
            "monetaryExpansion": to_fraction(self.expansion_rate.numerator, self.expansion_rate.denominator),
            "treasuryExpansion": to_fraction(self.treasury_growth_rate.numerator, self.treasury_growth_rate.denominator),
            "stakePoolRetirementEpochBound": self.maximum_epoch,
            "stakePoolPledgeInfluence": to_fraction(self.pool_pledge_influence.numerator, self.pool_pledge_influence.denominator),
            "federatedBlockProductionRatio": to_fraction(self.decentralization_constant.numerator, self.decentralization_constant.denominator),
            "extraEntropy": to_entropy_string(&self.extra_entropy),
        })
    }
}

impl ToCanonicalJson for BabbageProtParams {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "minFeeCoefficient": self.minfee_a,
            "minFeeConstant": self.minfee_b,
            "maxBlockBodySize": {
                "bytes": self.max_block_body_size,
            },
            "maxTransactionSize": {
                "bytes": self.max_transaction_size,
            },
            "maxBlockHeaderSize": {
                "bytes": self.max_block_header_size,
            },
            "stakeCredentialDeposit": {
                "ada": {
                    "lovelace": self.key_deposit,
                }
            },
            "stakePoolDeposit": {
                "ada": {
                    "lovelace": self.pool_deposit,
                }
            },
            "desiredNumberOfStakePools": self.desired_number_of_stake_pools,
            "version": {
                "major": self.protocol_version.0,
                "minor": self.protocol_version.1,
            },
            "minStakePoolCost": {
                "ada": {
                    "lovelace": self.min_pool_cost,
                }
            },
            "minUtxoDepositConstant": {
                "ada": {
                    "lovelace": self.ada_per_utxo_byte,
                }
            },
            "plutusCostModels": self.cost_models_for_script_languages,
            "scriptExecutionPrices": {
                "memory": self.execution_costs.mem_price,
                "cpu": self.execution_costs.step_price,
            },
            "maxExecutionUnitsPerTransaction": {
                "memory": self.max_tx_ex_units.mem,
                "cpu": self.max_tx_ex_units.steps,
            },
            "maxExecutionUnitsPerBlock": {
                "memory": self.max_block_ex_units.mem,
                "cpu": self.max_block_ex_units.steps,
            },
            "maxValueSize": {
                "bytes": self.max_value_size,
            },
            "collateralPercentage": self.collateral_percentage,
            "maxCollateralInputs": self.max_collateral_inputs,
            "monetaryExpansion": to_fraction(self.expansion_rate.numerator, self.expansion_rate.denominator),
            "treasuryExpansion": to_fraction(self.treasury_growth_rate.numerator, self.treasury_growth_rate.denominator),
            "stakePoolRetirementEpochBound": self.maximum_epoch,
            "stakePoolPledgeInfluence": to_fraction(self.pool_pledge_influence.numerator, self.pool_pledge_influence.denominator),
            "federatedBlockProductionRatio": to_fraction(self.decentralization_constant.numerator, self.decentralization_constant.denominator),
            "extraEntropy": to_entropy_string(&self.extra_entropy)
        })
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use pallas_codec::utils::KeyValuePairs;
    use pallas_crypto::hash::Hash;
    use pallas_primitives::{
        alonzo::{ExUnitPrices, Language},
        babbage::CostMdls,
        conway::{ExUnits, NonceVariant},
    };
    use pallas_traverse::update::{Nonce, RationalNumber};

    use super::*;

    #[test]
    pub fn test_byron_pparams_json() {
        let bp = MultiEraProtocolParameters::Byron(ByronProtParams {
            block_version: (0, 0, 0),
            script_version: 0,
            slot_duration: 20000,
            max_block_size: 2000000,
            max_header_size: 2000000,
            max_tx_size: 4096,
            max_proposal_size: 700,
            mpc_thd: 20000000000000,
            heavy_del_thd: 300000000000,
            update_vote_thd: 1000000000000,
            update_proposal_thd: 100000000000000,
            update_implicit: 10000,
            soft_fork_rule: (900000000000000, 600000000000000, 50000000000000),
            summand: 155381,
            multiplier: 44,
            unlock_stake_epoch: 18446744073709552,
        });
        let expected = r#"{"heavyDelegationThreshold":"3/10000","maxBlockBodySize":{"bytes":2000000},"maxBlockHeaderSize":{"bytes":2000000},"maxTransactionSize":{"bytes":4096},"maxUpdateProposalSize":{"bytes":700},"minFeeCoefficient":44,"minFeeConstant":{"ada":{"lovelace":155381}},"multiPartyComputationThreshold":"1/50","scriptVersion":0,"slotDuration":20000,"softForkDecrementThreshold":"1/20","softForkInitThreshold":"9/10","softForkMinThreshold":"3/5","updateProposalThreshold":"1/10","updateProposalTimeToLive":10000,"updateVoteThreshold":"1/1000"}"#;
        assert_eq!(expected, bp.to_json().to_string());
    }

    #[test]
    pub fn test_shelley_pparams_json() {
        let sp = MultiEraProtocolParameters::Shelley(ShelleyProtParams {
            minfee_a: 44,
            minfee_b: 155381,
            max_block_body_size: 65536,
            max_transaction_size: 16384,
            max_block_header_size: 1100,
            key_deposit: 2000000,
            pool_deposit: 500000000,
            desired_number_of_stake_pools: 150,
            protocol_version: (2, 0),
            min_utxo_value: 1000000,
            min_pool_cost: 340000000,
            expansion_rate: RationalNumber {
                numerator: 3,
                denominator: 1000,
            },
            treasury_growth_rate: RationalNumber {
                numerator: 1,
                denominator: 5,
            },
            maximum_epoch: 18,
            pool_pledge_influence: RationalNumber {
                numerator: 3,
                denominator: 10,
            },
            decentralization_constant: RationalNumber {
                numerator: 1,
                denominator: 1,
            },
            extra_entropy: Nonce {
                variant: NonceVariant::NeutralNonce,
                hash: None,
            },
        });
        let expected = r#"{"desiredNumberOfStakePools":150,"extraEntropy":"neutral","federatedBlockProductionRatio":"1","maxBlockBodySize":{"bytes":65536},"maxBlockHeaderSize":{"bytes":1100},"maxTransactionSize":{"bytes":16384},"minFeeCoefficient":44,"minFeeConstant":{"ada":{"lovelace":155381}},"minStakePoolCost":{"ada":{"lovelace":340000000}},"minUtxoDepositConstant":{"ada":{"lovelace":1000000}},"monetaryExpansion":"3/1000","stakeCredentialDeposit":{"ada":{"lovelace":2000000}},"stakePoolDeposit":{"ada":{"lovelace":500000000}},"stakePoolPledgeInfluence":"3/10","stakePoolRetirementEpochBound":18,"treasuryExpansion":"1/1000","version":{"major":2,"minor":0}}"#;
        assert_eq!(expected, sp.to_json().to_string());
    }

    #[test]
    pub fn test_alonzo_pparams_json() {
        let ap = MultiEraProtocolParameters::Alonzo(AlonzoProtParams {
            minfee_a: 44,
            minfee_b: 155381,
            max_block_body_size: 65536,
            max_transaction_size: 16384,
            max_block_header_size: 1100,
            key_deposit: 2000000,
            pool_deposit: 500000000,
            desired_number_of_stake_pools: 150,
            protocol_version: (5, 0),
            min_pool_cost: 340000000,
            expansion_rate: RationalNumber {
                numerator: 3,
                denominator: 1000,
            },
            treasury_growth_rate: RationalNumber {
                numerator: 1,
                denominator: 5,
            },
            maximum_epoch: 18,
            pool_pledge_influence: RationalNumber {
                numerator: 3,
                denominator: 10,
            },
            decentralization_constant: RationalNumber {
                numerator: 1,
                denominator: 1,
            },
            extra_entropy: Nonce {
                variant: NonceVariant::Nonce,
                hash: Some(
                    Hash::from_str(
                        "d513acca790d7ebc44c6c1b626913023dcee5a6e511a9bf840252eb047c263f8",
                    )
                    .unwrap(),
                ),
            },
            ada_per_utxo_byte: 4310,
            cost_models_for_script_languages: KeyValuePairs::Indef(vec![(
                Language::PlutusV1,
                vec![
                    197209, 0, 1, 1, 396231, 621, 0, 1, 150000, 1000, 0, 1, 150000, 32, 2477736,
                    29175, 4, 29773, 100, 29773, 100, 29773, 100, 29773, 100, 29773, 100, 29773,
                    100, 100, 100, 29773, 100, 150000, 32, 150000, 32, 150000, 32, 150000, 1000, 0,
                    1, 150000, 32, 150000, 1000, 0, 8, 148000, 425507, 118, 0, 1, 1, 150000, 1000,
                    0, 8, 150000, 112536, 247, 1, 150000, 10000, 1, 136542, 1326, 1, 1000, 150000,
                    1000, 1, 150000, 32, 150000, 32, 150000, 32, 1, 1, 150000, 1, 150000, 4,
                    103599, 248, 1, 103599, 248, 1, 145276, 1366, 1, 179690, 497, 1, 150000, 32,
                    150000, 32, 150000, 32, 150000, 32, 150000, 32, 150000, 32, 148000, 425507,
                    118, 0, 1, 1, 61516, 11218, 0, 1, 150000, 32, 148000, 425507, 118, 0, 1, 1,
                    148000, 425507, 118, 0, 1, 1, 2477736, 29175, 0, 82363, 4, 150000, 5000, 0, 1,
                    150000, 32, 197209, 0, 1, 1, 150000, 32, 150000, 32, 150000, 32, 150000, 32,
                    150000, 32, 150000, 32, 150000, 32, 3345831, 1, 1, 4,
                ],
            )]),
            execution_costs: ExUnitPrices {
                mem_price: RationalNumber {
                    numerator: 577,
                    denominator: 10000,
                },
                step_price: RationalNumber {
                    numerator: 721,
                    denominator: 10000000,
                },
            },
            max_tx_ex_units: ExUnits {
                mem: 14000000,
                steps: 10000000000,
            },
            max_block_ex_units: ExUnits {
                mem: 62000000,
                steps: 20000000000,
            },
            max_value_size: 5000,
            collateral_percentage: 150,
            max_collateral_inputs: 3,
        });
        let expected = r#"{"collateralPercentage":150,"desiredNumberOfStakePools":150,"extraEntropy":"d513acca790d7ebc44c6c1b626913023dcee5a6e511a9bf840252eb047c263f8","federatedBlockProductionRatio":"1","maxBlockBodySize":{"bytes":65536},"maxBlockHeaderSize":{"bytes":1100},"maxCollateralInputs":3,"maxExecutionUnitsPerBlock":{"cpu":20000000000,"memory":62000000},"maxExecutionUnitsPerTransaction":{"cpu":10000000000,"memory":14000000},"maxTransactionSize":{"bytes":16384},"maxValueSize":{"bytes":5000},"minFeeCoefficient":44,"minFeeConstant":155381,"minStakePoolCost":{"ada":{"lovelace":340000000}},"minUtxoDepositConstant":{"ada":{"lovelace":4310}},"monetaryExpansion":"3/1000","plutusCostModels":[["PlutusV1",[197209,0,1,1,396231,621,0,1,150000,1000,0,1,150000,32,2477736,29175,4,29773,100,29773,100,29773,100,29773,100,29773,100,29773,100,100,100,29773,100,150000,32,150000,32,150000,32,150000,1000,0,1,150000,32,150000,1000,0,8,148000,425507,118,0,1,1,150000,1000,0,8,150000,112536,247,1,150000,10000,1,136542,1326,1,1000,150000,1000,1,150000,32,150000,32,150000,32,1,1,150000,1,150000,4,103599,248,1,103599,248,1,145276,1366,1,179690,497,1,150000,32,150000,32,150000,32,150000,32,150000,32,150000,32,148000,425507,118,0,1,1,61516,11218,0,1,150000,32,148000,425507,118,0,1,1,148000,425507,118,0,1,1,2477736,29175,0,82363,4,150000,5000,0,1,150000,32,197209,0,1,1,150000,32,150000,32,150000,32,150000,32,150000,32,150000,32,150000,32,3345831,1,1,4]]],"scriptExecutionPrices":{"cpu":{"denominator":10000000,"numerator":721},"memory":{"denominator":10000,"numerator":577}},"stakeCredentialDeposit":{"ada":{"lovelace":2000000}},"stakePoolDeposit":{"ada":{"lovelace":500000000}},"stakePoolPledgeInfluence":"3/10","stakePoolRetirementEpochBound":18,"treasuryExpansion":"1/5","version":{"major":5,"minor":0}}"#;
        assert_eq!(expected, ap.to_json().to_string());
    }

    #[test]
    pub fn test_babbage_pparams_json() {
        let bp = MultiEraProtocolParameters::Babbage(BabbageProtParams {
            minfee_a: 44,
            minfee_b: 155381,
            max_block_body_size: 65536,
            max_transaction_size: 16384,
            max_block_header_size: 1100,
            key_deposit: 2000000,
            pool_deposit: 500000000,
            desired_number_of_stake_pools: 150,
            protocol_version: (8, 0),
            min_pool_cost: 340000000,
            expansion_rate: RationalNumber {
                numerator: 3,
                denominator: 1000,
            },
            treasury_growth_rate: RationalNumber {
                numerator: 1,
                denominator: 5,
            },
            maximum_epoch: 18,
            pool_pledge_influence: RationalNumber {
                numerator: 3,
                denominator: 10,
            },
            decentralization_constant: RationalNumber {
                numerator: 1,
                denominator: 1,
            },
            extra_entropy: Nonce {
                variant: NonceVariant::Nonce,
                hash: Some(
                    Hash::from_str(
                        "d513acca790d7ebc44c6c1b626913023dcee5a6e511a9bf840252eb047c263f8",
                    )
                    .unwrap(),
                ),
            },
            ada_per_utxo_byte: 4310,
            cost_models_for_script_languages: CostMdls {
                plutus_v1: Some(vec![
                    197209, 0, 1, 1, 396231, 621, 0, 1, 150000, 1000, 0, 1, 150000, 32, 2477736,
                    29175, 4, 29773, 100, 29773, 100, 29773, 100, 29773, 100, 29773, 100, 29773,
                    100, 100, 100, 29773, 100, 150000, 32, 150000, 32, 150000, 32, 150000, 1000, 0,
                    1, 150000, 32, 150000, 1000, 0, 8, 148000, 425507, 118, 0, 1, 1, 150000, 1000,
                    0, 8, 150000, 112536, 247, 1, 150000, 10000, 1, 136542, 1326, 1, 1000, 150000,
                    1000, 1, 150000, 32, 150000, 32, 150000, 32, 1, 1, 150000, 1, 150000, 4,
                    103599, 248, 1, 103599, 248, 1, 145276, 1366, 1, 179690, 497, 1, 150000, 32,
                    150000, 32, 150000, 32, 150000, 32, 150000, 32, 150000, 32, 148000, 425507,
                    118, 0, 1, 1, 61516, 11218, 0, 1, 150000, 32, 148000, 425507, 118, 0, 1, 1,
                    148000, 425507, 118, 0, 1, 1, 2477736, 29175, 0, 82363, 4, 150000, 5000, 0, 1,
                    150000, 32, 197209, 0, 1, 1, 150000, 32, 150000, 32, 150000, 32, 150000, 32,
                    150000, 32, 150000, 32, 150000, 32, 3345831, 1, 1, 4,
                ]),
                plutus_v2: Some(vec![
                    197209, 0, 1, 1, 396231, 621, 0, 1, 150000, 1000, 0, 1, 150000, 32, 2477736,
                    29175, 4, 29773, 100, 29773, 100, 29773, 100, 29773, 100, 29773, 100, 29773,
                    100, 100, 100, 29773, 100, 150000, 32, 150000, 32, 150000, 32, 150000, 1000, 0,
                    1, 150000, 32, 150000, 1000, 0, 8, 148000, 425507, 118, 0, 1, 1, 150000, 1000,
                    0, 8, 150000, 112536, 247, 1, 150000, 10000, 1, 136542, 1326, 1, 1000, 150000,
                    1000, 1, 150000, 32, 150000, 32, 150000, 32, 1, 1, 150000, 1, 150000, 4,
                    103599, 248, 1, 103599, 248, 1, 145276, 1366, 1, 179690, 497, 1, 150000, 32,
                    150000, 32, 150000, 32, 150000, 32, 150000, 32, 150000, 32, 148000, 425507,
                    118, 0, 1, 1, 61516, 11218, 0, 1, 150000, 32, 148000, 425507, 118, 0, 1, 1,
                    148000, 425507, 118, 0, 1, 1, 2477736, 29175, 0, 82363, 4, 150000, 5000, 0, 1,
                    150000, 32, 197209, 0, 1, 1, 150000, 32, 150000, 32, 150000, 32, 150000, 32,
                    150000, 32, 150000, 32, 150000, 32, 3345831, 1, 1, 4,
                ]),
            },
            execution_costs: ExUnitPrices {
                mem_price: RationalNumber {
                    numerator: 577,
                    denominator: 10000,
                },
                step_price: RationalNumber {
                    numerator: 721,
                    denominator: 10000000,
                },
            },
            max_tx_ex_units: ExUnits {
                mem: 14000000,
                steps: 10000000000,
            },
            max_block_ex_units: ExUnits {
                mem: 62000000,
                steps: 20000000000,
            },
            max_value_size: 5000,
            collateral_percentage: 150,
            max_collateral_inputs: 3,
        });
        let expected = r#"{"collateralPercentage":150,"desiredNumberOfStakePools":150,"extraEntropy":"d513acca790d7ebc44c6c1b626913023dcee5a6e511a9bf840252eb047c263f8","federatedBlockProductionRatio":"1","maxBlockBodySize":{"bytes":65536},"maxBlockHeaderSize":{"bytes":1100},"maxCollateralInputs":3,"maxExecutionUnitsPerBlock":{"cpu":20000000000,"memory":62000000},"maxExecutionUnitsPerTransaction":{"cpu":10000000000,"memory":14000000},"maxTransactionSize":{"bytes":16384},"maxValueSize":{"bytes":5000},"minFeeCoefficient":44,"minFeeConstant":155381,"minStakePoolCost":{"ada":{"lovelace":340000000}},"minUtxoDepositConstant":{"ada":{"lovelace":4310}},"monetaryExpansion":"3/1000","plutusCostModels":{"plutus_v1":[197209,0,1,1,396231,621,0,1,150000,1000,0,1,150000,32,2477736,29175,4,29773,100,29773,100,29773,100,29773,100,29773,100,29773,100,100,100,29773,100,150000,32,150000,32,150000,32,150000,1000,0,1,150000,32,150000,1000,0,8,148000,425507,118,0,1,1,150000,1000,0,8,150000,112536,247,1,150000,10000,1,136542,1326,1,1000,150000,1000,1,150000,32,150000,32,150000,32,1,1,150000,1,150000,4,103599,248,1,103599,248,1,145276,1366,1,179690,497,1,150000,32,150000,32,150000,32,150000,32,150000,32,150000,32,148000,425507,118,0,1,1,61516,11218,0,1,150000,32,148000,425507,118,0,1,1,148000,425507,118,0,1,1,2477736,29175,0,82363,4,150000,5000,0,1,150000,32,197209,0,1,1,150000,32,150000,32,150000,32,150000,32,150000,32,150000,32,150000,32,3345831,1,1,4],"plutus_v2":[197209,0,1,1,396231,621,0,1,150000,1000,0,1,150000,32,2477736,29175,4,29773,100,29773,100,29773,100,29773,100,29773,100,29773,100,100,100,29773,100,150000,32,150000,32,150000,32,150000,1000,0,1,150000,32,150000,1000,0,8,148000,425507,118,0,1,1,150000,1000,0,8,150000,112536,247,1,150000,10000,1,136542,1326,1,1000,150000,1000,1,150000,32,150000,32,150000,32,1,1,150000,1,150000,4,103599,248,1,103599,248,1,145276,1366,1,179690,497,1,150000,32,150000,32,150000,32,150000,32,150000,32,150000,32,148000,425507,118,0,1,1,61516,11218,0,1,150000,32,148000,425507,118,0,1,1,148000,425507,118,0,1,1,2477736,29175,0,82363,4,150000,5000,0,1,150000,32,197209,0,1,1,150000,32,150000,32,150000,32,150000,32,150000,32,150000,32,150000,32,3345831,1,1,4]},"scriptExecutionPrices":{"cpu":{"denominator":10000000,"numerator":721},"memory":{"denominator":10000,"numerator":577}},"stakeCredentialDeposit":{"ada":{"lovelace":2000000}},"stakePoolDeposit":{"ada":{"lovelace":500000000}},"stakePoolPledgeInfluence":"3/10","stakePoolRetirementEpochBound":18,"treasuryExpansion":"1/5","version":{"major":8,"minor":0}}"#;
        assert_eq!(expected, bp.to_json().to_string());
    }
}
