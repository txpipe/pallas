use minicbor::{Decode, Encode};
use pallas_crypto::key::ed25519::SecretKey;
use pallas_primitives::{
    babbage::{AuxiliaryData, TransactionBody, WitnessSet},
    Fragment,
};

mod input;
mod output;

pub use input::*;
pub use output::*;
use pallas_traverse::ComputeHash;

use crate::ValidationError;

#[derive(Debug, Encode, Decode, Clone)]
pub struct Transaction {
    #[n(0)]
    pub body: TransactionBody,
    #[n(1)]
    pub witness_set: WitnessSet,
    #[n(2)]
    pub is_valid: bool,
    #[n(3)]
    pub auxiliary_data: Option<AuxiliaryData>,
}

impl Transaction {
    // TODO: If we add more params after the transaction, the signatures should change as well, so
    // ideally we should only allow it after the transaction has been finalized. OTOH, it feels
    // really weird to add that directly to the transaction itself, and not to the builder. I'm
    // keeping this here for now, but need to find an ergonomic way to make it happen.
    pub fn sign(mut self, secret_key: SecretKey) -> Self {
        let pubkey: Vec<u8> = Vec::from(secret_key.public_key().as_ref());

        let hash = self.body.compute_hash();
        let signature = Vec::from(secret_key.sign(hash).as_ref());

        let mut vkey_witnesses = self.witness_set.vkeywitness.unwrap_or(vec![]);

        vkey_witnesses.push(pallas_primitives::babbage::VKeyWitness {
            vkey: pubkey.into(),
            signature: signature.into(),
        });

        self.witness_set.vkeywitness = Some(vkey_witnesses);

        self
    }

    pub fn hex_encoded(&self) -> Result<String, ValidationError> {
        self.encode_fragment()
            .map(hex::encode)
            .map_err(|_| ValidationError::UnencodableTransaction)
    }
}
