use std::borrow::Cow;

use pallas_primitives::{alonzo, byron};

use crate::MultiEraInput;

impl<'b> MultiEraInput<'b> {
    pub fn from_byron(input: &'b byron::TxIn) -> Self {
        Self::Byron(Box::new(Cow::Borrowed(input)))
    }

    pub fn from_alonzo_compatible(input: &'b alonzo::TransactionInput) -> Self {
        Self::AlonzoCompatible(Box::new(Cow::Borrowed(input)))
    }

    pub fn as_alonzo(&self) -> Option<&alonzo::TransactionInput> {
        match self {
            MultiEraInput::Byron(_) => None,
            MultiEraInput::AlonzoCompatible(x) => Some(x),
        }
    }

    pub fn as_byron(&self) -> Option<&byron::TxIn> {
        match self {
            MultiEraInput::Byron(x) => Some(x),
            MultiEraInput::AlonzoCompatible(_) => None,
        }
    }
}
