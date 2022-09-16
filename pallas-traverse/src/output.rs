use std::{borrow::Cow, ops::Deref};

use pallas_addresses::{Address, ByronAddress, Error as AddressError};
use pallas_codec::minicbor;
use pallas_primitives::{
    alonzo,
    babbage::{self, Coin, DatumOption, ScriptRef},
    byron,
};

use crate::{Asset, Era, MultiEraOutput, Subject};

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

    pub fn datum(&self) -> Option<DatumOption> {
        match self {
            MultiEraOutput::AlonzoCompatible(x) => x.datum_hash.map(DatumOption::Hash),
            MultiEraOutput::Babbage(x) => match x.deref().deref() {
                babbage::TransactionOutput::Legacy(x) => x.datum_hash.map(DatumOption::Hash),
                babbage::TransactionOutput::PostAlonzo(x) => x.datum_option.clone(),
            },
            _ => None,
        }
    }

    pub fn script_ref(&self) -> Option<&ScriptRef> {
        match &self {
            MultiEraOutput::Babbage(x) => match x.deref().deref() {
                babbage::TransactionOutput::Legacy(_) => None,
                babbage::TransactionOutput::PostAlonzo(x) => x.script_ref.as_ref(),
            },
            _ => None,
        }
    }

    pub fn address(&self) -> Result<Address, AddressError> {
        match self {
            MultiEraOutput::AlonzoCompatible(x) => Address::from_bytes(&x.address),
            MultiEraOutput::Babbage(x) => match x.deref().deref() {
                babbage::TransactionOutput::Legacy(x) => Address::from_bytes(&x.address),
                babbage::TransactionOutput::PostAlonzo(x) => Address::from_bytes(&x.address),
            },
            MultiEraOutput::Byron(x) => {
                Ok(ByronAddress::new(&x.address.payload.0, x.address.crc).into())
            }
        }
    }

    pub fn ada_amount(&self) -> u64 {
        match self {
            MultiEraOutput::Byron(x) => x.amount,
            MultiEraOutput::Babbage(x) => match x.deref().deref() {
                babbage::TransactionOutput::Legacy(x) => match x.amount {
                    babbage::Value::Coin(c) => c,
                    babbage::Value::Multiasset(c, _) => c,
                },
                babbage::TransactionOutput::PostAlonzo(x) => match x.value {
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

    pub fn assets(&self) -> Vec<Asset> {
        let mut assets = Vec::new();

        match self {
            MultiEraOutput::Byron(x) => {
                push_lovelace(&mut assets, x.amount);
            }
            MultiEraOutput::Babbage(x) => match x.deref().deref() {
                babbage::TransactionOutput::Legacy(x) => match &x.amount {
                    babbage::Value::Coin(c) => {
                        push_lovelace(&mut assets, *c);
                    }
                    babbage::Value::Multiasset(c, multi_asset) => {
                        push_lovelace(&mut assets, *c);

                        push_native_asset(&mut assets, multi_asset);
                    }
                },
                babbage::TransactionOutput::PostAlonzo(x) => match &x.value {
                    babbage::Value::Coin(c) => {
                        push_lovelace(&mut assets, *c);
                    }
                    babbage::Value::Multiasset(c, multi_asset) => {
                        push_lovelace(&mut assets, *c);

                        push_native_asset(&mut assets, multi_asset);
                    }
                },
            },
            MultiEraOutput::AlonzoCompatible(x) => match &x.amount {
                alonzo::Value::Coin(c) => {
                    push_lovelace(&mut assets, *c);
                }
                alonzo::Value::Multiasset(c, multi_asset) => {
                    push_lovelace(&mut assets, *c);

                    push_native_asset(&mut assets, multi_asset);
                }
            },
        };

        assets
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
        }
    }
}

fn push_lovelace(assets: &mut Vec<Asset>, quantity: u64) {
    assets.push(Asset {
        subject: Subject::Lovelace,
        quantity,
    })
}

fn push_native_asset(assets: &mut Vec<Asset>, multi_asset: &alonzo::Multiasset<Coin>) {
    for (policy_id, names) in multi_asset.iter() {
        for (asset_name, quantity) in names.iter() {
            assets.push(Asset {
                subject: Subject::NativeAsset(*policy_id, asset_name.clone()),
                quantity: *quantity,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::MultiEraBlock;

    #[test]
    fn traverse_block_with_varied_outputs() {
        let str = include_str!("../../test_data/alonzo24.block");
        let bytes = hex::decode(str).unwrap();
        let block = MultiEraBlock::decode(&bytes).unwrap();

        for tx in block.txs() {
            for output in tx.outputs() {
                assert_ne!(output.assets()[0].quantity, 0);
                assert_ne!(output.ada_amount(), 0);
                assert!(matches!(output.address(), Ok(_)));
            }
        }
    }
}
