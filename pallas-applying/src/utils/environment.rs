//! Types used for representing the environment required for validation in each
//! era.
use pallas_traverse::MultiEraProtocolParameters;

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
