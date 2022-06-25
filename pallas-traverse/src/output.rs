use std::{
    borrow::Cow,
    ops::{Add, Deref},
};

use pallas_addresses::{Address, Error as AddressError};
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

    pub fn address(&self) -> Result<Address, AddressError> {
        match self {
            MultiEraOutput::AlonzoCompatible(x) => Address::from_bytes(&x.address),
            MultiEraOutput::Babbage(x) => match x.deref().deref() {
                babbage::TransactionOutput::Legacy(x) => Address::from_bytes(&x.address),
                babbage::TransactionOutput::PostAlonzo(x) => Address::from_bytes(&x.address),
            },
            MultiEraOutput::Byron(x) => {
                // TODO: we need to move the byron address struct out of primitives and into the
                // addresses crate. Primitive crate should only handle bytes, without decoding.
                // Decoding should happen at this step when use asks for a decoded address via
                // traverse.
                todo!()
            }
        }
    }

    pub fn ada_amount(&self) -> u64 {
        match self {
            MultiEraOutput::Byron(x) => x.amount,
            MultiEraOutput::Babbage(x) => match x.deref().deref() {
                babbage::TransactionOutput::Legacy(x) => match x.amount {
                    babbage::Value::Coin(c) => u64::from(c),
                    babbage::Value::Multiasset(c, _) => u64::from(c),
                },
                babbage::TransactionOutput::PostAlonzo(x) => match x.value {
                    babbage::Value::Coin(c) => u64::from(c),
                    babbage::Value::Multiasset(c, _) => u64::from(c),
                },
            },
            MultiEraOutput::AlonzoCompatible(x) => match x.amount {
                alonzo::Value::Coin(c) => u64::from(c),
                alonzo::Value::Multiasset(c, _) => u64::from(c),
            },
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
