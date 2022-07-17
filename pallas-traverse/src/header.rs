use std::borrow::Cow;
use std::ops::Deref;

use pallas_codec::minicbor;
use pallas_crypto::hash::{Hash, Hasher};
use pallas_primitives::{alonzo, babbage, byron, ToHash};

use crate::Era::Byron;
use crate::{Error, MultiEraHeader};

impl<'b> MultiEraHeader<'b> {
    pub fn decode(tag: u8, subtag: Option<u8>, cbor: &'b [u8]) -> Result<Self, Error> {
        match tag {
            0 => match subtag {
                Some(0) => {
                    let header = minicbor::decode(cbor).map_err(Error::invalid_cbor)?;
                    Ok(MultiEraHeader::EpochBoundary(Cow::Owned(header)))
                }
                _ => {
                    let header = minicbor::decode(cbor).map_err(Error::invalid_cbor)?;
                    Ok(MultiEraHeader::Byron(Cow::Owned(header)))
                }
            },
            5 => {
                let header = minicbor::decode(cbor).map_err(Error::invalid_cbor)?;
                Ok(MultiEraHeader::Babbage(Cow::Owned(header)))
            }
            _ => {
                let header = minicbor::decode(cbor).map_err(Error::invalid_cbor)?;
                Ok(MultiEraHeader::AlonzoCompatible(Cow::Owned(header)))
            }
        }
    }

    pub fn cbor(&self) -> &'b [u8] {
        match self {
            MultiEraHeader::EpochBoundary(x) => x.raw_cbor(),
            MultiEraHeader::AlonzoCompatible(x) => x.raw_cbor(),
            MultiEraHeader::Babbage(x) => x.raw_cbor(),
            MultiEraHeader::Byron(x) => x.raw_cbor(),
        }
    }

    pub fn number(&self) -> u64 {
        match self {
            MultiEraHeader::EpochBoundary(x) => x
                .consensus_data
                .difficulty
                .first()
                .cloned()
                .unwrap_or_default(),
            MultiEraHeader::AlonzoCompatible(x) => x.header_body.block_number,
            MultiEraHeader::Babbage(x) => x.header_body.block_number,
            MultiEraHeader::Byron(x) => x.consensus_data.2.first().cloned().unwrap_or_default(),
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
            MultiEraHeader::AlonzoCompatible(x) => Ok(x.header_body.leader_vrf.0.to_vec()),
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
            MultiEraHeader::AlonzoCompatible(x) => Ok(x.header_body.nonce_vrf.0.to_vec()),
            MultiEraHeader::Babbage(x) => {
                let mut nonce_tagged_vrf: Vec<u8> = vec![0x4E_u8]; /* "N" */
                nonce_tagged_vrf.extend(&*x.header_body.vrf_result.0);
                Ok(Hasher::<256>::hash(&nonce_tagged_vrf).to_vec())
            }
            MultiEraHeader::Byron(_) => Err(Error::InvalidEra(Byron)),
        }
    }

    pub fn as_eb(&self) -> Option<&byron::EbbHead> {
        match self {
            MultiEraHeader::EpochBoundary(x) => Some(x.deref().deref()),
            _ => None,
        }
    }

    pub fn as_byron(&self) -> Option<&byron::BlockHead> {
        match self {
            MultiEraHeader::Byron(x) => Some(x.deref().deref()),
            _ => None,
        }
    }

    pub fn as_alonzo(&self) -> Option<&alonzo::Header> {
        match self {
            MultiEraHeader::AlonzoCompatible(x) => Some(x.deref().deref()),
            _ => None,
        }
    }

    pub fn as_babbage(&self) -> Option<&babbage::Header> {
        match self {
            MultiEraHeader::Babbage(x) => Some(x.deref().deref()),
            _ => None,
        }
    }
}
