use std::{collections::HashMap, time::Duration};

use pallas_codec::utils::Bytes;
use pallas_primitives::babbage::{
    AddrKeyhash, Certificate, PolicyId, TransactionBody, TransactionInput, TransactionOutput,
    Value, WitnessSet,
};

use crate::{strategy::*, transaction, NetworkParams, ValidationError};

#[derive(Default)]
pub struct TransactionBuilder<T: Default> {
    strategy: T,

    network_params: NetworkParams,
    mint: Option<Value>,
    required_signers: Vec<AddrKeyhash>,
    signatures: HashMap<AddrKeyhash, Bytes>,
    valid_after: Option<Duration>,
    valid_until: Option<Duration>,

    certificates: Vec<Certificate>,
}

impl<T: Default + Strategy> TransactionBuilder<T> {
    pub fn new(network_params: NetworkParams) -> TransactionBuilder<T> {
        TransactionBuilder {
            network_params,
            ..Default::default()
        }
    }

    pub fn mint_assets(mut self, policy: PolicyId, assets: Vec<(Bytes, u64)>) -> Self {
        let mint = vec![(policy, assets.into())].into();
        self.mint = Some(Value::Multiasset(0, mint));

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
        let (inputs, outputs) = self.strategy.resolve()?;

        Ok(transaction::Transaction {
            body: TransactionBody {
                inputs,
                outputs,
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

impl TransactionBuilder<Manual> {
    pub fn input(mut self, input: TransactionInput, resolved: TransactionOutput) -> Self {
        self.strategy.input(input, resolved);
        self
    }

    pub fn output(mut self, output: TransactionOutput) -> Self {
        self.strategy.output(output);
        self
    }
}

/// Converts a duration into a slot for the transaction to use.
fn convert_slot(_duration: Duration) -> u64 {
    todo!()
}

#[inline(always)]
fn opt_if_empty<T>(v: Vec<T>) -> Option<Vec<T>> {
    if v.is_empty() {
        None
    } else {
        Some(v)
    }
}
