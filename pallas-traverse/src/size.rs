use pallas_codec::utils::Nullable;

use crate::{MultiEraBlock, MultiEraTx};

impl MultiEraTx<'_> {
    fn aux_data_size(&self) -> usize {
        match self {
            MultiEraTx::AlonzoCompatible(x, _) => match &x.auxiliary_data {
                Nullable::Some(x) => x.raw_cbor().len() + 1,
                _ => 2,
            },
            MultiEraTx::Babbage(x) => match &x.auxiliary_data {
                Nullable::Some(x) => x.raw_cbor().len() + 1,
                _ => 2,
            },
            MultiEraTx::Byron(_) => 0,
            MultiEraTx::Conway(x) => match &x.auxiliary_data {
                Nullable::Some(x) => x.raw_cbor().len() + 1,
                _ => 2,
            },
        }
    }

    fn body_size(&self) -> usize {
        match self {
            MultiEraTx::AlonzoCompatible(x, _) => x.transaction_body.raw_cbor().len(),
            MultiEraTx::Babbage(x) => x.transaction_body.raw_cbor().len(),
            MultiEraTx::Byron(x) => x.transaction.raw_cbor().len(),
            MultiEraTx::Conway(x) => x.transaction_body.raw_cbor().len(),
        }
    }

    fn witness_set_size(&self) -> usize {
        match self {
            MultiEraTx::AlonzoCompatible(x, _) => x.transaction_witness_set.raw_cbor().len(),
            MultiEraTx::Babbage(x) => x.transaction_witness_set.raw_cbor().len(),
            MultiEraTx::Byron(x) => x.witness.raw_cbor().len(),
            MultiEraTx::Conway(x) => x.transaction_witness_set.raw_cbor().len(),
        }
    }

    pub fn size(&self) -> usize {
        match self {
            MultiEraTx::Byron(_) => self.body_size(),
            _ => self.body_size() + self.witness_set_size() + self.aux_data_size(),
        }
    }
}

impl MultiEraBlock<'_> {
    pub fn body_size(&self) -> Option<usize> {
        match self {
            MultiEraBlock::AlonzoCompatible(x, _) => {
                Some(x.header.header_body.block_body_size as usize)
            }
            MultiEraBlock::Babbage(x) => Some(x.header.header_body.block_body_size as usize),
            MultiEraBlock::EpochBoundary(_) => None,
            MultiEraBlock::Byron(_) => None,
            MultiEraBlock::Conway(x) => Some(x.header.header_body.block_body_size as usize),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::MultiEraTx;

    #[test]
    fn known_size_matches() {
        let cbor = hex::decode(include_str!("../../test_data/alonzo1.tx")).expect("invalid hex");
        let tx = MultiEraTx::decode(&cbor).expect("invalid cbor");

        assert_eq!(tx.size(), 265);

        let cbor = hex::decode(include_str!("../../test_data/byron1.tx")).expect("invalid hex");
        let tx = MultiEraTx::decode(&cbor).expect("invalid cbor");

        assert_eq!(tx.size(), 220);

        let cbor = hex::decode(include_str!("../../test_data/conway1.tx")).expect("invalid hex");
        let tx = MultiEraTx::decode(&cbor).expect("invalid cbor");

        assert_eq!(tx.size(), 1096);

        let cbor = hex::decode(include_str!("../../test_data/mary1.tx")).expect("invalid hex");
        let tx = MultiEraTx::decode(&cbor).expect("invalid cbor");

        assert_eq!(tx.size(), 439);

        let cbor = hex::decode(include_str!("../../test_data/babbage2.tx")).expect("invalid hex");
        let tx = MultiEraTx::decode(&cbor).expect("invalid cbor");

        assert_eq!(tx.size(), 1748);

        let cbor = hex::decode(include_str!("../../test_data/shelley1.tx")).expect("invalid hex");
        let tx = MultiEraTx::decode(&cbor).expect("invalid cbor");

        assert_eq!(tx.size(), 293);

        let cbor = hex::decode(include_str!("../../test_data/conway7.tx")).expect("invalid hex");
        let tx = MultiEraTx::decode(&cbor).expect("invalid cbor");

        assert_eq!(tx.size(), 3396);
    }
}
