//! Types used for representing the environment required for validation in each
//! era.

#[derive(Debug)]
pub struct Environment {
    pub prot_params: MultiEraProtParams,
    pub prot_magic: u32,
    pub block_slot: u64,
    pub network_id: u8,
}

// TODO: add variants for the other eras.
#[derive(Debug)]
#[non_exhaustive]
pub enum MultiEraProtParams {
    Byron(ByronProtParams),
    Shelley(ShelleyProtParams),
    Alonzo(AlonzoProtParams),
    Babbage(BabbageProtParams),
}

#[derive(Debug, Clone)]
pub struct ByronProtParams {
    pub fee_policy: FeePolicy,
    pub max_tx_size: u64,
}

#[derive(Debug, Clone)]
pub struct ShelleyProtParams {
    pub fee_policy: FeePolicy,
    pub max_tx_size: u64,
    pub min_lovelace: u64,
}

#[derive(Debug, Clone)]
pub struct FeePolicy {
    pub summand: u64,
    pub multiplier: u64,
}

#[derive(Debug, Clone)]
pub struct AlonzoProtParams {
    pub fee_policy: FeePolicy,
    pub max_tx_size: u64,
    pub max_block_ex_mem: u64,
    pub max_block_ex_steps: u64,
    pub max_tx_ex_mem: u32,
    pub max_tx_ex_steps: u64,
    pub max_val_size: u64,
    pub collateral_percent: u64,
    pub max_collateral_inputs: u64,
    pub coins_per_utxo_word: u64,
}

#[derive(Debug, Clone)]
pub struct BabbageProtParams {
    pub plutus_v2_cost_model_starting_slot: u64,
    pub fee_policy: FeePolicy,
    pub max_tx_size: u64,
    pub max_block_ex_mem: u64,
    pub max_block_ex_steps: u64,
    pub max_tx_ex_mem: u32,
    pub max_tx_ex_steps: u64,
    pub max_val_size: u64,
    pub collateral_percent: u64,
    pub max_collateral_inputs: u64,
    pub coins_per_utxo_word: u64,
}

impl Environment {
    pub fn prot_params(&self) -> &MultiEraProtParams {
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
