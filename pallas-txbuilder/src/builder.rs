use pallas_codec::utils::Bytes;
use pallas_crypto::hash::Hash;
use pallas_primitives::babbage::{
    AddrKeyhash, Certificate, NetworkId, TransactionBody, TransactionInput, TransactionOutput,
    WitnessSet,
};
use pallas_traverse::ComputeHash;

use crate::{asset::MultiAsset, fee::Fee, transaction, NetworkParams, ValidationError};

pub struct TransactionBuilder {
    inputs: Vec<(TransactionInput, TransactionOutput)>,
    outputs: Vec<TransactionOutput>,

    reference_inputs: Vec<TransactionInput>,
    network_params: NetworkParams,
    mint: Option<MultiAsset<i64>>,
    required_signers: Vec<AddrKeyhash>,
    valid_after: Option<u64>,
    valid_until: Option<u64>,

    certificates: Vec<Certificate>,
}

impl Default for TransactionBuilder {
    fn default() -> Self {
        Self {
            network_params: NetworkParams::mainnet(),

            reference_inputs: Default::default(),
            inputs: Default::default(),
            outputs: Default::default(),
            mint: Default::default(),
            required_signers: Default::default(),
            valid_after: Default::default(),
            valid_until: Default::default(),
            certificates: Default::default(),
        }
    }
}

impl TransactionBuilder {
    pub fn new(network_params: NetworkParams) -> TransactionBuilder {
        TransactionBuilder {
            network_params,
            ..Default::default()
        }
    }

    pub fn input(mut self, input: TransactionInput, resolved: TransactionOutput) -> Self {
        self.inputs.push((input, resolved));
        self
    }

    pub fn reference_input(mut self, input: TransactionInput) -> Self {
        self.reference_inputs.push(input);
        self
    }

    pub fn output(mut self, output: TransactionOutput) -> Self {
        self.outputs.push(output);
        self
    }

    pub fn mint(mut self, assets: MultiAsset<i64>) -> Self {
        self.mint = Some(assets);

        self
    }

    pub fn require_signer(mut self, signer: AddrKeyhash) -> Self {
        self.required_signers.push(signer);
        self
    }

    pub fn valid_after(mut self, timestamp: u64) -> Self {
        self.valid_after = Some(timestamp);
        self
    }

    pub fn valid_until(mut self, timestamp: u64) -> Self {
        self.valid_until = Some(timestamp);
        self
    }

    pub fn certificate(mut self, cert: Certificate) -> Self {
        self.certificates.push(cert);
        self
    }

    pub fn build(self) -> Result<transaction::Transaction, ValidationError> {
        if self.inputs.is_empty() {
            return Err(ValidationError::NoInputs);
        }

        if self.outputs.is_empty() {
            return Err(ValidationError::NoOutputs);
        }

        let inputs = self.inputs.iter().map(|x| x.0.clone()).collect();
        let outputs = self.outputs.clone();

        let mut tx = transaction::Transaction {
            body: TransactionBody {
                inputs,
                outputs,
                ttl: self.convert_timestamp(self.valid_until)?,
                validity_interval_start: self.convert_timestamp(self.valid_after)?,
                fee: 0,
                certificates: opt_if_empty(self.certificates),
                withdrawals: None,
                update: None,
                auxiliary_data_hash: None,
                mint: self.mint.map(|x| x.build()),
                script_data_hash: None,
                collateral: None,
                required_signers: opt_if_empty(self.required_signers),
                network_id: NetworkId::from_u64(self.network_params.network_id()),
                collateral_return: None,
                total_collateral: None,
                reference_inputs: opt_if_empty(self.reference_inputs),
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
        };

        tx.body.fee = Fee::linear().calculate(&tx)?;
        tx.body.auxiliary_data_hash = tx.auxiliary_data.clone().map(hash_to_bytes);

        Ok(tx)
    }

    fn convert_timestamp(&self, timestamp: Option<u64>) -> Result<Option<u64>, ValidationError> {
        match timestamp {
            Some(v) => match self.network_params.timestamp_to_slot(v) {
                Some(v) => Ok(Some(v)),
                None => return Err(ValidationError::InvalidTimestamp),
            },
            _ => Ok(None),
        }
    }
}

#[inline(always)]
fn opt_if_empty<T>(v: Vec<T>) -> Option<Vec<T>> {
    if v.is_empty() {
        None
    } else {
        Some(v)
    }
}

#[inline(always)]
fn hash_to_bytes<const N: usize, T: ComputeHash<N>>(input: T) -> Bytes {
    let b = input.compute_hash().as_ref().to_vec();
    b.into()
}
