use std::{borrow::Cow, ops::Deref};

use pallas_addresses::{Address, ByronAddress, Error as AddressError};
use pallas_codec::minicbor;
use pallas_primitives::{alonzo, babbage, byron};

use crate::{Era, MultiEraOutput, MultiEraPolicyAssets};

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

    pub fn script_ref(&self) -> Option<babbage::MintedScriptRef> {
        match &self {
            MultiEraOutput::Babbage(x) => match x.deref().deref() {
                babbage::MintedTransactionOutput::Legacy(_) => None,
                babbage::MintedTransactionOutput::PostAlonzo(x) => {
                    x.script_ref.clone().map(|x| x.unwrap())
                }
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
            Era::Babbage | Era::Conway => {
                let tx = minicbor::decode(cbor)?;
                let tx = Box::new(Cow::Owned(tx));
                Ok(Self::Babbage(tx))
            }
        }
    }

    /// The amount of ADA asset expressed in Lovelace unit
    ///
    /// The value returned provides the amount of the ADA in a particular
    /// output. The value is expressed in 'lovelace' (1 ADA = 1,000,000
    /// lovelace).
    pub fn lovelace_amount(&self) -> u64 {
        match self {
            MultiEraOutput::Byron(x) => x.amount,
            MultiEraOutput::Babbage(x) => match x.deref().deref() {
                babbage::MintedTransactionOutput::Legacy(x) => match x.amount {
                    babbage::Value::Coin(c) => c,
                    babbage::Value::Multiasset(c, _) => c,
                },
                babbage::MintedTransactionOutput::PostAlonzo(x) => match x.value {
                    babbage::Value::Coin(c) => c,
                    babbage::Value::Multiasset(c, _) => c,
                },
            },
            MultiEraOutput::AlonzoCompatible(x) => match x.amount {
                alonzo::Value::Coin(c) => c,
                alonzo::Value::Multiasset(c, _) => c,
            },
        }
    }

    /// List of native assets in the output
    ///
    /// Returns a list of Asset structs where each one represent a native asset
    /// present in the output of the tx. ADA assets are not included in this
    /// list.
    pub fn non_ada_assets(&self) -> Vec<MultiEraPolicyAssets> {
        match self {
            MultiEraOutput::Byron(_) => vec![],
            MultiEraOutput::Babbage(x) => match x.deref().deref() {
                babbage::MintedTransactionOutput::Legacy(x) => match &x.amount {
                    babbage::Value::Coin(_) => vec![],
                    babbage::Value::Multiasset(_, x) => x
                        .iter()
                        .map(|(k, v)| MultiEraPolicyAssets::AlonzoCompatibleOutput(k, v))
                        .collect(),
                },
                babbage::MintedTransactionOutput::PostAlonzo(x) => match &x.value {
                    babbage::Value::Coin(_) => vec![],
                    babbage::Value::Multiasset(_, x) => x
                        .iter()
                        .map(|(k, v)| MultiEraPolicyAssets::AlonzoCompatibleOutput(k, v))
                        .collect(),
                },
            },
            MultiEraOutput::AlonzoCompatible(x) => match &x.amount {
                alonzo::Value::Coin(_) => vec![],
                alonzo::Value::Multiasset(_, x) => x
                    .iter()
                    .map(|(k, v)| MultiEraPolicyAssets::AlonzoCompatibleOutput(k, v))
                    .collect(),
            },
        }
    }
}
