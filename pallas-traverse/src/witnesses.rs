use pallas_primitives::{alonzo, babbage};

use crate::MultiEraWitnesses;

impl<'b> MultiEraWitnesses<'b> {
    pub fn as_alonzo(&self) -> Option<&alonzo::TransactionWitnessSet> {
        match self {
            Self::AlonzoCompatible(x) => Some(x),
            _ => None,
        }
    }

    pub fn as_babbage(&self) -> Option<&babbage::TransactionWitnessSet> {
        match self {
            Self::Babbage(x) => Some(x),
            _ => None,
        }
    }

    pub fn cbor(&self) -> &[u8] {
        match self {
            MultiEraWitnesses::AlonzoCompatible(x) => x.raw_cbor(),
            MultiEraWitnesses::Babbage(x) => x.raw_cbor(),
            MultiEraWitnesses::Byron(x) => x.raw_cbor(),
        }
    }
}
