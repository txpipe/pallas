use pallas_primitives::babbage::{TransactionInput, TransactionOutput};

use crate::prelude::*;

pub trait Strategy {
    fn resolve(&self) -> Result<(Vec<TransactionInput>, Vec<TransactionOutput>), ValidationError>;
}

#[derive(Default)]
/// Receives low-level inputs and outputs, do not resolve anything differently.
// TODO: document this better
pub struct Manual {
    pub inputs: Vec<(TransactionInput, TransactionOutput)>,
    pub outputs: Vec<TransactionOutput>,
}

impl Strategy for Manual {
    fn resolve(&self) -> Result<(Vec<TransactionInput>, Vec<TransactionOutput>), ValidationError> {
        if self.inputs.is_empty() {
            return Err(ValidationError::NoInputs);
        }

        if self.outputs.is_empty() {
            return Err(ValidationError::NoOutputs);
        }

        Ok((
            self.inputs.iter().map(|x| x.0.clone()).collect(),
            self.outputs.clone(),
        ))
    }
}

impl Manual {
    pub fn input(&mut self, input: TransactionInput, resolved: TransactionOutput) {
        self.inputs.push((input, resolved));
    }

    pub fn output(&mut self, output: TransactionOutput) {
        self.outputs.push(output);
    }
}
