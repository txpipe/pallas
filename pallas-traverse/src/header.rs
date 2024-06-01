use std::borrow::Cow;
use std::ops::Deref;

use pallas_codec::minicbor;
use pallas_crypto::hash::{Hash, Hasher};
use pallas_primitives::{alonzo, babbage, byron};

use crate::{wellknown::GenesisValues, Era, Error, MultiEraHeader, OriginalHash};

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
            1..=4 => {
                let header = minicbor::decode(cbor).map_err(Error::invalid_cbor)?;
                Ok(MultiEraHeader::ShelleyCompatible(Cow::Owned(header)))
            }
            _ => {
                let header = minicbor::decode(cbor).map_err(Error::invalid_cbor)?;
                Ok(MultiEraHeader::BabbageCompatible(Cow::Owned(header)))
            }
        }
    }

    pub fn cbor(&self) -> &'b [u8] {
        match self {
            MultiEraHeader::EpochBoundary(x) => x.raw_cbor(),
            MultiEraHeader::ShelleyCompatible(x) => x.raw_cbor(),
            MultiEraHeader::BabbageCompatible(x) => x.raw_cbor(),
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
            MultiEraHeader::ShelleyCompatible(x) => x.header_body.block_number,
            MultiEraHeader::BabbageCompatible(x) => x.header_body.block_number,
            MultiEraHeader::Byron(x) => x.consensus_data.2.first().cloned().unwrap_or_default(),
        }
    }

    pub fn slot(&self) -> u64 {
        match self {
            MultiEraHeader::ShelleyCompatible(x) => x.header_body.slot,
            MultiEraHeader::BabbageCompatible(x) => x.header_body.slot,
            MultiEraHeader::EpochBoundary(x) => {
                let genesis = GenesisValues::default();
                genesis.relative_slot_to_absolute(x.consensus_data.epoch_id, 0)
            }
            MultiEraHeader::Byron(x) => {
                let genesis = GenesisValues::default();
                genesis.relative_slot_to_absolute(x.consensus_data.0.epoch, x.consensus_data.0.slot)
            }
        }
    }

    pub fn hash(&self) -> Hash<32> {
        match self {
            MultiEraHeader::EpochBoundary(x) => x.original_hash(),
            MultiEraHeader::ShelleyCompatible(x) => x.original_hash(),
            MultiEraHeader::BabbageCompatible(x) => x.original_hash(),
            MultiEraHeader::Byron(x) => x.original_hash(),
        }
    }

    pub fn previous_hash(&self) -> Option<Hash<32>> {
        match self {
            MultiEraHeader::ShelleyCompatible(x) => x.header_body.prev_hash,
            MultiEraHeader::BabbageCompatible(x) => x.header_body.prev_hash,
            MultiEraHeader::EpochBoundary(x) => Some(x.prev_block),
            MultiEraHeader::Byron(x) => Some(x.prev_block),
        }
    }

    pub fn vrf_vkey(&self) -> Option<&[u8]> {
        match self {
            MultiEraHeader::ShelleyCompatible(x) => Some(x.header_body.vrf_vkey.as_ref()),
            MultiEraHeader::BabbageCompatible(x) => Some(x.header_body.vrf_vkey.as_ref()),
            MultiEraHeader::EpochBoundary(_) => None,
            MultiEraHeader::Byron(_) => None,
        }
    }

    pub fn issuer_vkey(&self) -> Option<&[u8]> {
        match self {
            MultiEraHeader::ShelleyCompatible(x) => Some(x.header_body.issuer_vkey.as_ref()),
            MultiEraHeader::BabbageCompatible(x) => Some(x.header_body.issuer_vkey.as_ref()),
            MultiEraHeader::EpochBoundary(_) => None,
            MultiEraHeader::Byron(_) => None,
        }
    }

    pub fn leader_vrf_output(&self) -> Result<Vec<u8>, Error> {
        match self {
            MultiEraHeader::EpochBoundary(_) => Err(Error::InvalidEra(Era::Byron)),
            MultiEraHeader::ShelleyCompatible(x) => Ok(x.header_body.leader_vrf.0.to_vec()),
            MultiEraHeader::BabbageCompatible(x) => {
                let mut leader_tagged_vrf: Vec<u8> = vec![0x4C_u8]; /* "L" */
                leader_tagged_vrf.extend(&*x.header_body.vrf_result.0);
                Ok(Hasher::<256>::hash(&leader_tagged_vrf).to_vec())
            }
            MultiEraHeader::Byron(_) => Err(Error::InvalidEra(Era::Byron)),
        }
    }

    pub fn nonce_vrf_output(&self) -> Result<Vec<u8>, Error> {
        match self {
            MultiEraHeader::EpochBoundary(_) => Err(Error::InvalidEra(Era::Byron)),
            MultiEraHeader::ShelleyCompatible(x) => Ok(x.header_body.nonce_vrf.0.to_vec()),
            MultiEraHeader::BabbageCompatible(x) => {
                let mut nonce_tagged_vrf: Vec<u8> = vec![0x4E_u8]; /* "N" */
                nonce_tagged_vrf.extend(&*x.header_body.vrf_result.0);
                Ok(Hasher::<256>::hash(&nonce_tagged_vrf).to_vec())
            }
            MultiEraHeader::Byron(_) => Err(Error::InvalidEra(Era::Byron)),
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
            MultiEraHeader::ShelleyCompatible(x) => Some(x.deref().deref()),
            _ => None,
        }
    }

    pub fn as_babbage(&self) -> Option<&babbage::Header> {
        match self {
            MultiEraHeader::BabbageCompatible(x) => Some(x.deref().deref()),
            _ => None,
        }
    }
}
