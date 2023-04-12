use std::borrow::Cow;

use pallas_codec::minicbor;
use pallas_crypto::hash::Hash;
use pallas_primitives::{alonzo, babbage, byron};

use crate::{probe, support, Era, Error, MultiEraBlock, MultiEraHeader, MultiEraTx};

type BlockWrapper<T> = (u16, T);

impl<'b> MultiEraBlock<'b> {
    pub fn decode_epoch_boundary(cbor: &'b [u8]) -> Result<Self, Error> {
        let (_, block): BlockWrapper<byron::MintedEbBlock> =
            minicbor::decode(cbor).map_err(Error::invalid_cbor)?;

        Ok(Self::EpochBoundary(Box::new(block)))
    }

    pub fn decode_byron(cbor: &'b [u8]) -> Result<Self, Error> {
        let (_, block): BlockWrapper<byron::MintedBlock> =
            minicbor::decode(cbor).map_err(Error::invalid_cbor)?;

        Ok(Self::Byron(Box::new(block)))
    }

    pub fn decode_shelley(cbor: &'b [u8]) -> Result<Self, Error> {
        let (_, block): BlockWrapper<alonzo::MintedBlock> =
            minicbor::decode(cbor).map_err(Error::invalid_cbor)?;

        Ok(Self::AlonzoCompatible(Box::new(block), Era::Shelley))
    }

    pub fn decode_allegra(cbor: &'b [u8]) -> Result<Self, Error> {
        let (_, block): BlockWrapper<alonzo::MintedBlock> =
            minicbor::decode(cbor).map_err(Error::invalid_cbor)?;

        Ok(Self::AlonzoCompatible(Box::new(block), Era::Allegra))
    }

    pub fn decode_mary(cbor: &'b [u8]) -> Result<Self, Error> {
        let (_, block): BlockWrapper<alonzo::MintedBlock> =
            minicbor::decode(cbor).map_err(Error::invalid_cbor)?;

        Ok(Self::AlonzoCompatible(Box::new(block), Era::Mary))
    }

    pub fn decode_alonzo(cbor: &'b [u8]) -> Result<Self, Error> {
        let (_, block): BlockWrapper<alonzo::MintedBlock> =
            minicbor::decode(cbor).map_err(Error::invalid_cbor)?;

        Ok(Self::AlonzoCompatible(Box::new(block), Era::Alonzo))
    }

    pub fn decode_babbage(cbor: &'b [u8]) -> Result<Self, Error> {
        let (_, block): BlockWrapper<babbage::MintedBlock> =
            minicbor::decode(cbor).map_err(Error::invalid_cbor)?;

        Ok(Self::Babbage(Box::new(block)))
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
                MultiEraHeader::AlonzoCompatible(Cow::Borrowed(&x.header))
            }
            MultiEraBlock::Babbage(x) => MultiEraHeader::Babbage(Cow::Borrowed(&x.header)),
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
        }
    }

    /// Returns the count of txs in the block
    pub fn tx_count(&self) -> usize {
        match self {
            MultiEraBlock::EpochBoundary(_) => 0,
            MultiEraBlock::AlonzoCompatible(x, _) => x.transaction_bodies.len(),
            MultiEraBlock::Babbage(x) => x.transaction_bodies.len(),
            MultiEraBlock::Byron(x) => x.body.tx_payload.len(),
        }
    }

    /// Returns true if the block has any auxiliary data
    pub fn has_aux_data(&self) -> bool {
        match self {
            MultiEraBlock::EpochBoundary(_) => false,
            MultiEraBlock::AlonzoCompatible(x, _) => !x.auxiliary_data_set.is_empty(),
            MultiEraBlock::Babbage(x) => !x.auxiliary_data_set.is_empty(),
            MultiEraBlock::Byron(_) => false,
        }
    }

    pub fn as_alonzo(&self) -> Option<&alonzo::MintedBlock> {
        match self {
            MultiEraBlock::EpochBoundary(_) => None,
            MultiEraBlock::AlonzoCompatible(x, _) => Some(x),
            MultiEraBlock::Babbage(_) => None,
            MultiEraBlock::Byron(_) => None,
        }
    }

    pub fn as_babbage(&self) -> Option<&babbage::MintedBlock> {
        match self {
            MultiEraBlock::EpochBoundary(_) => None,
            MultiEraBlock::AlonzoCompatible(_, _) => None,
            MultiEraBlock::Babbage(x) => Some(x),
            MultiEraBlock::Byron(_) => None,
        }
    }

    pub fn as_byron(&self) -> Option<&byron::MintedBlock> {
        match self {
            MultiEraBlock::EpochBoundary(_) => None,
            MultiEraBlock::AlonzoCompatible(_, _) => None,
            MultiEraBlock::Babbage(_) => None,
            MultiEraBlock::Byron(x) => Some(x),
        }
    }
}

#[cfg(test)]
mod tests {
    use pallas_codec::utils::KeepRaw;
    use pallas_primitives::{
        babbage::{TransactionInput, TransactionOutput, PseudoTx},
        Fragment,
    };
    
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

    #[test]
    fn test_hydra_tx_with_adjusted_ledger_params() {
        let tx_hex = "84a600838258200c8644756fbd3e75f6702930db5e45aebe3e5bb3aa0d825b2d7895e1955f5e69008258200c8644756fbd3e75f6702930db5e45aebe3e5bb3aa0d825b2d7895e1955f5e69018258200c8644756fbd3e75f6702930db5e45aebe3e5bb3aa0d825b2d7895e1955f5e69020182a300581d70c2d43417ca2475019ca9ae6a0b4393ef8275aefdd26acb792131187e011a000f4240028201d8185873d8799fa3d8799f1a039d3cdb1a002acb8dffd8799f1a00885c760000ffd8799f1a18d485a91a00595cadffd8799f00001a0044d669ffd8799f1a185cede71a00435778ffd8799f001a0023b1fa00ff5820e894512825e98a423cf6ac44bfbe27f17cdf07be6f746cbe9f2d7f07a20d71ab00ff82581d60f8a68cd18e59a6ace848155a0e967af64f4d00cf8acee8adc95a6b0d3a000f423f02000d81825820c4370f145c7c1bb0734ca90e9a93f04ca36f087c91de350a462a14365de29aaf0a1082581d60f8a68cd18e59a6ace848155a0e967af64f4d00cf8acee8adc95a6b0d1a3b9aa106111a000f4240a20583840000d8799f5820e894512825e98a423cf6ac44bfbe27f17cdf07be6f746cbe9f2d7f07a20d71abff820000840001d8799f5820e894512825e98a423cf6ac44bfbe27f17cdf07be6f746cbe9f2d7f07a20d71abff820000840002d8799f5820e894512825e98a423cf6ac44bfbe27f17cdf07be6f746cbe9f2d7f07a20d71abff820000068158fd58fb0100003232323232323232323232323232222533300a323232533300d3370e900000089919251375c6028002601600a264646464944dd6980b000980b0011bae3014001300b005300b0043011001300832533300b3370e9001180500088008a99806a4812a4578706563746564206f6e20696e636f727265637420636f6e7374727563746f722076617269616e742e001633006300800148008526163001001222533300d00214984cc024c004c038008ccc00c00cc03c008004cc0040052000222233330073370e00200601a4666600a00a66e000112002300f0010020022300737540024600a6ea80055cd2b9b5738aae7555cf2ab9f5742ae89f5f6";
        let inputs = "838258200c8644756fbd3e75f6702930db5e45aebe3e5bb3aa0d825b2d7895e1955f5e69008258200c8644756fbd3e75f6702930db5e45aebe3e5bb3aa0d825b2d7895e1955f5e69018258200c8644756fbd3e75f6702930db5e45aebe3e5bb3aa0d825b2d7895e1955f5e6902";
        let outputs = "83a300581d70c2d43417ca2475019ca9ae6a0b4393ef8275aefdd26acb792131187e0100028201d8185836d8799f5820597a7a58d23dc54ad69f86916d59201425585fbd2eec31dca8fc592e28f2c0851a0044d6691a18d485a91a00595cad00ffa300581d70c2d43417ca2475019ca9ae6a0b4393ef8275aefdd26acb792131187e0100028201d8185836d8799f58204210bd8a2c44db9ac68ae56d076bf690fe6bee79e564c0b551e883943aa33aec1a00885c761a039d3cdb1a002acb8d02ffa300581d70c2d43417ca2475019ca9ae6a0b4393ef8275aefdd26acb792131187e0100028201d8185836d8799f5820b7d3edbd94305cd319c1bdb5ac95b176002b55e73b1c5c121cce5d85ee06a2c21a0023b1fa1a185cede71a0043577801ff";

        let tx_bytes: Vec<u8> = hex::decode(tx_hex).unwrap();

        let tx_bytes: &[u8] = tx_bytes.as_slice();
    
        let pseudo: PseudoTx<KeepRaw<babbage::PseudoTransactionBody<babbage::PseudoTransactionOutput<babbage::PseudoPostAlonzoTransactionOutput<babbage::PseudoDatumOption<KeepRaw<babbage::PlutusData>>>>>>, KeepRaw<babbage::MintedWitnessSet>, KeepRaw<babbage::AuxiliaryData>> = minicbor::decode(&tx_bytes).unwrap();
        dbg!(pseudo);

        let tx: MultiEraTx = MultiEraTx::decode(Era::Babbage, &tx_bytes)
            .or_else(|_| MultiEraTx::decode(Era::Alonzo, &tx_bytes))
            .unwrap();
    
        let inputs_bytes: Vec<u8> = hex::decode(inputs).unwrap();
        let outputs_bytes: Vec<u8> = hex::decode(outputs).unwrap();
    
        let inputs = Vec::<TransactionInput>::decode_fragment(&inputs_bytes)
            .unwrap();
    
        let outputs = Vec::<TransactionOutput>::decode_fragment(&outputs_bytes)
            .unwrap();

        let as_babbage = tx.as_babbage();

        dbg!(inputs);
        dbg!(outputs);
        dbg!(as_babbage);
    }

}
