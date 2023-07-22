use pallas_primitives::babbage::{TransactionInput, TransactionOutput};

use crate::{transaction, ValidationError};

pub trait Strategy {
    fn resolve(&self) -> Result<(Vec<TransactionInput>, Vec<TransactionOutput>), ValidationError>;
}

#[derive(Default)]
/// The simple strategy automatically calculates the inputs and outputs to a transaction, taking
/// care of balancing the utxos between them.
///
/// TODO: Find a better name, the `simple` strategy is much smarter than the normal one, as it
/// balances (and hopefully unfracks) the outputs.
pub struct Automatic {
    pub inputs: Vec<transaction::Input>,
    pub outputs: Vec<transaction::Output>,
}

impl Strategy for Automatic {
    fn resolve(&self) -> Result<(Vec<TransactionInput>, Vec<TransactionOutput>), ValidationError> {
        todo!()
    }
}

#[derive(Default)]
/// The graph strategy allows control on where/how the utxos are consumed, and what kind of output
/// they generate.
///
/// TODO: Find a better name.
pub struct Manual;

impl Strategy for Manual {
    fn resolve(&self) -> Result<(Vec<TransactionInput>, Vec<TransactionOutput>), ValidationError> {
        todo!()
    }
}
