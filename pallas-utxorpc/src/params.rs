use pallas_primitives::conway;
use pallas_validate::utils::MultiEraProtocolParameters;
use utxorpc_spec::utxorpc::v1alpha::cardano as u5c;

use crate::{rational_number_to_u5c, LedgerContext, Mapper};

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
                }
                .into(),
                ..Default::default()
            },
            _ => unimplemented!(),
        }
    }

    pub fn map_conway_pparams_update(&self, x: &conway::ProtocolParamUpdate) -> u5c::PParams {
        u5c::PParams {
            coins_per_utxo_byte: x.ada_per_utxo_byte.unwrap_or_default(),
            max_tx_size: x.max_transaction_size.unwrap_or_default(),
            min_fee_coefficient: x.minfee_a.unwrap_or_default(),
            min_fee_constant: x.minfee_b.unwrap_or_default(),
            max_block_body_size: x.max_block_body_size.unwrap_or_default(),
            max_block_header_size: x.max_block_header_size.unwrap_or_default(),
            stake_key_deposit: x.key_deposit.unwrap_or_default(),
            pool_deposit: x.pool_deposit.unwrap_or_default(),
            pool_retirement_epoch_bound: x.maximum_epoch.unwrap_or_default(),
            desired_number_of_pools: x.desired_number_of_stake_pools.unwrap_or_default(),
            pool_influence: x.pool_pledge_influence.clone().map(rational_number_to_u5c),
            monetary_expansion: x.expansion_rate.clone().map(rational_number_to_u5c),
            treasury_expansion: x.treasury_growth_rate.clone().map(rational_number_to_u5c),
            min_pool_cost: x.min_pool_cost.unwrap_or_default(),
            protocol_version: None,
            max_value_size: x.max_value_size.unwrap_or_default(),
            collateral_percentage: x.collateral_percentage.unwrap_or_default(),
            max_collateral_inputs: x.max_collateral_inputs.unwrap_or_default(),
            cost_models: x
                .cost_models_for_script_languages
                .clone()
                .map(|cm| u5c::CostModels {
                    plutus_v1: cm.plutus_v1.map(|values| u5c::CostModel { values }),
                    plutus_v2: cm.plutus_v2.map(|values| u5c::CostModel { values }),
                    plutus_v3: cm.plutus_v3.map(|values| u5c::CostModel { values }),
                }),
            prices: x.execution_costs.clone().map(|p| u5c::ExPrices {
                memory: Some(rational_number_to_u5c(p.mem_price)),
                steps: Some(rational_number_to_u5c(p.step_price)),
            }),
            max_execution_units_per_transaction: x.max_tx_ex_units.map(|u| u5c::ExUnits {
                memory: u.mem,
                steps: u.steps,
            }),
            max_execution_units_per_block: x.max_block_ex_units.map(|u| u5c::ExUnits {
                memory: u.mem,
                steps: u.steps,
            }),
            min_fee_script_ref_cost_per_byte: x
                .minfee_refscript_cost_per_byte
                .clone()
                .map(rational_number_to_u5c),
            pool_voting_thresholds: x.pool_voting_thresholds.clone().map(|t| {
                u5c::VotingThresholds {
                    thresholds: vec![
                        rational_number_to_u5c(t.motion_no_confidence),
                        rational_number_to_u5c(t.committee_normal),
                        rational_number_to_u5c(t.committee_no_confidence),
                        rational_number_to_u5c(t.hard_fork_initiation),
                        rational_number_to_u5c(t.security_voting_threshold),
                    ],
                }
            }),
            drep_voting_thresholds: x.drep_voting_thresholds.clone().map(|t| {
                u5c::VotingThresholds {
                    thresholds: vec![
                        rational_number_to_u5c(t.motion_no_confidence),
                        rational_number_to_u5c(t.committee_normal),
                        rational_number_to_u5c(t.committee_no_confidence),
                        rational_number_to_u5c(t.update_constitution),
                        rational_number_to_u5c(t.hard_fork_initiation),
                        rational_number_to_u5c(t.pp_network_group),
                        rational_number_to_u5c(t.pp_economic_group),
                        rational_number_to_u5c(t.pp_technical_group),
                        rational_number_to_u5c(t.pp_governance_group),
                        rational_number_to_u5c(t.treasury_withdrawal),
                    ],
                }
            }),
            min_committee_size: x.min_committee_size.unwrap_or_default() as u32,
            committee_term_limit: x.committee_term_limit.unwrap_or_default(),
            governance_action_validity_period: x
                .governance_action_validity_period
                .unwrap_or_default(),
            governance_action_deposit: x.governance_action_deposit.unwrap_or_default(),
            drep_deposit: x.drep_deposit.unwrap_or_default(),
            drep_inactivity_period: x.drep_inactivity_period.unwrap_or_default(),
        }
    }
}
