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
                Ok(ByronAddress::new(&x.address.payload.0, x.address.crc).into())
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

    pub fn encode(&self) -> Result<Vec<u8>, minicbor::encode::Error<std::io::Error>> {
        match self {
            Self::AlonzoCompatible(x) => minicbor::to_vec(x),
            Self::Babbage(x) => minicbor::to_vec(x),
            Self::Byron(x) => minicbor::to_vec(x),
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
                assert_ne!(output.ada_amount(), 0);
                assert!(matches!(output.address(), Ok(_)));
            }
        }
    }
}
