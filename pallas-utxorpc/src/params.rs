use pallas_applying::MultiEraProtocolParameters;
use pallas_primitives::UnitInterval;
use utxorpc_spec::utxorpc::v1alpha::cardano as u5c;

use crate::{LedgerContext, Mapper};

impl<C: LedgerContext> Mapper<C> {
    pub fn map_pparams(&self, pparams: MultiEraProtocolParameters) -> u5c::PParams {
        match pparams {
            MultiEraProtocolParameters::Alonzo(params) => u5c::PParams {
                max_tx_size: params.max_transaction_size.into(),
                max_block_body_size: params.max_block_body_size.into(),
                max_block_header_size: params.max_block_header_size.into(),
                min_fee_coefficient: params.minfee_a.into(),
                min_fee_constant: params.minfee_b.into(),
                coins_per_utxo_byte: params.ada_per_utxo_byte,
                stake_key_deposit: params.key_deposit,
                pool_deposit: params.pool_deposit,
                desired_number_of_pools: params.desired_number_of_stake_pools.into(),
                pool_influence: Some(u5c::RationalNumber {
                    numerator: params.pool_pledge_influence.numerator as i32,
                    denominator: params.pool_pledge_influence.denominator as u32,
                }),
                monetary_expansion: Some(u5c::RationalNumber {
                    numerator: params.expansion_rate.numerator as i32,
                    denominator: params.expansion_rate.denominator as u32,
                }),
                treasury_expansion: Some(u5c::RationalNumber {
                    numerator: params.treasury_growth_rate.numerator as i32,
                    denominator: params.treasury_growth_rate.denominator as u32,
                }),
                min_pool_cost: params.min_pool_cost,
                protocol_version: Some(u5c::ProtocolVersion {
                    major: params.protocol_version.0 as u32,
                    minor: params.protocol_version.1 as u32,
                }),
                max_value_size: params.max_value_size.into(),
                collateral_percentage: params.collateral_percentage.into(),
                max_collateral_inputs: params.max_collateral_inputs.into(),
                prices: Some(u5c::ExPrices {
                    steps: Some(u5c::RationalNumber {
                        numerator: params.execution_costs.step_price.numerator as i32,
                        denominator: params.execution_costs.step_price.denominator as u32,
                    }),
                    memory: Some(u5c::RationalNumber {
                        numerator: params.execution_costs.mem_price.numerator as i32,
                        denominator: params.execution_costs.mem_price.denominator as u32,
                    }),
                }),
                max_execution_units_per_transaction: Some(u5c::ExUnits {
                    memory: params.max_tx_ex_units.mem,
                    steps: params.max_tx_ex_units.steps,
                }),
                max_execution_units_per_block: Some(u5c::ExUnits {
                    memory: params.max_block_ex_units.mem,
                    steps: params.max_block_ex_units.steps,
                }),
                cost_models: u5c::CostModels {
                    // Only plutusv1.
                    plutus_v1: params
                        .cost_models_for_script_languages
                        .first()
                        .map(|(_, data)| u5c::CostModel {
                            values: data.to_vec(),
                        }),
                    ..Default::default()
                }
                .into(),
                ..Default::default()
            },
            MultiEraProtocolParameters::Shelley(params) => u5c::PParams {
                max_tx_size: params.max_transaction_size.into(),
                max_block_body_size: params.max_block_body_size.into(),
                max_block_header_size: params.max_block_header_size.into(),
                min_fee_coefficient: params.minfee_a.into(),
                min_fee_constant: params.minfee_b.into(),
                stake_key_deposit: params.key_deposit,
                pool_deposit: params.pool_deposit,
                desired_number_of_pools: params.desired_number_of_stake_pools.into(),
                pool_influence: Some(u5c::RationalNumber {
                    numerator: params.pool_pledge_influence.numerator as i32,
                    denominator: params.pool_pledge_influence.denominator as u32,
                }),
                monetary_expansion: Some(u5c::RationalNumber {
                    numerator: params.expansion_rate.numerator as i32,
                    denominator: params.expansion_rate.denominator as u32,
                }),
                treasury_expansion: Some(u5c::RationalNumber {
                    numerator: params.treasury_growth_rate.numerator as i32,
                    denominator: params.treasury_growth_rate.denominator as u32,
                }),
                min_pool_cost: params.min_pool_cost,
                protocol_version: Some(u5c::ProtocolVersion {
                    major: params.protocol_version.0 as u32,
                    minor: params.protocol_version.1 as u32,
                }),
                ..Default::default()
            },
            MultiEraProtocolParameters::Babbage(params) => u5c::PParams {
                max_tx_size: params.max_transaction_size.into(),
                max_block_body_size: params.max_block_body_size.into(),
                max_block_header_size: params.max_block_header_size.into(),
                min_fee_coefficient: params.minfee_a.into(),
                min_fee_constant: params.minfee_b.into(),
                coins_per_utxo_byte: params.ada_per_utxo_byte,
                stake_key_deposit: params.key_deposit,
                pool_deposit: params.pool_deposit,
                desired_number_of_pools: params.desired_number_of_stake_pools.into(),
                pool_influence: Some(u5c::RationalNumber {
                    numerator: params.pool_pledge_influence.numerator as i32,
                    denominator: params.pool_pledge_influence.denominator as u32,
                }),
                monetary_expansion: u5c::RationalNumber {
                    numerator: params.expansion_rate.numerator as i32,
                    denominator: params.expansion_rate.denominator as u32,
                }
                .into(),
                treasury_expansion: Some(u5c::RationalNumber {
                    numerator: params.treasury_growth_rate.numerator as i32,
                    denominator: params.treasury_growth_rate.denominator as u32,
                }),
                min_pool_cost: params.min_pool_cost,
                protocol_version: u5c::ProtocolVersion {
                    major: params.protocol_version.0 as u32,
                    minor: params.protocol_version.1 as u32,
                }
                .into(),
                max_value_size: params.max_value_size.into(),
                collateral_percentage: params.collateral_percentage.into(),
                max_collateral_inputs: params.max_collateral_inputs.into(),
                prices: Some(u5c::ExPrices {
                    steps: Some(u5c::RationalNumber {
                        numerator: params.execution_costs.step_price.numerator as i32,
                        denominator: params.execution_costs.step_price.denominator as u32,
                    }),
                    memory: Some(u5c::RationalNumber {
                        numerator: params.execution_costs.mem_price.numerator as i32,
                        denominator: params.execution_costs.mem_price.denominator as u32,
                    }),
                }),
                max_execution_units_per_transaction: Some(u5c::ExUnits {
                    memory: params.max_tx_ex_units.mem,
                    steps: params.max_tx_ex_units.steps,
                }),
                max_execution_units_per_block: Some(u5c::ExUnits {
                    memory: params.max_block_ex_units.mem,
                    steps: params.max_block_ex_units.steps,
                }),
                cost_models: u5c::CostModels {
                    plutus_v1: params
                        .cost_models_for_script_languages
                        .plutus_v1
                        .map(|values| u5c::CostModel { values }),
                    plutus_v2: params
                        .cost_models_for_script_languages
                        .plutus_v2
                        .map(|values| u5c::CostModel { values }),
                    ..Default::default()
                }
                .into(),
                ..Default::default()
            },
            MultiEraProtocolParameters::Byron(params) => u5c::PParams {
                max_tx_size: params.max_tx_size,
                max_block_body_size: params.max_block_size - params.max_header_size,
                max_block_header_size: params.max_header_size,
                ..Default::default()
            },
            MultiEraProtocolParameters::Conway(params) => u5c::PParams {
                max_tx_size: params.max_transaction_size.into(),
                max_block_body_size: params.max_block_body_size.into(),
                max_block_header_size: params.max_block_header_size.into(),
                min_fee_coefficient: params.minfee_a.into(),
                min_fee_constant: params.minfee_b.into(),
                coins_per_utxo_byte: params.ada_per_utxo_byte,
                stake_key_deposit: params.key_deposit,
                pool_deposit: params.pool_deposit,
                desired_number_of_pools: params.desired_number_of_stake_pools.into(),
                pool_influence: Some(u5c::RationalNumber {
                    numerator: params.pool_pledge_influence.numerator as i32,
                    denominator: params.pool_pledge_influence.denominator as u32,
                }),
                monetary_expansion: u5c::RationalNumber {
                    numerator: params.expansion_rate.numerator as i32,
                    denominator: params.expansion_rate.denominator as u32,
                }
                .into(),
                treasury_expansion: Some(u5c::RationalNumber {
                    numerator: params.treasury_growth_rate.numerator as i32,
                    denominator: params.treasury_growth_rate.denominator as u32,
                }),
                min_pool_cost: params.min_pool_cost,
                protocol_version: u5c::ProtocolVersion {
                    major: params.protocol_version.0 as u32,
                    minor: params.protocol_version.1 as u32,
                }
                .into(),
                max_value_size: params.max_value_size.into(),
                collateral_percentage: params.collateral_percentage.into(),
                max_collateral_inputs: params.max_collateral_inputs.into(),
                prices: Some(u5c::ExPrices {
                    steps: Some(u5c::RationalNumber {
                        numerator: params.execution_costs.step_price.numerator as i32,
                        denominator: params.execution_costs.step_price.denominator as u32,
                    }),
                    memory: Some(u5c::RationalNumber {
                        numerator: params.execution_costs.mem_price.numerator as i32,
                        denominator: params.execution_costs.mem_price.denominator as u32,
                    }),
                }),
                max_execution_units_per_transaction: Some(u5c::ExUnits {
                    memory: params.max_tx_ex_units.mem,
                    steps: params.max_tx_ex_units.steps,
                }),
                max_execution_units_per_block: Some(u5c::ExUnits {
                    memory: params.max_block_ex_units.mem,
                    steps: params.max_block_ex_units.steps,
                }),
                min_fee_script_ref_cost_per_byte: Some(u5c::RationalNumber {
                    numerator: params.minfee_refscript_cost_per_byte.numerator as i32,
                    denominator: params.minfee_refscript_cost_per_byte.denominator as u32,
                }),
                pool_voting_thresholds: Some(u5c::VotingThresholds{
                    thresholds: vec![
                        u5c::RationalNumber {
                            numerator: params.pool_voting_thresholds.motion_no_confidence.numerator as i32,
                            denominator: params.pool_voting_thresholds.motion_no_confidence.denominator as u32,
                        },
                        u5c::RationalNumber {
                            numerator: params.pool_voting_thresholds.committee_normal.numerator as i32,
                            denominator: params.pool_voting_thresholds.committee_normal.denominator as u32,
                        },
                        u5c::RationalNumber {
                            numerator: params.pool_voting_thresholds.committee_no_confidence.numerator as i32,
                            denominator: params.pool_voting_thresholds.committee_no_confidence.denominator as u32,
                        },
                        u5c::RationalNumber {
                            numerator: params.pool_voting_thresholds.hard_fork_initiation.numerator as i32,
                            denominator: params.pool_voting_thresholds.hard_fork_initiation.denominator as u32,
                        },
                        u5c::RationalNumber {
                            numerator: params.pool_voting_thresholds.security_voting_threshold.numerator as i32,
                            denominator: params.pool_voting_thresholds.security_voting_threshold.denominator as u32,
                        },
                    ]
                }),
                drep_voting_thresholds: Some(u5c::VotingThresholds{
                    thresholds: vec![
                        u5c::RationalNumber {
                            numerator: params.drep_voting_thresholds.motion_no_confidence.numerator as i32,
                            denominator: params.drep_voting_thresholds.motion_no_confidence.denominator as u32,
                        },
                        u5c::RationalNumber {
                            numerator: params.drep_voting_thresholds.committee_normal.numerator as i32,
                            denominator: params.drep_voting_thresholds.committee_normal.denominator as u32,
                        },
                        u5c::RationalNumber {
                            numerator: params.drep_voting_thresholds.committee_no_confidence.numerator as i32,
                            denominator: params.drep_voting_thresholds.committee_no_confidence.denominator as u32,
                        },
                        u5c::RationalNumber {
                            numerator: params.drep_voting_thresholds.update_constitution.numerator as i32,
                            denominator: params.drep_voting_thresholds.update_constitution.denominator as u32,
                        },
                        u5c::RationalNumber {
                            numerator: params.drep_voting_thresholds.hard_fork_initiation.numerator as i32,
                            denominator: params.drep_voting_thresholds.hard_fork_initiation.denominator as u32,
                        },
                        u5c::RationalNumber {
                            numerator: params.drep_voting_thresholds.pp_network_group.numerator as i32,
                            denominator: params.drep_voting_thresholds.pp_network_group.denominator as u32,
                        },
                        u5c::RationalNumber {
                            numerator: params.drep_voting_thresholds.pp_economic_group.numerator as i32,
                            denominator: params.drep_voting_thresholds.pp_economic_group.denominator as u32,
                        },
                        u5c::RationalNumber {
                            numerator: params.drep_voting_thresholds.pp_technical_group.numerator as i32,
                            denominator: params.drep_voting_thresholds.pp_technical_group.denominator as u32,
                        },
                        u5c::RationalNumber {
                            numerator: params.drep_voting_thresholds.pp_governance_group.numerator as i32,
                            denominator: params.drep_voting_thresholds.pp_governance_group.denominator as u32,
                        },
                        u5c::RationalNumber {
                            numerator: params.drep_voting_thresholds.treasury_withdrawal.numerator as i32,
                            denominator: params.drep_voting_thresholds.treasury_withdrawal.denominator as u32,
                        },
                    ]
                }),
                min_committee_size: params.min_committee_size as u32,
                committee_term_limit: params.committee_term_limit.into(),
                governance_action_validity_period: params.governance_action_validity_period.into(),
                governance_action_deposit: params.governance_action_deposit.into(),
                drep_deposit: params.drep_deposit.into(),
                drep_inactivity_period: params.drep_inactivity_period.into(),
                cost_models: u5c::CostModels {
                    plutus_v1: params
                        .cost_models_for_script_languages
                        .plutus_v1
                        .map(|values| u5c::CostModel { values }),
                    plutus_v2: params
                        .cost_models_for_script_languages
                        .plutus_v2
                        .map(|values| u5c::CostModel { values }),
                    plutus_v3: params
                        .cost_models_for_script_languages
                        .plutus_v3
                        .map(|values| u5c::CostModel { values }),
                }
                .into(),
                ..Default::default()
            },
            _ => unimplemented!(),
        }
    }
}
