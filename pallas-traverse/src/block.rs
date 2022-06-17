use std::borrow::Cow;

use pallas_codec::minicbor;
use pallas_crypto::hash::Hash;
use pallas_primitives::{alonzo, byron, ToHash};

use crate::{probe, support, Era, Error, MultiEraBlock, MultiEraTx};

type BlockWrapper<T> = (u16, T);

impl<'b> MultiEraBlock<'b> {
    pub fn decode_epoch_boundary(cbor: &'b [u8]) -> Result<Self, Error> {
        let (_, block): BlockWrapper<byron::EbBlock> =
            minicbor::decode(cbor).map_err(Error::invalid_cbor)?;

        Ok(Self::EpochBoundary(Box::new(Cow::Owned(block))))
    }

    pub fn decode_byron(cbor: &'b [u8]) -> Result<Self, Error> {
        let (_, block): BlockWrapper<byron::MintedBlock> =
            minicbor::decode(cbor).map_err(Error::invalid_cbor)?;

        Ok(Self::Byron(Box::new(Cow::Owned(block))))
    }

    pub fn decode_shelley(cbor: &'b [u8]) -> Result<Self, Error> {
        let (_, block): BlockWrapper<alonzo::MintedBlock> =
            minicbor::decode(cbor).map_err(Error::invalid_cbor)?;

        Ok(Self::AlonzoCompatible(
            Box::new(Cow::Owned(block)),
            Era::Shelley,
        ))
    }

    pub fn decode_allegra(cbor: &'b [u8]) -> Result<Self, Error> {
        let (_, block): BlockWrapper<alonzo::MintedBlock> =
            minicbor::decode(cbor).map_err(Error::invalid_cbor)?;

        Ok(Self::AlonzoCompatible(
            Box::new(Cow::Owned(block)),
            Era::Allegra,
        ))
    }

    pub fn decode_mary(cbor: &'b [u8]) -> Result<Self, Error> {
        let (_, block): BlockWrapper<alonzo::MintedBlock> =
            minicbor::decode(cbor).map_err(Error::invalid_cbor)?;

        Ok(Self::AlonzoCompatible(
            Box::new(Cow::Owned(block)),
            Era::Mary,
        ))
    }

    pub fn decode_alonzo(cbor: &'b [u8]) -> Result<Self, Error> {
        let (_, block): BlockWrapper<alonzo::MintedBlock> =
            minicbor::decode(cbor).map_err(Error::invalid_cbor)?;

        Ok(Self::AlonzoCompatible(
            Box::new(Cow::Owned(block)),
            Era::Alonzo,
        ))
    }

    pub fn decode(cbor: &'b [u8]) -> Result<MultiEraBlock<'b>, Error> {
        match probe::block_era(cbor) {
            probe::Outcome::EpochBoundary => Self::decode_epoch_boundary(cbor),
            probe::Outcome::Matched(era) => match era {
                Era::Byron => Self::decode_byron(cbor),
                Era::Shelley => Self::decode_shelley(cbor),
                Era::Allegra => Self::decode_allegra(cbor),
                Era::Mary => Self::decode_mary(cbor),
                Era::Alonzo => Self::decode_alonzo(cbor),
            },
            probe::Outcome::Inconclusive => Err(Error::unknown_cbor(cbor)),
        }
    }

    pub fn era(&self) -> Era {
        match self {
            MultiEraBlock::EpochBoundary(_) => Era::Byron,
            MultiEraBlock::AlonzoCompatible(_, x) => *x,
            MultiEraBlock::Byron(_) => Era::Byron,
        }
    }

    pub fn hash(&self) -> Hash<32> {
        match self {
            MultiEraBlock::EpochBoundary(x) => x.header.to_hash(),
            MultiEraBlock::AlonzoCompatible(x, _) => x.header.to_hash(),
            MultiEraBlock::Byron(x) => x.header.to_hash(),
        }
    }

    pub fn slot(&self) -> u64 {
        match self {
            MultiEraBlock::EpochBoundary(x) => x.header.to_abs_slot(),
            MultiEraBlock::AlonzoCompatible(x, _) => x.header.header_body.slot,
            MultiEraBlock::Byron(x) => x.header.consensus_data.0.to_abs_slot(),
        }
    }

    pub fn txs(&self) -> Vec<MultiEraTx> {
        match self {
            MultiEraBlock::AlonzoCompatible(x, era) => support::clone_alonzo_txs(x)
                .into_iter()
                .map(|x| MultiEraTx::AlonzoCompatible(Box::new(Cow::Owned(x)), *era))
                .collect(),
            MultiEraBlock::Byron(x) => support::clone_byron_txs(x)
                .into_iter()
                .map(|x| MultiEraTx::Byron(Box::new(Cow::Owned(x))))
                .collect(),
            MultiEraBlock::EpochBoundary(_) => vec![],
        }
    }

    pub fn as_alonzo(&self) -> Option<&alonzo::MintedBlock> {
        match self {
            MultiEraBlock::EpochBoundary(_) => None,
            MultiEraBlock::AlonzoCompatible(x, _) => Some(x),
            MultiEraBlock::Byron(_) => None,
        }
    }

    pub fn as_byron(&self) -> Option<&byron::MintedBlock> {
        match self {
            MultiEraBlock::EpochBoundary(_) => None,
            MultiEraBlock::AlonzoCompatible(_, _) => None,
            MultiEraBlock::Byron(x) => Some(x),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iteration() {
        let blocks = vec![
            (include_str!("../../test_data/byron2.block"), 2usize),
            (include_str!("../../test_data/shelley1.block"), 0),
            (include_str!("../../test_data/mary1.block"), 0),
            (include_str!("../../test_data/allegra1.block"), 0),
            (include_str!("../../test_data/alonzo1.block"), 5),
        ];

        for (block_str, tx_count) in blocks.into_iter() {
            let cbor = hex::decode(block_str).expect("invalid hex");
            let block = MultiEraBlock::decode(&cbor).expect("invalid cbor");
            assert_eq!(block.txs().len(), tx_count);
        }
    }
}
