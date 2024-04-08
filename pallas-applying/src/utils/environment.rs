//! Types used for representing the environment required for validation in each
//! era.
use pallas_codec::minicbor::{self, Decode, Encode};
use pallas_primitives::{
    alonzo::{
        Coin, CostMdls, Epoch, ExUnitPrices, ExUnits, Nonce, ProtocolVersion, RationalNumber,
        UnitInterval,
    },
    babbage::CostMdls as BabbageCostMdls,
};

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum MultiEraProtocolParameters {
    Byron(ByronProtParams),
    Shelley(ShelleyProtParams),
    Alonzo(AlonzoProtParams),
    Babbage(BabbageProtParams),
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct ByronProtParams {
    #[n(0)]
    pub script_version: u16,

    #[n(1)]
    pub slot_duration: u64,

    #[n(2)]
    pub max_block_size: u64,

    #[n(3)]
    pub max_header_size: u64,

    #[n(4)]
    pub max_tx_size: u64,

    #[n(5)]
    pub max_proposal_size: u64,

    #[n(6)]
    pub mpc_thd: u64,

    #[n(7)]
    pub heavy_del_thd: u64,

    #[n(8)]
    pub update_vote_thd: u64,

    #[n(9)]
    pub update_proposal_thd: u64,

    #[n(10)]
    pub update_implicit: u64,

    #[n(11)]
    pub soft_fork_rule: (u64, u64, u64),

    #[n(12)]
    pub summand: u64,

    #[n(13)]
    pub multiplier: u64,

    #[n(14)]
    pub unlock_stake_epoch: u64,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct ShelleyProtParams {
    #[n(0)]
    pub minfee_a: u32,

    #[n(1)]
    pub minfee_b: u32,

    #[n(2)]
    pub max_block_body_size: u32,

    #[n(3)]
    pub max_transaction_size: u32,

    #[n(4)]
    pub max_block_header_size: u32,

    #[n(5)]
    pub key_deposit: Coin,

    #[n(6)]
    pub pool_deposit: Coin,

    #[n(7)]
    pub maximum_epoch: Epoch,

    #[n(8)]
    pub desired_number_of_stake_pools: u32,

    #[n(9)]
    pub pool_pledge_influence: RationalNumber,

    #[n(10)]
    pub expansion_rate: UnitInterval,

    #[n(11)]
    pub treasury_growth_rate: UnitInterval,

    #[n(12)]
    pub decentralization_constant: UnitInterval,

    #[n(13)]
    pub extra_entropy: Nonce,

    #[n(14)]
    pub protocol_version: ProtocolVersion,

    #[n(15)]
    pub min_utxo_value: Coin,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct AlonzoProtParams {
    #[n(0)]
    pub minfee_a: u32,

    #[n(1)]
    pub minfee_b: u32,

    #[n(2)]
    pub max_block_body_size: u32,

    #[n(3)]
    pub max_transaction_size: u32,

    #[n(4)]
    pub max_block_header_size: u32,

    #[n(5)]
    pub key_deposit: Coin,

    #[n(6)]
    pub pool_deposit: Coin,

    #[n(7)]
    pub maximum_epoch: Epoch,

    #[n(8)]
    pub desired_number_of_stake_pools: u32,

    #[n(9)]
    pub pool_pledge_influence: RationalNumber,

    #[n(10)]
    pub expansion_rate: UnitInterval,

    #[n(11)]
    pub treasury_growth_rate: UnitInterval,

    #[n(12)]
    pub decentralization_constant: UnitInterval,

    #[n(13)]
    pub extra_entropy: Nonce,

    #[n(14)]
    pub protocol_version: ProtocolVersion,

    #[n(15)]
    pub min_pool_cost: Coin,

    #[n(16)]
    pub ada_per_utxo_byte: Coin,

    #[n(17)]
    pub cost_models_for_script_languages: CostMdls,

    #[n(18)]
    pub execution_costs: ExUnitPrices,

    #[n(19)]
    pub max_tx_ex_units: ExUnits,

    #[n(20)]
    pub max_block_ex_units: ExUnits,

    #[n(21)]
    pub max_value_size: u32,

    #[n(22)]
    pub collateral_percentage: u32,

    #[n(24)]
    pub max_collateral_inputs: u32,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct BabbageProtParams {
    #[n(0)]
    pub minfee_a: u32,

    #[n(1)]
    pub minfee_b: u32,

    #[n(2)]
    pub max_block_body_size: u32,

    #[n(3)]
    pub max_transaction_size: u32,

    #[n(4)]
    pub max_block_header_size: u32,

    #[n(5)]
    pub key_deposit: Coin,

    #[n(6)]
    pub pool_deposit: Coin,

    #[n(7)]
    pub maximum_epoch: Epoch,

    #[n(8)]
    pub desired_number_of_stake_pools: u32,

    #[n(9)]
    pub pool_pledge_influence: RationalNumber,

    #[n(10)]
    pub expansion_rate: UnitInterval,

    #[n(11)]
    pub treasury_growth_rate: UnitInterval,

    #[n(12)]
    pub decentralization_constant: UnitInterval,

    #[n(13)]
    pub extra_entropy: Nonce,

    #[n(14)]
    pub protocol_version: ProtocolVersion,

    #[n(15)]
    pub min_pool_cost: Coin,

    #[n(16)]
    pub ada_per_utxo_byte: Coin,

    #[n(17)]
    pub cost_models_for_script_languages: BabbageCostMdls,

    #[n(18)]
    pub execution_costs: ExUnitPrices,

    #[n(19)]
    pub max_tx_ex_units: ExUnits,

    #[n(20)]
    pub max_block_ex_units: ExUnits,

    #[n(21)]
    pub max_value_size: u32,

    #[n(22)]
    pub collateral_percentage: u32,

    #[n(24)]
    pub max_collateral_inputs: u32,
}

#[derive(Debug)]
pub struct Environment {
    pub prot_params: MultiEraProtocolParameters,
    pub prot_magic: u32,
    pub block_slot: u64,
    pub network_id: u8,
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
}
