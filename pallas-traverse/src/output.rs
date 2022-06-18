use std::borrow::Cow;

use pallas_primitives::{alonzo, babbage, byron};

use crate::MultiEraOutput;

impl<'b> MultiEraOutput<'b> {
    pub fn from_byron(output: &'b byron::TxOut) -> Self {
        Self::Byron(Box::new(Cow::Borrowed(output)))
    }

    pub fn from_alonzo_compatible(output: &'b alonzo::TransactionOutput) -> Self {
        Self::AlonzoCompatible(Box::new(Cow::Borrowed(output)))
    }

    pub fn from_babbage(output: &'b babbage::TransactionOutput) -> Self {
        Self::Babbage(Box::new(Cow::Borrowed(output)))
    }

    pub fn address(&self, hrp: &str) -> String {
        match self {
            MultiEraOutput::AlonzoCompatible(x) => {
                x.to_bech32_address(hrp).expect("invalid address value")
            }
            MultiEraOutput::Babbage(x) => x.to_bech32_address(hrp).expect("invalid address value"),
            MultiEraOutput::Byron(x) => x.address.to_addr_string().expect("invalid address value"),
        }
    }

    pub fn as_babbage(&self) -> Option<&babbage::TransactionOutput> {
        match self {
            MultiEraOutput::AlonzoCompatible(_) => None,
            MultiEraOutput::Babbage(x) => Some(x),
            MultiEraOutput::Byron(_) => None,
        }
    }

    pub fn as_alonzo(&self) -> Option<&alonzo::TransactionOutput> {
        match self {
            MultiEraOutput::AlonzoCompatible(x) => Some(x),
            MultiEraOutput::Babbage(_) => None,
            MultiEraOutput::Byron(_) => None,
        }
    }

    pub fn as_byron(&self) -> Option<&byron::TxOut> {
        match self {
            MultiEraOutput::AlonzoCompatible(_) => None,
            MultiEraOutput::Babbage(_) => None,
            MultiEraOutput::Byron(x) => Some(x),
        }
    }
}
