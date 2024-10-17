//! Types used for representing the environment required for validation in each
//! era.
use pallas_primitives::{
    alonzo::{
        Coin, CostMdls, ExUnitPrices, ExUnits, Nonce, ProtocolVersion, RationalNumber, UnitInterval,
    },
    babbage::CostMdls as BabbageCostMdls,
    conway::{CostMdls as ConwayCostMdls, Epoch},
};

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum MultiEraProtocolParameters {
    Byron(ByronProtParams),
    Shelley(ShelleyProtParams),
    Alonzo(AlonzoProtParams),
    Babbage(BabbageProtParams),
    Conway(ConwayProtParams),
}

impl MultiEraProtocolParameters {
    pub fn protocol_version(&self) -> usize {
        match self {
            MultiEraProtocolParameters::Byron(ByronProtParams {
                block_version: (x, ..),
                ..
            }) => *x as usize,
            MultiEraProtocolParameters::Shelley(ShelleyProtParams {
                protocol_version: (x, ..),
                ..
            }) => *x as usize,
            MultiEraProtocolParameters::Alonzo(AlonzoProtParams {
                protocol_version: (x, ..),
                ..
            }) => *x as usize,
            MultiEraProtocolParameters::Babbage(BabbageProtParams {
                protocol_version: (x, ..),
                ..
            }) => *x as usize,
            MultiEraProtocolParameters::Conway(ConwayProtParams {
                protocol_version: (x, ..),
                ..
            }) => *x as usize,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ByronProtParams {
    pub block_version: (u16, u16, u8),
    pub script_version: u16,
    pub slot_duration: u64,
    pub max_block_size: u64,
    pub max_header_size: u64,
    pub max_tx_size: u64,
    pub max_proposal_size: u64,
    pub mpc_thd: u64,
    pub heavy_del_thd: u64,
    pub update_vote_thd: u64,
    pub update_proposal_thd: u64,
    pub update_implicit: u64,
    pub soft_fork_rule: (u64, u64, u64),
    pub summand: u64,
    pub multiplier: u64,
    pub unlock_stake_epoch: u64,
}

#[derive(Debug, Clone)]
pub struct ShelleyProtParams {
    pub minfee_a: u32,
    pub minfee_b: u32,
    pub max_block_body_size: u32,
    pub max_transaction_size: u32,
    pub max_block_header_size: u32,
    pub key_deposit: Coin,
    pub pool_deposit: Coin,
    pub desired_number_of_stake_pools: u32,
    pub protocol_version: ProtocolVersion,
    pub min_utxo_value: Coin,
    pub min_pool_cost: Coin,
    pub expansion_rate: UnitInterval,
    pub treasury_growth_rate: UnitInterval,
    pub maximum_epoch: Epoch,
    pub pool_pledge_influence: RationalNumber,
    pub decentralization_constant: UnitInterval,
    pub extra_entropy: Nonce,
}

#[derive(Debug, Clone)]
pub struct AlonzoProtParams {
    pub minfee_a: u32,
    pub minfee_b: u32,
    pub max_block_body_size: u32,
    pub max_transaction_size: u32,
    pub max_block_header_size: u32,
    pub key_deposit: Coin,
    pub pool_deposit: Coin,
    pub desired_number_of_stake_pools: u32,
    pub protocol_version: ProtocolVersion,
    pub min_pool_cost: Coin,
    pub ada_per_utxo_byte: Coin,
    pub cost_models_for_script_languages: CostMdls,
    pub execution_costs: ExUnitPrices,
    pub max_tx_ex_units: ExUnits,
    pub max_block_ex_units: ExUnits,
    pub max_value_size: u32,
    pub collateral_percentage: u32,
    pub max_collateral_inputs: u32,
    pub expansion_rate: UnitInterval,
    pub treasury_growth_rate: UnitInterval,
    pub maximum_epoch: Epoch,
    pub pool_pledge_influence: RationalNumber,
    pub decentralization_constant: UnitInterval,
    pub extra_entropy: Nonce,
}

#[derive(Debug, Clone)]
pub struct BabbageProtParams {
    pub minfee_a: u32,
    pub minfee_b: u32,
    pub max_block_body_size: u32,
    pub max_transaction_size: u32,
    pub max_block_header_size: u32,
    pub key_deposit: Coin,
    pub pool_deposit: Coin,
    pub desired_number_of_stake_pools: u32,
    pub protocol_version: ProtocolVersion,
    pub min_pool_cost: Coin,
    pub ada_per_utxo_byte: Coin,
    pub cost_models_for_script_languages: BabbageCostMdls,
    pub execution_costs: ExUnitPrices,
    pub max_tx_ex_units: ExUnits,
    pub max_block_ex_units: ExUnits,
    pub max_value_size: u32,
    pub collateral_percentage: u32,
    pub max_collateral_inputs: u32,
    pub expansion_rate: UnitInterval,
    pub treasury_growth_rate: UnitInterval,
    pub maximum_epoch: Epoch,
    pub pool_pledge_influence: RationalNumber,
    pub decentralization_constant: UnitInterval,
    pub extra_entropy: Nonce,
}

#[derive(Debug, Clone)]
pub struct ConwayProtParams {
    pub minfee_a: u32,
    pub minfee_b: u32,
    pub max_block_body_size: u32,
    pub max_transaction_size: u32,
    pub max_block_header_size: u32,
    pub key_deposit: Coin,
    pub pool_deposit: Coin,
    pub desired_number_of_stake_pools: u32,
    pub protocol_version: ProtocolVersion,
    pub min_pool_cost: Coin,
    pub ada_per_utxo_byte: Coin,
    pub cost_models_for_script_languages: ConwayCostMdls,
    pub execution_costs: ExUnitPrices,
    pub max_tx_ex_units: ExUnits,
    pub max_block_ex_units: ExUnits,
    pub max_value_size: u32,
    pub collateral_percentage: u32,
    pub max_collateral_inputs: u32,
    pub expansion_rate: UnitInterval,
    pub treasury_growth_rate: UnitInterval,
    pub maximum_epoch: Epoch,
    pub pool_pledge_influence: RationalNumber,
    pub pool_voting_thresholds: pallas_primitives::conway::PoolVotingThresholds,
    pub drep_voting_thresholds: pallas_primitives::conway::DRepVotingThresholds,
    pub min_committee_size: u64,
    pub committee_term_limit: Epoch,
    pub governance_action_validity_period: Epoch,
    pub governance_action_deposit: Coin,
    pub drep_deposit: Coin,
    pub drep_inactivity_period: Epoch,
    pub minfee_refscript_cost_per_byte: UnitInterval,
}

#[derive(Default, Debug)]
pub struct AccountState {
    pub treasury: Coin,
    pub reserves: Coin,
}

#[derive(Debug)]
pub struct Environment {
    pub prot_params: MultiEraProtocolParameters,
    pub prot_magic: u32,
    pub block_slot: u64,
    pub network_id: u8,
    pub acnt: Option<AccountState>,
}

impl Environment {
    pub fn prot_params(&self) -> &MultiEraProtocolParameters {
        &self.prot_params
    }

    pub fn prot_magic(&self) -> &u32 {
        &self.prot_magic
    }

    pub fn block_slot(&self) -> &u64 {
        &self.block_slot
    }

    pub fn network_id(&self) -> &u8 {
        &self.network_id
    }

    pub fn acnt(&self) -> &Option<AccountState> {
        &self.acnt
    }
}
