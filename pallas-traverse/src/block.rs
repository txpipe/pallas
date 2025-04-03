use std::{borrow::Cow, ops::Deref};

use pallas_codec::minicbor;
use pallas_crypto::hash::Hash;
use pallas_primitives::{alonzo, babbage, byron, conway};

use crate::{
    probe, support, Era, Error, MultiEraBlock, MultiEraHeader, MultiEraTx, MultiEraUpdate,
};

type BlockWrapper<T> = (u16, T);

impl<'b> MultiEraBlock<'b> {
    pub fn decode_epoch_boundary(cbor: &'b [u8]) -> Result<Self, Error> {
        let (_, block): BlockWrapper<byron::EbBlock> =
            minicbor::decode(cbor).map_err(Error::invalid_cbor)?;

        Ok(Self::EpochBoundary(Box::new(block)))
    }

    pub fn decode_byron(cbor: &'b [u8]) -> Result<Self, Error> {
        let (_, block): BlockWrapper<byron::Block> =
            minicbor::decode(cbor).map_err(Error::invalid_cbor)?;

        Ok(Self::Byron(Box::new(block)))
    }

    pub fn decode_shelley(cbor: &'b [u8]) -> Result<Self, Error> {
        let (_, block): BlockWrapper<alonzo::Block> =
            minicbor::decode(cbor).map_err(Error::invalid_cbor)?;

        Ok(Self::AlonzoCompatible(Box::new(block), Era::Shelley))
    }

    pub fn decode_allegra(cbor: &'b [u8]) -> Result<Self, Error> {
        let (_, block): BlockWrapper<alonzo::Block> =
            minicbor::decode(cbor).map_err(Error::invalid_cbor)?;

        Ok(Self::AlonzoCompatible(Box::new(block), Era::Allegra))
    }

    pub fn decode_mary(cbor: &'b [u8]) -> Result<Self, Error> {
        let (_, block): BlockWrapper<alonzo::Block> =
            minicbor::decode(cbor).map_err(Error::invalid_cbor)?;

        Ok(Self::AlonzoCompatible(Box::new(block), Era::Mary))
    }

    pub fn decode_alonzo(cbor: &'b [u8]) -> Result<Self, Error> {
        let (_, block): BlockWrapper<alonzo::Block> =
            minicbor::decode(cbor).map_err(Error::invalid_cbor)?;

        Ok(Self::AlonzoCompatible(Box::new(block), Era::Alonzo))
    }

    pub fn decode_babbage(cbor: &'b [u8]) -> Result<Self, Error> {
        let (_, block): BlockWrapper<babbage::Block> =
            minicbor::decode(cbor).map_err(Error::invalid_cbor)?;

        Ok(Self::Babbage(Box::new(block)))
    }

    pub fn decode_conway(cbor: &'b [u8]) -> Result<Self, Error> {
        let (_, block): BlockWrapper<conway::Block> =
            minicbor::decode(cbor).map_err(Error::invalid_cbor)?;

        Ok(Self::Conway(Box::new(block)))
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
                Era::Babbage => Self::decode_babbage(cbor),
                Era::Conway => Self::decode_conway(cbor),
            },
            probe::Outcome::Inconclusive => Err(Error::unknown_cbor(cbor)),
        }
    }

    pub fn header(&self) -> MultiEraHeader<'_> {
        match self {
            MultiEraBlock::EpochBoundary(x) => {
                MultiEraHeader::EpochBoundary(Cow::Borrowed(&x.header))
            }
            MultiEraBlock::Byron(x) => MultiEraHeader::Byron(Cow::Borrowed(&x.header)),
            MultiEraBlock::AlonzoCompatible(x, _) => {
                MultiEraHeader::ShelleyCompatible(Cow::Borrowed(&x.header))
            }
            MultiEraBlock::Babbage(x) => {
                MultiEraHeader::BabbageCompatible(Cow::Borrowed(&x.header))
            }
            MultiEraBlock::Conway(x) => MultiEraHeader::BabbageCompatible(Cow::Borrowed(&x.header)),
        }
    }

    /// Returns the block number (aka: height)
    pub fn number(&self) -> u64 {
        self.header().number()
    }

    pub fn era(&self) -> Era {
        match self {
            MultiEraBlock::EpochBoundary(_) => Era::Byron,
            MultiEraBlock::AlonzoCompatible(_, x) => *x,
            MultiEraBlock::Babbage(_) => Era::Babbage,
            MultiEraBlock::Byron(_) => Era::Byron,
            MultiEraBlock::Conway(_) => Era::Conway,
        }
    }

    pub fn hash(&self) -> Hash<32> {
        self.header().hash()
    }

    pub fn slot(&self) -> u64 {
        self.header().slot()
    }

    /// Builds a vec with the Txs of the block
    pub fn txs(&self) -> Vec<MultiEraTx> {
        match self {
            MultiEraBlock::AlonzoCompatible(x, era) => support::clone_alonzo_txs(x)
                .into_iter()
                .map(|x| MultiEraTx::AlonzoCompatible(Box::new(Cow::Owned(x)), *era))
                .collect(),
            MultiEraBlock::Babbage(x) => support::clone_babbage_txs(x)
                .into_iter()
                .map(|x| MultiEraTx::Babbage(Box::new(Cow::Owned(x))))
                .collect(),
            MultiEraBlock::Byron(x) => support::clone_byron_txs(x)
                .into_iter()
                .map(|x| MultiEraTx::Byron(Box::new(Cow::Owned(x))))
                .collect(),
            MultiEraBlock::Conway(x) => support::clone_conway_txs(x)
                .into_iter()
                .map(|x| MultiEraTx::Conway(Box::new(Cow::Owned(x))))
                .collect(),
            MultiEraBlock::EpochBoundary(_) => vec![],
        }
    }

    /// Returns true if the there're no tx in the block
    pub fn is_empty(&self) -> bool {
        match self {
            MultiEraBlock::EpochBoundary(_) => true,
            MultiEraBlock::AlonzoCompatible(x, _) => x.transaction_bodies.is_empty(),
            MultiEraBlock::Babbage(x) => x.transaction_bodies.is_empty(),
            MultiEraBlock::Byron(x) => x.body.tx_payload.is_empty(),
            MultiEraBlock::Conway(x) => x.transaction_bodies.is_empty(),
        }
    }

    /// Returns the count of txs in the block
    pub fn tx_count(&self) -> usize {
        match self {
            MultiEraBlock::EpochBoundary(_) => 0,
            MultiEraBlock::AlonzoCompatible(x, _) => x.transaction_bodies.len(),
            MultiEraBlock::Babbage(x) => x.transaction_bodies.len(),
            MultiEraBlock::Byron(x) => x.body.tx_payload.len(),
            MultiEraBlock::Conway(x) => x.transaction_bodies.len(),
        }
    }

    /// Returns true if the block has any auxiliary data
    pub fn has_aux_data(&self) -> bool {
        match self {
            MultiEraBlock::EpochBoundary(_) => false,
            MultiEraBlock::AlonzoCompatible(x, _) => !x.auxiliary_data_set.is_empty(),
            MultiEraBlock::Babbage(x) => !x.auxiliary_data_set.is_empty(),
            MultiEraBlock::Byron(_) => false,
            MultiEraBlock::Conway(x) => !x.auxiliary_data_set.is_empty(),
        }
    }

    /// Returns any block-level param update proposals (byron-specific)
    pub fn update(&self) -> Option<MultiEraUpdate> {
        match self {
            MultiEraBlock::Byron(x) => {
                if let Some(up) = x.body.upd_payload.proposal.deref() {
                    let epoch = x.header.consensus_data.0.epoch;
                    Some(MultiEraUpdate::Byron(
                        epoch,
                        Box::new(Cow::Owned(up.clone())),
                    ))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn as_alonzo(&self) -> Option<&alonzo::Block> {
        match self {
            MultiEraBlock::AlonzoCompatible(x, _) => Some(x),
            _ => None,
        }
    }

    pub fn as_babbage(&self) -> Option<&babbage::Block> {
        match self {
            MultiEraBlock::Babbage(x) => Some(x),
            _ => None,
        }
    }

    pub fn as_byron(&self) -> Option<&byron::Block> {
        match self {
            MultiEraBlock::Byron(x) => Some(x),
            _ => None,
        }
    }

    pub fn as_conway(&self) -> Option<&conway::Block> {
        match self {
            MultiEraBlock::Conway(x) => Some(x),
            _ => None,
        }
    }

    /// Return the size of the serialised block in bytes
    pub fn size(&self) -> usize {
        match self {
            MultiEraBlock::EpochBoundary(b) => minicbor::to_vec(b).unwrap().len(),
            MultiEraBlock::Byron(b) => minicbor::to_vec(b).unwrap().len(),
            MultiEraBlock::AlonzoCompatible(b, _) => minicbor::to_vec(b).unwrap().len(),
            MultiEraBlock::Babbage(b) => minicbor::to_vec(b).unwrap().len(),
            MultiEraBlock::Conway(b) => minicbor::to_vec(b).unwrap().len(),
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
            (include_str!("../../test_data/shelley1.block"), 4),
            (include_str!("../../test_data/mary1.block"), 14),
            (include_str!("../../test_data/allegra1.block"), 3),
            (include_str!("../../test_data/alonzo1.block"), 5),
        ];

        for (block_str, tx_count) in blocks.into_iter() {
            let cbor = hex::decode(block_str).expect("invalid hex");
            let block = MultiEraBlock::decode(&cbor).expect("invalid cbor");
            assert_eq!(block.txs().len(), tx_count);
        }
    }
}
