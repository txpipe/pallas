use pallas_applying::MultiEraProtocolParameters;
use utxorpc_spec::utxorpc::v1alpha::cardano as u5c;

use crate::{LedgerContext, Mapper};

fn rational_number_to_u5c(value: pallas_primitives::RationalNumber) -> u5c::RationalNumber {
    u5c::RationalNumber {
        numerator: value.numerator as i32,
        denominator: value.denominator as u32,
    }
}

fn execution_prices_to_u5c(value: pallas_primitives::ExUnitPrices) -> u5c::ExPrices {
    u5c::ExPrices {
        steps: Some(rational_number_to_u5c(value.step_price)),
        memory: Some(rational_number_to_u5c(value.mem_price)),
    }
}

fn execution_units_to_u5c(value: pallas_primitives::ExUnits) -> u5c::ExUnits {
    u5c::ExUnits {
        memory: value.mem,
        steps: value.steps,
    }
}

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
                pool_influence: Some(rational_number_to_u5c(params.pool_pledge_influence)),
                monetary_expansion: Some(rational_number_to_u5c(params.expansion_rate)),
                treasury_expansion: Some(rational_number_to_u5c(params.treasury_growth_rate)),
                min_pool_cost: params.min_pool_cost,
                protocol_version: Some(u5c::ProtocolVersion {
                    major: params.protocol_version.0 as u32,
                    minor: params.protocol_version.1 as u32,
                }),
                max_value_size: params.max_value_size.into(),
                collateral_percentage: params.collateral_percentage.into(),
                max_collateral_inputs: params.max_collateral_inputs.into(),
                prices: Some(execution_prices_to_u5c(params.execution_costs)),
                max_execution_units_per_transaction: Some(execution_units_to_u5c(
                    params.max_tx_ex_units,
                )),
                max_execution_units_per_block: Some(execution_units_to_u5c(
                    params.max_block_ex_units,
                )),
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
                pool_influence: Some(rational_number_to_u5c(params.pool_pledge_influence)),
                monetary_expansion: Some(rational_number_to_u5c(params.expansion_rate)),
                treasury_expansion: Some(rational_number_to_u5c(params.treasury_growth_rate)),
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
                pool_influence: Some(rational_number_to_u5c(params.pool_pledge_influence)),
                monetary_expansion: Some(rational_number_to_u5c(params.expansion_rate)),
                treasury_expansion: Some(rational_number_to_u5c(params.treasury_growth_rate)),
                min_pool_cost: params.min_pool_cost,
                protocol_version: u5c::ProtocolVersion {
                    major: params.protocol_version.0 as u32,
                    minor: params.protocol_version.1 as u32,
                }
                .into(),
                max_value_size: params.max_value_size.into(),
                collateral_percentage: params.collateral_percentage.into(),
                max_collateral_inputs: params.max_collateral_inputs.into(),
                prices: Some(execution_prices_to_u5c(params.execution_costs)),
                max_execution_units_per_transaction: Some(execution_units_to_u5c(
                    params.max_tx_ex_units,
                )),
                max_execution_units_per_block: Some(execution_units_to_u5c(
                    params.max_block_ex_units,
                )),
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
                pool_influence: Some(rational_number_to_u5c(params.pool_pledge_influence)),
                monetary_expansion: Some(rational_number_to_u5c(params.expansion_rate)),
                treasury_expansion: Some(rational_number_to_u5c(params.treasury_growth_rate)),
                min_pool_cost: params.min_pool_cost,
                protocol_version: u5c::ProtocolVersion {
                    major: params.protocol_version.0 as u32,
                    minor: params.protocol_version.1 as u32,
                }
                .into(),
                max_value_size: params.max_value_size.into(),
                collateral_percentage: params.collateral_percentage.into(),
                max_collateral_inputs: params.max_collateral_inputs.into(),
                prices: Some(execution_prices_to_u5c(params.execution_costs)),
                max_execution_units_per_transaction: Some(execution_units_to_u5c(
                    params.max_tx_ex_units,
                )),
                max_execution_units_per_block: Some(execution_units_to_u5c(
                    params.max_block_ex_units,
                )),
                min_fee_script_ref_cost_per_byte: Some(rational_number_to_u5c(
                    params.minfee_refscript_cost_per_byte,
                )),
                pool_voting_thresholds: Some(u5c::VotingThresholds {
                    thresholds: vec![
                        rational_number_to_u5c(params.pool_voting_thresholds.motion_no_confidence),
                        rational_number_to_u5c(params.pool_voting_thresholds.committee_normal),
                        rational_number_to_u5c(
                            params.pool_voting_thresholds.committee_no_confidence,
                        ),
                        rational_number_to_u5c(params.pool_voting_thresholds.hard_fork_initiation),
                        rational_number_to_u5c(
                            params.pool_voting_thresholds.security_voting_threshold,
                        ),
                    ],
                }),
                drep_voting_thresholds: Some(u5c::VotingThresholds {
                    thresholds: vec![
                        rational_number_to_u5c(params.drep_voting_thresholds.motion_no_confidence),
                        rational_number_to_u5c(params.drep_voting_thresholds.committee_normal),
                        rational_number_to_u5c(
                            params.drep_voting_thresholds.committee_no_confidence,
                        ),
                        rational_number_to_u5c(params.drep_voting_thresholds.update_constitution),
                        rational_number_to_u5c(params.drep_voting_thresholds.hard_fork_initiation),
                        rational_number_to_u5c(params.drep_voting_thresholds.pp_network_group),
                        rational_number_to_u5c(params.drep_voting_thresholds.pp_economic_group),
                        rational_number_to_u5c(params.drep_voting_thresholds.pp_technical_group),
                        rational_number_to_u5c(params.drep_voting_thresholds.pp_governance_group),
                        rational_number_to_u5c(params.drep_voting_thresholds.treasury_withdrawal),
                    ],
                }),
                min_committee_size: params.min_committee_size as u32,
                committee_term_limit: params.committee_term_limit,
                governance_action_validity_period: params.governance_action_validity_period,
                governance_action_deposit: params.governance_action_deposit,
                drep_deposit: params.drep_deposit,
                drep_inactivity_period: params.drep_inactivity_period,
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
                    ..Default::default()
                }
                .into(),
                ..Default::default()
            },
            _ => unimplemented!(),
        }
    }
}
