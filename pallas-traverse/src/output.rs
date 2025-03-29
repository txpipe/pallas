use std::{borrow::Cow, ops::Deref};

use pallas_addresses::{Address, ByronAddress, Error as AddressError};
use pallas_codec::minicbor;
use pallas_primitives::{alonzo, babbage, byron, conway};

use crate::{Era, MultiEraOutput, MultiEraPolicyAssets, MultiEraValue};

impl<'b> MultiEraOutput<'b> {
    pub fn from_byron(output: &'b byron::TxOut) -> Self {
        Self::Byron(Box::new(Cow::Borrowed(output)))
    }

    pub fn from_alonzo_compatible(output: &'b alonzo::TransactionOutput, era: Era) -> Self {
        Self::AlonzoCompatible(Box::new(Cow::Borrowed(output)), era)
    }

    pub fn from_babbage(output: &'b babbage::MintedTransactionOutput<'b>) -> Self {
        Self::Babbage(Box::new(Cow::Borrowed(output)))
    }

    pub fn from_conway(output: &'b conway::MintedTransactionOutput<'b>) -> Self {
        Self::Conway(Box::new(Cow::Borrowed(output)))
    }

    pub fn datum(&self) -> Option<conway::DatumOption> {
        match self {
            MultiEraOutput::AlonzoCompatible(x, _) => {
                x.datum_hash.map(babbage::MintedDatumOption::Hash)
            }
            MultiEraOutput::Babbage(x) => match x.deref().deref() {
                babbage::MintedTransactionOutput::Legacy(x) => {
                    x.datum_hash.map(babbage::MintedDatumOption::Hash)
                }
                babbage::MintedTransactionOutput::PostAlonzo(x) => x
                    .datum_option
                    .clone()
                    .map(|y| y.unwrap()),
            },
            MultiEraOutput::Byron(_) => None,
            MultiEraOutput::Conway(x) => match x.deref().deref() {
                conway::MintedTransactionOutput::Legacy(x) => {
                    x.datum_hash.map(babbage::MintedDatumOption::Hash)
                }
                conway::MintedTransactionOutput::PostAlonzo(x) => x
                    .datum_option
                    .clone()
                    .map(|y| y.unwrap()),
            },
        }
    }

    pub fn script_ref(&self) -> Option<conway::MintedScriptRef> {
        match &self {
            MultiEraOutput::AlonzoCompatible(..) => None,
            MultiEraOutput::Babbage(x) => match x.deref().deref() {
                babbage::MintedTransactionOutput::Legacy(_) => None,
                babbage::MintedTransactionOutput::PostAlonzo(x) => {
                    x.script_ref.clone().map(|x| x.unwrap().into())
                }
            },
            MultiEraOutput::Byron(_) => None,
            MultiEraOutput::Conway(x) => match x.deref().deref() {
                conway::MintedTransactionOutput::Legacy(_) => None,
                conway::MintedTransactionOutput::PostAlonzo(x) => {
                    x.script_ref.clone().map(|x| x.unwrap())
                }
            },
        }
    }

    pub fn address(&self) -> Result<Address, AddressError> {
        match self {
            MultiEraOutput::AlonzoCompatible(x, _) => Address::from_bytes(&x.address),
            MultiEraOutput::Babbage(x) => match x.deref().deref() {
                babbage::MintedTransactionOutput::Legacy(x) => Address::from_bytes(&x.address),
                babbage::MintedTransactionOutput::PostAlonzo(x) => Address::from_bytes(&x.address),
            },
            MultiEraOutput::Byron(x) => {
                Ok(ByronAddress::new(&x.address.payload.0, x.address.crc).into())
            }
            MultiEraOutput::Conway(x) => match x.deref().deref() {
                conway::MintedTransactionOutput::Legacy(x) => Address::from_bytes(&x.address),
                conway::MintedTransactionOutput::PostAlonzo(x) => Address::from_bytes(&x.address),
            },
        }
    }

    pub fn as_alonzo(&self) -> Option<&alonzo::TransactionOutput> {
        match self {
            MultiEraOutput::AlonzoCompatible(x, _) => Some(x),
            MultiEraOutput::Babbage(_) => None,
            MultiEraOutput::Byron(_) => None,
            MultiEraOutput::Conway(_) => None,
        }
    }

    pub fn as_babbage(&self) -> Option<&babbage::MintedTransactionOutput> {
        match self {
            MultiEraOutput::AlonzoCompatible(..) => None,
            MultiEraOutput::Babbage(x) => Some(x),
            MultiEraOutput::Byron(_) => None,
            MultiEraOutput::Conway(_) => None,
        }
    }

    pub fn as_byron(&self) -> Option<&byron::TxOut> {
        match self {
            MultiEraOutput::AlonzoCompatible(..) => None,
            MultiEraOutput::Babbage(_) => None,
            MultiEraOutput::Byron(x) => Some(x),
            MultiEraOutput::Conway(_) => None,
        }
    }

    pub fn as_conway(&self) -> Option<&conway::MintedTransactionOutput> {
        match self {
            MultiEraOutput::AlonzoCompatible(..) => None,
            MultiEraOutput::Babbage(_) => None,
            MultiEraOutput::Byron(_) => None,
            MultiEraOutput::Conway(x) => Some(x),
        }
    }

    pub fn era(&self) -> Era {
        match self {
            MultiEraOutput::AlonzoCompatible(_, x) => *x,
            MultiEraOutput::Babbage(_) => Era::Babbage,
            MultiEraOutput::Conway(_) => Era::Conway,
            MultiEraOutput::Byron(_) => Era::Byron,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        // to_vec is infallible
        match self {
            Self::AlonzoCompatible(x, _) => minicbor::to_vec(x).unwrap(),
            Self::Babbage(x) => minicbor::to_vec(x).unwrap(),
            Self::Byron(x) => minicbor::to_vec(x).unwrap(),
            Self::Conway(x) => minicbor::to_vec(x).unwrap(),
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
                Ok(Self::AlonzoCompatible(tx, era))
            }
            Era::Babbage => {
                let tx = minicbor::decode(cbor)?;
                let tx = Box::new(Cow::Owned(tx));
                Ok(Self::Babbage(tx))
            }
            Era::Conway => {
                let tx = minicbor::decode(cbor)?;
                let tx = Box::new(Cow::Owned(tx));
                Ok(Self::Conway(tx))
            }
        }
    }

    pub fn value(&self) -> MultiEraValue<'_> {
        match self {
            MultiEraOutput::Byron(x) => MultiEraValue::Byron(x.amount),
            MultiEraOutput::AlonzoCompatible(x, _) => {
                MultiEraValue::AlonzoCompatible(Cow::Borrowed(&x.amount))
            }
            MultiEraOutput::Babbage(x) => match x.deref().deref() {
                babbage::MintedTransactionOutput::Legacy(x) => {
                    MultiEraValue::AlonzoCompatible(Cow::Borrowed(&x.amount))
                }
                babbage::MintedTransactionOutput::PostAlonzo(x) => {
                    MultiEraValue::AlonzoCompatible(Cow::Borrowed(&x.value))
                }
            },
            MultiEraOutput::Conway(x) => match x.deref().deref() {
                conway::MintedTransactionOutput::Legacy(x) => {
                    MultiEraValue::AlonzoCompatible(Cow::Borrowed(&x.amount))
                }
                conway::MintedTransactionOutput::PostAlonzo(x) => {
                    MultiEraValue::Conway(Cow::Borrowed(&x.value))
                }
            },
        }
    }

    #[deprecated(note = "Use `value().coin()` instead")]
    pub fn lovelace_amount(&self) -> u64 {
        self.value().coin()
    }

    #[deprecated(note = "Use `value().assets()` instead")]
    pub fn non_ada_assets(&self) -> Vec<MultiEraPolicyAssets> {
        match self {
            MultiEraOutput::AlonzoCompatible(x, _) => match &x.amount {
                alonzo::Value::Coin(_) => vec![],
                alonzo::Value::Multiasset(_, x) => x
                    .iter()
                    .map(|(k, v)| MultiEraPolicyAssets::AlonzoCompatibleOutput(k, v))
                    .collect(),
            },
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
            MultiEraOutput::Byron(_) => vec![],
            MultiEraOutput::Conway(x) => match x.deref().deref() {
                conway::MintedTransactionOutput::Legacy(x) => match &x.amount {
                    babbage::Value::Coin(_) => vec![],
                    babbage::Value::Multiasset(_, x) => x
                        .iter()
                        .map(|(k, v)| MultiEraPolicyAssets::AlonzoCompatibleOutput(k, v))
                        .collect(),
                },
                conway::MintedTransactionOutput::PostAlonzo(x) => match &x.value {
                    conway::Value::Coin(_) => vec![],
                    conway::Value::Multiasset(_, x) => x
                        .iter()
                        .map(|(k, v)| MultiEraPolicyAssets::ConwayOutput(k, v))
                        .collect(),
                },
            },
        }
    }
}
