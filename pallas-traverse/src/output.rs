use std::{borrow::Cow, ops::Deref};

use pallas_addresses::{Address, ByronAddress, Error as AddressError};
use pallas_codec::minicbor;
use pallas_primitives::{alonzo, babbage, byron};

use crate::{Era, MultiEraOutput};

impl<'b> MultiEraOutput<'b> {
    pub fn from_byron(output: &'b byron::TxOut) -> Self {
        Self::Byron(Box::new(Cow::Borrowed(output)))
    }

    pub fn from_alonzo_compatible(output: &'b alonzo::TransactionOutput) -> Self {
        Self::AlonzoCompatible(Box::new(Cow::Borrowed(output)))
    }

    pub fn from_babbage(output: &'b babbage::MintedTransactionOutput<'b>) -> Self {
        Self::Babbage(Box::new(Cow::Borrowed(output)))
    }

    pub fn datum(&self) -> Option<babbage::MintedDatumOption> {
        match self {
            MultiEraOutput::AlonzoCompatible(x) => {
                x.datum_hash.map(babbage::MintedDatumOption::Hash)
            }
            MultiEraOutput::Babbage(x) => match x.deref().deref() {
                babbage::MintedTransactionOutput::Legacy(x) => {
                    x.datum_hash.map(babbage::MintedDatumOption::Hash)
                }
                babbage::MintedTransactionOutput::PostAlonzo(x) => x.datum_option.clone(),
            },
            _ => None,
        }
    }

    pub fn script_ref(&self) -> Option<&babbage::ScriptRef> {
        match &self {
            MultiEraOutput::Babbage(x) => match x.deref().deref() {
                babbage::MintedTransactionOutput::Legacy(_) => None,
                babbage::MintedTransactionOutput::PostAlonzo(x) => x.script_ref.as_ref(),
            },
            _ => None,
        }
    }

    pub fn address(&self) -> Result<Address, AddressError> {
        match self {
            MultiEraOutput::AlonzoCompatible(x) => Address::from_bytes(&x.address),
            MultiEraOutput::Babbage(x) => match x.deref().deref() {
                babbage::MintedTransactionOutput::Legacy(x) => Address::from_bytes(&x.address),
                babbage::MintedTransactionOutput::PostAlonzo(x) => Address::from_bytes(&x.address),
            },
            MultiEraOutput::Byron(x) => {
                Ok(ByronAddress::new(&x.address.payload.0, x.address.crc).into())
            }
        }
    }

    pub fn as_babbage(&self) -> Option<&babbage::MintedTransactionOutput> {
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

    pub fn encode(&self) -> Vec<u8> {
        // to_vec is infallible
        match self {
            Self::AlonzoCompatible(x) => minicbor::to_vec(x).unwrap(),
            Self::Babbage(x) => minicbor::to_vec(x).unwrap(),
            Self::Byron(x) => minicbor::to_vec(x).unwrap(),
        }
    }

    pub fn decode(era: Era, cbor: &'b [u8]) -> Result<Self, minicbor::decode::Error> {
        match era {
            Era::Byron => {
                let tx = minicbor::decode(cbor)?;
                let tx = Box::new(Cow::Owned(tx));
                Ok(Self::Byron(tx))
            }
            Era::Shelley | Era::Allegra | Era::Mary | Era::Alonzo => {
                let tx = minicbor::decode(cbor)?;
                let tx = Box::new(Cow::Owned(tx));
                Ok(Self::AlonzoCompatible(tx))
            }
            Era::Babbage => {
                let tx = minicbor::decode(cbor)?;
                let tx = Box::new(Cow::Owned(tx));
                Ok(Self::Babbage(tx))

            }
            Era::Conway => {
                let tx = minicbor::decode(cbor)?;
                let tx = Box::new(Cow::Owned(tx));
                Ok(Self::Babbage(tx))
            }
        }
    }
}
