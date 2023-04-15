use std::time::Duration;

use pallas_codec::{
    minicbor::{self, Decode, Encode},
    utils::Bytes,
};
use pallas_primitives::babbage::{
    AddrKeyhash, AuxiliaryData, PolicyId, TransactionBody, TransactionInput, TransactionOutput,
    Value, WitnessSet,
};

// TODO: Replace with some slot conversion lookup
const SLOT_CONVERSION: u64 = 0;

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

struct TransactionBuilderInput {
    input: TransactionInput,
    resolved: TransactionOutput,
}

struct TransactionBuilderOutput {
    output: TransactionOutput,
}

pub struct TxBuilder {
    inputs: Vec<TransactionBuilderInput>,
    outputs: Vec<TransactionBuilderOutput>,
    mint: Option<Value>,
    required_signers: Vec<AddrKeyhash>,
    valid_after: Option<Duration>,
    valid_until: Option<Duration>,
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
            valid_after: None,
            valid_until: None,
        }
    }

    /// Add an input
    pub fn input(mut self, input: TransactionInput, resolved: TransactionOutput) -> Self {
        self.inputs
            .push(TransactionBuilderInput { input, resolved });

        self
    }

    /// Add an output
    pub fn output(mut self, output: TransactionOutput) -> Self {
        self.outputs.push(TransactionBuilderOutput { output });

        self
    }

    /// Require a signer
    pub fn signer(mut self, new_signer: AddrKeyhash) -> Self {
        self.required_signers.push(new_signer);

        self
    }

    /// Assets to mint or burn
    pub fn mint_assets(mut self, policy: PolicyId, assets: Vec<(Bytes, u64)>) -> Self {
        let mint = vec![(policy, assets.into())];

        self.mint = Some(Value::Multiasset(0, mint.into()));

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

    pub fn build(self) -> Transaction {
        Transaction {
            body: TransactionBody {
                inputs: self.inputs.into_iter().map(|i| i.input).collect(),
                outputs: self.outputs.into_iter().map(|i| i.output).collect(),
                fee: 0,
                ttl: self.valid_until.map(|i| i.as_secs() - SLOT_CONVERSION),

                certificates: None,
                withdrawals: None,
                update: None,
                auxiliary_data_hash: None,
                validity_interval_start: self.valid_after.map(|i| i.as_secs() - SLOT_CONVERSION),
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

    use std::time::Duration;

    use pallas_primitives::{
        babbage::{PseudoPostAlonzoTransactionOutput, TransactionInput, TransactionOutput, Value},
        Fragment,
    };

    use crate::Transaction;

    #[test]
    fn build_basic() {
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
            .build();

        let bytes = tx.encode_fragment().expect("encoding failed");

        assert_eq!(hex::encode(bytes), "83a300818258200000000000000000000000000000000000000000000000000000000000000000000181a20040011a000f42400200a0f5")
    }

    #[test]
    fn build_ttl_valid_after() {
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

        let valid_after = 1618400000; // Unix time for April 14, 2021, 12:00:00 AM UTC
        let valid_until = 1618430000;

        let tx = Transaction::builder()
            .input(input, resolved)
            .output(output)
            .valid_after(Duration::from_secs(valid_after))
            .valid_until(Duration::from_secs(valid_until))
            .build();

        let bytes = tx.encode_fragment().expect("encoding failed");

        assert_eq!(hex::encode(bytes), "83a500818258200000000000000000000000000000000000000000000000000000000000000000000181a20040011a000f42400200031a60774830081a6076d300a0f5")
    }

    #[test]
    fn build_mint() {
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
            .mint_assets(policy, assets)
            .build();

        let bytes = tx.encode_fragment().expect("encoding failed");

        assert_eq!(hex::encode(bytes), "83a500818258200000000000000000000000000000000000000000000000000000000000000000000181a20040011a000f42400200031a60774830081a6076d300a0f5")
    }
}
