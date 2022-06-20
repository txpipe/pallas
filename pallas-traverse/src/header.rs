use pallas_codec::minicbor;
use pallas_crypto::hash::{Hash, Hasher};
use pallas_primitives::ToHash;

use crate::{Error, MultiEraHeader};
use crate::Era::Byron;

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
            5 => {
                let header = minicbor::decode(cbor).map_err(Error::invalid_cbor)?;
                Ok(MultiEraHeader::Babbage(header))
            }
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
            MultiEraHeader::Babbage(x) => x.header_body.slot,
            MultiEraHeader::Byron(x) => x.consensus_data.0.to_abs_slot(),
        }
    }

    pub fn hash(&self) -> Hash<32> {
        match self {
            MultiEraHeader::EpochBoundary(x) => x.to_hash(),
            MultiEraHeader::AlonzoCompatible(x) => x.to_hash(),
            MultiEraHeader::Babbage(x) => x.to_hash(),
            MultiEraHeader::Byron(x) => x.to_hash(),
        }
    }

    pub fn leader_vrf_output(&self) -> Result<Vec<u8>, Error> {
        match self {
            MultiEraHeader::EpochBoundary(_) => Err(Error::InvalidEra(Byron)),
            MultiEraHeader::AlonzoCompatible(x) => {
                Ok(x.header_body.leader_vrf.0.to_vec())
            }
            MultiEraHeader::Babbage(x) => {
                let mut leader_tagged_vrf: Vec<u8> = vec![0x4C_u8]; /* "L" */
                leader_tagged_vrf.extend(&*x.header_body.vrf_result.0);
                Ok(Hasher::<256>::hash(&leader_tagged_vrf).to_vec())
            }
            MultiEraHeader::Byron(_) => Err(Error::InvalidEra(Byron)),
        }
    }

    pub fn nonce_vrf_output(&self) -> Result<Vec<u8>, Error> {
        match self {
            MultiEraHeader::EpochBoundary(_) => Err(Error::InvalidEra(Byron)),
            MultiEraHeader::AlonzoCompatible(x) => {
                Ok(x.header_body.nonce_vrf.0.to_vec())
            }
            MultiEraHeader::Babbage(x) => {
                let mut nonce_tagged_vrf: Vec<u8> = vec![0x4E_u8]; /* "N" */
                nonce_tagged_vrf.extend(&*x.header_body.vrf_result.0);
                Ok(Hasher::<256>::hash(&nonce_tagged_vrf).to_vec())
            }
            MultiEraHeader::Byron(_) => Err(Error::InvalidEra(Byron)),
        }
    }
}