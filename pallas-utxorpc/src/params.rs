use pallas_applying::MultiEraProtocolParameters;
use utxorpc_spec::utxorpc::v1alpha::cardano::{self as u5c, CostModel};

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
                        .map(|(_, data)| CostModel {
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
                }
                .into(),
                ..Default::default()
            },
            _ => unimplemented!(),
        }
    }
}
