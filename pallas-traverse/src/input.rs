use std::{borrow::Cow, fmt::Display, ops::Deref};

use pallas_codec::utils::CborWrap;
use pallas_crypto::hash::Hash;
use pallas_primitives::{alonzo, byron};

use crate::{MultiEraInput, OutputRef};

impl OutputRef<'_> {
    pub fn tx_id(&self) -> &Hash<32> {
        &self.0
    }

    pub fn tx_index(&self) -> u64 {
        self.1
    }
}

impl Display for OutputRef<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}#{}", self.tx_id(), self.tx_index())
    }
}

impl<'b> MultiEraInput<'b> {
    pub fn from_byron(input: &'b byron::TxIn) -> Self {
        Self::Byron(Box::new(Cow::Borrowed(input)))
    }

    pub fn from_alonzo_compatible(input: &'b alonzo::TransactionInput) -> Self {
        Self::AlonzoCompatible(Box::new(Cow::Borrowed(input)))
    }

    pub fn output_ref(&self) -> Option<OutputRef> {
        match self {
            MultiEraInput::Byron(x) => match x.deref().deref() {
                byron::TxIn::Variant0(CborWrap((tx, idx))) => {
                    Some(OutputRef(Cow::Borrowed(tx), *idx as u64))
                }
                byron::TxIn::Other(_, _) => None,
            },
            MultiEraInput::AlonzoCompatible(x) => {
                Some(OutputRef(Cow::Borrowed(&x.transaction_id), x.index))
            }
        }
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

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn test_expected_values() {
        let blocks = vec![
            include_str!("../../test_data/byron2.block"),
            include_str!("../../test_data/alonzo1.block"),
        ];

        let mut expected = vec![
            "da832fb5ef57df5b91817e9a7448d26e92552afb34f8ee5adb491b24bbe990d5#14",
            "e059de2179400cd7e81ddb6683c0136c9d68119ff3a27a472ad2d98e2f1fbc9c#3",
            "adeb5745e6dba2c05a98f0ad9162b947f1484e998b8b3335f98213e0c67f426e#0",
            "f0fb258a6e741a02ae91b8dc7fe340b9e5b601a6048bf2a0c205f9cc6f51768d#1",
            "c2e4e1f1d8217724b76d979166b16cb0cf5cd6506f70f48c618a085b10460c44#2",
            "aaca2f41f4a17fe464481c69f1220a7bfd93b1a6854f52006094271204e7df7c#0",
            "89185f2daf9ea3bdfdb5d1fef7eced7e890cb89b8821275c0bf0973be08c4ee9#1",
            "bf1f12a83095ac6738ecce5e3e540ad2cff160c46af9137eb6dc0b971f0ac5de#0",
            "df4ebe9ac3ad31a55a06f3e51ca0dbaa947aaf25857ab3a12fe9315cabec11d3#0",
            "087138a5596168650835c8c00f488e167e869bd991ef0683d2dbf3696b0e6650#1",
            "cc9f28625de0b5b9bbe8f61c9332bfda2c987162f85d2e42e437666c27826573#0",
            "d0965859ce9b3025ccbe64f24e3cb30f7400252eb3e235c3604986c2fdd755db#1",
        ];

        for block_str in blocks {
            let cbor = hex::decode(block_str).expect("invalid hex");
            let block = MultiEraBlock::decode(&cbor).expect("invalid cbor");
            for tx in block.txs() {
                for input in tx.inputs() {
                    if let Some(out) = input.output_ref() {
                        let right = expected.remove(0);
                        assert_eq!(out.to_string(), right);
                    }
                }
            }
        }
    }
}
