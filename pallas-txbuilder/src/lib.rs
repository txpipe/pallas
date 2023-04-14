use pallas_codec::minicbor::{self, Decode, Encode};
use pallas_primitives::babbage::{
    AddrKeyhash, AuxiliaryData, TransactionBody, TransactionInput, TransactionOutput, Value,
    WitnessSet,
};

#[derive(Encode, Decode, Clone)]
pub struct Transaction {
    #[n(0)]
    body: TransactionBody,
    #[n(1)]
    witness_set: WitnessSet,
    #[n(2)]
    is_valid: bool,
    #[n(3)]
    auxiliary_data: Option<AuxiliaryData>,
}

impl Transaction {
    pub fn builder() -> TxBuilder {
        TxBuilder::new()
    }
}

pub struct TxBuilder {
    inputs: Vec<(TransactionInput, TransactionOutput)>,
    outputs: Vec<TransactionOutput>,
    mint: Option<Value>,
    required_signers: Vec<AddrKeyhash>,
}

impl Default for TxBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TxBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        TxBuilder {
            inputs: vec![],
            outputs: vec![],
            mint: None,
            required_signers: vec![],
        }
    }

    /// Add an input
    pub fn input(mut self, input: TransactionInput, resolved: TransactionOutput) -> Self {
        self.inputs.push((input, resolved));

        self
    }

    /// Add an output
    pub fn output(mut self, output: TransactionOutput) -> Self {
        self.outputs.push(output);

        self
    }

    /// Require a signer
    pub fn signer(mut self, new_signer: AddrKeyhash) -> Self {
        self.required_signers.push(new_signer);

        self
    }

    /// Assets to mint or burn
    pub fn mint_assets(self) -> Self {
        todo!()
    }

    pub fn finalize(self) -> Transaction {
        Transaction {
            body: TransactionBody {
                inputs: self.inputs.into_iter().map(|i| i.0).collect(),
                outputs: self.outputs,
                fee: 0,
                ttl: None,
                certificates: None,
                withdrawals: None,
                update: None,
                auxiliary_data_hash: None,
                validity_interval_start: None,
                mint: None,
                script_data_hash: None,
                collateral: None,
                required_signers: if self.required_signers.is_empty() {
                    None
                } else {
                    Some(self.required_signers)
                },
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
        }
    }
}

#[cfg(test)]
mod test {

    use pallas_primitives::{
        babbage::{PseudoPostAlonzoTransactionOutput, TransactionInput, TransactionOutput, Value},
        Fragment,
    };

    use crate::Transaction;

    #[test]
    fn build() {
        let input = TransactionInput {
            transaction_id: [0; 32].into(),
            index: 0,
        };

        let resolved = TransactionOutput::PostAlonzo(PseudoPostAlonzoTransactionOutput {
            address: vec![].into(),
            value: Value::Coin(1000000),
            datum_option: None,
            script_ref: None,
        });

        let output = TransactionOutput::PostAlonzo(PseudoPostAlonzoTransactionOutput {
            address: vec![].into(),
            value: Value::Coin(1000000),
            datum_option: None,
            script_ref: None,
        });

        let tx = Transaction::builder()
            .input(input, resolved)
            .output(output)
            .finalize();

        let bytes = tx.encode_fragment().expect("encoding failed");

        assert_eq!(hex::encode(bytes), "83a300818258200000000000000000000000000000000000000000000000000000000000000000000181a20040011a000f42400200a0f5")
    }
}
