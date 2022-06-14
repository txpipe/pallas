use std::borrow::Cow;

use pallas_primitives::{alonzo, byron};

use crate::MultiEraOutput;

impl<'b> MultiEraOutput<'b> {
    pub fn from_byron(output: &'b byron::TxOut) -> Self {
        Self::Byron(Box::new(Cow::Borrowed(output)))
    }

    pub fn from_alonzo_compatible(output: &'b alonzo::TransactionOutput) -> Self {
        Self::AlonzoCompatible(Box::new(Cow::Borrowed(output)))
    }

    pub fn address(&self, hrp: &str) -> String {
        match self {
            MultiEraOutput::Byron(x) => x.address.to_addr_string().expect("invalid address value"),
            MultiEraOutput::AlonzoCompatible(x) => {
                x.to_bech32_address(hrp).expect("invalid address value")
            }
        }
    }

    pub fn as_alonzo(&self) -> Option<&alonzo::TransactionOutput> {
        match self {
            MultiEraOutput::Byron(_) => None,
            MultiEraOutput::AlonzoCompatible(x) => Some(x),
        }
    }

    pub fn as_byron(&self) -> Option<&byron::TxOut> {
        match self {
            MultiEraOutput::Byron(x) => Some(x),
            MultiEraOutput::AlonzoCompatible(_) => None,
        }
    }
}
