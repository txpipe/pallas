use pallas_codec::minicbor;
use pallas_crypto::hash::Hash;
use pallas_primitives::{alonzo, byron, ToHash};

use crate::{probe, Era, Error, MultiEraBlock};

type BlockWrapper<T> = (u16, T);

impl<'b> MultiEraBlock<'b> {
    pub fn decode(cbor: &[u8]) -> Result<MultiEraBlock, Error> {
        match probe::block_era(&cbor) {
            probe::Outcome::EpochBoundary => {
                let (_, block): BlockWrapper<byron::EbBlock> =
                    minicbor::decode(&cbor).map_err(Error::invalid_cbor)?;

                Ok(MultiEraBlock::EpochBoundary(block))
            }
            probe::Outcome::Matched(era) => match era {
                Era::Byron => {
                    let (_, block): BlockWrapper<byron::MintedBlock> =
                        minicbor::decode(&cbor).map_err(Error::invalid_cbor)?;

                    Ok(MultiEraBlock::Byron(block))
                }
                Era::Shelley | Era::Allegra | Era::Mary | Era::Alonzo => {
                    let (_, block): BlockWrapper<alonzo::MintedBlock> =
                        minicbor::decode(&cbor).map_err(Error::invalid_cbor)?;

                    Ok(MultiEraBlock::AlonzoCompatible(block))
                }
            },
            probe::Outcome::Inconclusive => Err(Error::unknown_cbor(cbor)),
        }
    }

    pub fn hash(&self) -> Hash<32> {
        match self {
            MultiEraBlock::EpochBoundary(x) => x.header.to_hash(),
            MultiEraBlock::AlonzoCompatible(x) => x.header.to_hash(),
            MultiEraBlock::Byron(x) => x.header.to_hash(),
        }
    }

    pub fn slot(&self) -> u64 {
        match self {
            MultiEraBlock::EpochBoundary(x) => x.header.to_abs_slot(),
            MultiEraBlock::AlonzoCompatible(x) => x.header.header_body.slot,
            MultiEraBlock::Byron(x) => x.header.consensus_data.0.to_abs_slot(),
        }
    }
}
