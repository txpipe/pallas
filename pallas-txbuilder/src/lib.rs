use std::{collections::HashMap, time::Duration};

use pallas_codec::utils::Bytes;
use pallas_primitives::babbage::{AddrKeyhash, Certificate, PolicyId, TransactionBody, WitnessSet};

use crate::strategy::*;

pub mod prelude;
pub mod strategy;
pub mod transaction;

/// Converts a duration into a slot for the transaction to use.
fn convert_slot(_duration: Duration) -> u64 {
    todo!()
}

#[inline(always)]
fn opt_if_empty<T>(v: Vec<T>) -> Option<Vec<T>> {
    if v.is_empty() {
        Some(v)
    } else {
        None
    }
}

#[derive(Default)]
pub struct TransactionBuilder<T: Default> {
    strategy: T,

    mint: HashMap<PolicyId, Vec<Vec<(Bytes, u64)>>>,
    required_signers: Vec<AddrKeyhash>,
    valid_after: Option<Duration>,
    valid_until: Option<Duration>,

    certificates: Vec<Certificate>,
}

pub enum ValidationError {
    TransactionUnbalanced,
}

impl<T: Default + Strategy> TransactionBuilder<T> {
    pub fn new() -> TransactionBuilder<Simple> {
        Default::default()
    }

    pub fn new_graph() -> TransactionBuilder<Graph> {
        Default::default()
    }

    pub fn mint(mut self, policy: PolicyId, assets: Vec<(Bytes, u64)>) -> Self {
        self.mint
            .entry(policy)
            .and_modify(|v| v.push(assets.clone()))
            .or_insert(vec![assets]);

        self
    }

    pub fn required_signer(mut self, signer: AddrKeyhash) -> Self {
        self.required_signers.push(signer);
        self
    }

    pub fn valid_after(mut self, duration: Duration) -> Self {
        self.valid_after = Some(duration);
        self
    }

    pub fn valid_until(mut self, duration: Duration) -> Self {
        self.valid_until = Some(duration);
        self
    }

    pub fn certificate(mut self, cert: Certificate) -> Self {
        self.certificates.push(cert);
        self
    }

    pub fn sign(self) -> Self {
        todo!()
    }

    pub fn build(self) -> Result<transaction::Transaction, ValidationError> {
        Ok(transaction::Transaction {
            body: TransactionBody {
                inputs: self.strategy.inputs(),
                outputs: self.strategy.outputs()?,
                fee: 0,
                ttl: self.valid_until.map(convert_slot),
                certificates: opt_if_empty(self.certificates),
                withdrawals: None,
                update: None,
                auxiliary_data_hash: None,
                validity_interval_start: self.valid_after.map(convert_slot),
                mint: None,
                script_data_hash: None,
                collateral: None,
                required_signers: opt_if_empty(self.required_signers),
                network_id: None,
                collateral_return: None,
                total_collateral: None,
                reference_inputs: None,
            },
            witness_set: WitnessSet {
                vkeywitness: None,
                native_script: None,
                bootstrap_witness: None,
                plutus_v1_script: None,
                plutus_data: None,
                redeemer: None,
                plutus_v2_script: None,
            },
            is_valid: true,
            auxiliary_data: None,
        })
    }
}

impl TransactionBuilder<Simple> {
    /// Add an input UTXO into the transaction
    pub fn input(mut self, input: transaction::Input) -> Self {
        self.strategy.inputs.push(input);
        self
    }

    /// Add an output UTXO into the transaction
    pub fn output(mut self, output: transaction::Output) -> Self {
        self.strategy.outputs.push(output);
        self
    }

    /// Defines where change outputs go in the transaction
    pub fn change_address(self, _address: AddrKeyhash) -> Self {
        todo!()
    }
}
