use indexmap::IndexMap;
use pallas_codec::utils::Bytes;

use pallas_primitives::babbage::{
    AddrKeyhash, Certificate, NetworkId, PlutusData, TransactionBody, TransactionInput,
    TransactionOutput, WitnessSet,
};
use pallas_traverse::ComputeHash;

use crate::{
    asset::MultiAsset,
    fee::Fee,
    transaction::{self, OutputExt},
    NetworkParams, ValidationError,
};

pub struct TransactionBuilder {
    inputs: IndexMap<TransactionInput, TransactionOutput>,
    outputs: Vec<TransactionOutput>,

    reference_inputs: Vec<TransactionInput>,
    collaterals: Vec<TransactionInput>,
    collateral_return: Option<TransactionOutput>,
    network_params: NetworkParams,
    mint: Option<MultiAsset<i64>>,
    required_signers: Vec<AddrKeyhash>,
    valid_after: Option<u64>,
    valid_until: Option<u64>,
    certificates: Vec<Certificate>,
    plutus_data: Vec<PlutusData>,
}

impl TransactionBuilder {
    pub fn new(network_params: NetworkParams) -> TransactionBuilder {
        TransactionBuilder {
            network_params,

            inputs: Default::default(),
            outputs: Default::default(),
            reference_inputs: Default::default(),
            collaterals: Default::default(),
            collateral_return: Default::default(),
            mint: Default::default(),
            required_signers: Default::default(),
            valid_after: Default::default(),
            valid_until: Default::default(),
            certificates: Default::default(),
            plutus_data: Default::default(),
        }
    }

    pub fn input(mut self, input: TransactionInput, resolved: TransactionOutput) -> Self {
        self.inputs.insert(input, resolved);
        self
    }

    pub fn reference_input(mut self, input: TransactionInput) -> Self {
        self.reference_inputs.push(input);
        self
    }

    pub fn collateral(mut self, input: TransactionInput) -> Self {
        self.collaterals.push(input);
        self
    }

    pub fn collateral_return(mut self, output: TransactionOutput) -> Self {
        self.collateral_return = Some(output);
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

    pub fn plutus_data(mut self, data: impl Into<PlutusData>) -> Self {
        self.plutus_data.push(data.into());
        self
    }

    pub fn build(self) -> Result<transaction::Transaction, ValidationError> {
        if self.inputs.is_empty() {
            return Err(ValidationError::NoInputs);
        }

        if self.outputs.is_empty() {
            return Err(ValidationError::NoOutputs);
        }

        if self.collaterals.iter().any(|c| {
            self.inputs
                .get(c)
                .map(|i| i.is_multiasset())
                .unwrap_or(false)
        }) {
            return Err(ValidationError::InvalidCollateralInput);
        }

        if self
            .collateral_return
            .as_ref()
            .map(|i| i.is_multiasset())
            .unwrap_or(false)
        {
            return Err(ValidationError::InvalidCollateralReturn);
        }

        let inputs = self.inputs.iter().map(|(k, _)| k.clone()).collect();
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
                collateral: opt_if_empty(self.collaterals),
                required_signers: opt_if_empty(self.required_signers),
                network_id: NetworkId::from_u64(self.network_params.network_id()),
                collateral_return: self.collateral_return,
                total_collateral: None,
                reference_inputs: opt_if_empty(self.reference_inputs),
            },
            witness_set: WitnessSet {
                vkeywitness: None,
                native_script: None,
                bootstrap_witness: None,
                plutus_v1_script: None,
                plutus_data: opt_if_empty(self.plutus_data),
                redeemer: None,
                plutus_v2_script: None,
            },
            is_valid: true,
            auxiliary_data: None,
        };

        tx.body.auxiliary_data_hash = tx.auxiliary_data.clone().map(hash_to_bytes);

        let mut calculated_fee = 0;

        loop {
            calculated_fee = Fee::linear().calculate(&tx)?;

            if tx.body.fee == calculated_fee {
                break;
            }

            tx.body.fee = calculated_fee;
        }

        tx.body.fee = calculated_fee;

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
