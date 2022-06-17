use pallas_codec::minicbor;
use pallas_crypto::hash::Hash;
use pallas_primitives::ToHash;

use crate::{Error, MultiEraHeader};

impl<'b> MultiEraHeader<'b> {
    pub fn decode(tag: u8, subtag: Option<u8>, cbor: &'b [u8]) -> Result<Self, Error> {
        match tag {
            0 => match subtag {
                Some(0) => {
                    let header = minicbor::decode(cbor).map_err(Error::invalid_cbor)?;
                    Ok(MultiEraHeader::EpochBoundary(header))
                }
                _ => {
                    let header = minicbor::decode(cbor).map_err(Error::invalid_cbor)?;
                    Ok(MultiEraHeader::Byron(header))
                }
            },
            _ => {
                let header = minicbor::decode(cbor).map_err(Error::invalid_cbor)?;
                Ok(MultiEraHeader::AlonzoCompatible(header))
            }
        }
    }

    pub fn slot(&self) -> u64 {
        match self {
            MultiEraHeader::EpochBoundary(x) => x.to_abs_slot(),
            MultiEraHeader::AlonzoCompatible(x) => x.header_body.slot,
            MultiEraHeader::Byron(x) => x.consensus_data.0.to_abs_slot(),
        }
    }

    pub fn hash(&self) -> Hash<32> {
        match self {
            MultiEraHeader::EpochBoundary(x) => x.to_hash(),
            MultiEraHeader::AlonzoCompatible(x) => x.to_hash(),
            MultiEraHeader::Byron(x) => x.to_hash(),
        }
    }
}
