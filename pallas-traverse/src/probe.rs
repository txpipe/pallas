//! Lightweight inspection of block data without full CBOR decoding

use pallas_codec::minicbor::{data::Token, decode::Tokenizer};

#[cfg(feature = "unstable")]
use pallas_codec::minicbor::{self, data::Type};

use crate::Era;

#[derive(Debug)]
pub enum Outcome {
    Matched(Era),
    EpochBoundary,
    Inconclusive,
}

/// Which transaction rule a CBOR value's shape matches.
///
/// Dijkstra writes the validity flag last, so its position is the signal.
#[cfg(feature = "unstable")]
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum TxShape {
    /// `[body, witness_set, auxiliary_data/ nil, bool]`, Dijkstra's
    /// `block_transaction`.
    DijkstraBlock,
    /// `[body, witness_set, auxiliary_data/ nil]`, Dijkstra's
    /// `mempool_transaction`.
    DijkstraMempool,
    /// `[body, witness_set, bool, auxiliary_data/ nil]`, which is every era
    /// from Alonzo through Conway, and the form Dijkstra's mempool rule
    /// tolerates when the flag is `true`.
    ValidityThird,
    /// A shape no transaction rule this crate models describes.
    Other,
}

/// Read a transaction's shape without decoding its body or witness set.
#[cfg(feature = "unstable")]
pub(crate) fn tx_shape(cbor: &[u8]) -> TxShape {
    fn indefinite_len(mut d: minicbor::Decoder) -> Result<u64, minicbor::decode::Error> {
        let mut len = 0;

        // Every count above four reads as Other, so the count stops at five.
        while len < 5 && d.datatype()? != Type::Break {
            d.skip()?;
            len += 1;
        }

        Ok(len)
    }

    fn read(cbor: &[u8]) -> Result<TxShape, minicbor::decode::Error> {
        let mut d = minicbor::Decoder::new(cbor);

        let len = match d.array()? {
            Some(len) => len,
            None => indefinite_len(d.clone())?,
        };

        if len != 3 && len != 4 {
            return Ok(TxShape::Other);
        }

        d.skip()?;
        d.skip()?;

        let third_is_bool = d.datatype()? == Type::Bool;

        if len == 3 {
            return Ok(if third_is_bool {
                TxShape::Other
            } else {
                TxShape::DijkstraMempool
            });
        }

        if third_is_bool {
            return Ok(TxShape::ValidityThird);
        }

        d.skip()?;

        Ok(if d.datatype()? == Type::Bool {
            TxShape::DijkstraBlock
        } else {
            TxShape::Other
        })
    }

    read(cbor).unwrap_or(TxShape::Other)
}

// Executes a very lightweight inspection of the initial tokens of the CBOR
// block payload to extract the tag of the block wrapper which defines the era
// of the contained bytes.
pub fn block_era(cbor: &[u8]) -> Outcome {
    let mut tokenizer = Tokenizer::new(cbor);

    if !matches!(tokenizer.next(), Some(Ok(Token::Array(2)))) {
        return Outcome::Inconclusive;
    }

    match tokenizer.next() {
        Some(Ok(Token::U8(variant))) => match variant {
            0 => Outcome::EpochBoundary,
            1 => Outcome::Matched(Era::Byron),
            2 => Outcome::Matched(Era::Shelley),
            3 => Outcome::Matched(Era::Allegra),
            4 => Outcome::Matched(Era::Mary),
            5 => Outcome::Matched(Era::Alonzo),
            6 => Outcome::Matched(Era::Babbage),
            7 => Outcome::Matched(Era::Conway),
            #[cfg(feature = "unstable")]
            8 => Outcome::Matched(Era::Dijkstra),
            _ => Outcome::Inconclusive,
        },
        _ => Outcome::Inconclusive,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn genesis_block_detected() {
        let block_str = include_str!("../../test_data/genesis.block");
        let bytes = hex::decode(block_str).unwrap();

        let inference = block_era(bytes.as_slice());

        assert!(matches!(inference, Outcome::EpochBoundary));
    }

    #[test]
    fn byron_block_detected() {
        let block_str = include_str!("../../test_data/byron1.block");
        let bytes = hex::decode(block_str).unwrap();

        let inference = block_era(bytes.as_slice());

        assert!(matches!(inference, Outcome::Matched(Era::Byron)));
    }

    #[test]
    fn shelley_block_detected() {
        let block_str = include_str!("../../test_data/shelley1.block");
        let bytes = hex::decode(block_str).unwrap();

        let inference = block_era(bytes.as_slice());

        assert!(matches!(inference, Outcome::Matched(Era::Shelley)));
    }

    #[test]
    fn allegra_block_detected() {
        let block_str = include_str!("../../test_data/allegra1.block");
        let bytes = hex::decode(block_str).unwrap();

        let inference = block_era(bytes.as_slice());

        assert!(matches!(inference, Outcome::Matched(Era::Allegra)));
    }

    #[test]
    fn mary_block_detected() {
        let block_str = include_str!("../../test_data/mary1.block");
        let bytes = hex::decode(block_str).unwrap();

        let inference = block_era(bytes.as_slice());

        assert!(matches!(inference, Outcome::Matched(Era::Mary)));
    }

    #[test]
    fn alonzo_block_detected() {
        let block_str = include_str!("../../test_data/alonzo1.block");
        let bytes = hex::decode(block_str).unwrap();

        let inference = block_era(bytes.as_slice());

        assert!(matches!(inference, Outcome::Matched(Era::Alonzo)));
    }

    #[test]
    fn babbage_block_detected() {
        let block_str = include_str!("../../test_data/babbage1.block");
        let bytes = hex::decode(block_str).unwrap();

        let inference = block_era(bytes.as_slice());

        assert!(matches!(inference, Outcome::Matched(Era::Babbage)));
    }

    #[test]
    fn conway_block_detected() {
        let block_str = include_str!("../../test_data/conway1.block");
        let bytes = hex::decode(block_str).unwrap();

        let inference = block_era(bytes.as_slice());

        assert!(matches!(inference, Outcome::Matched(Era::Conway)));
    }

    #[cfg(feature = "unstable")]
    #[test]
    fn dijkstra_block_detected() {
        let block_str = include_str!("../../test_data/dijkstra1.block");
        let bytes = hex::decode(block_str).unwrap();

        let inference = block_era(bytes.as_slice());

        assert!(matches!(inference, Outcome::Matched(Era::Dijkstra)));
    }

    #[cfg(not(feature = "unstable"))]
    #[test]
    fn dijkstra_block_is_inconclusive_without_the_feature() {
        let block_str = include_str!("../../test_data/dijkstra1.block");
        let bytes = hex::decode(block_str).unwrap();

        let inference = block_era(bytes.as_slice());

        assert!(matches!(inference, Outcome::Inconclusive));
    }

    #[cfg(feature = "unstable")]
    fn first_tx_bytes(block_str: &str) -> Vec<u8> {
        let cbor = hex::decode(block_str).unwrap();
        let block = crate::MultiEraBlock::decode(&cbor).unwrap();
        block.txs().first().unwrap().encode()
    }

    #[cfg(feature = "unstable")]
    #[test]
    fn the_validity_flag_position_tells_the_two_four_element_shapes_apart() {
        let dijkstra = first_tx_bytes(include_str!("../../test_data/dijkstra3.block"));
        assert_eq!(tx_shape(&dijkstra), TxShape::DijkstraBlock);

        let conway = first_tx_bytes(include_str!("../../test_data/conway1.block"));
        assert_eq!(tx_shape(&conway), TxShape::ValidityThird);

        let babbage = first_tx_bytes(include_str!("../../test_data/babbage6.block"));
        assert_eq!(tx_shape(&babbage), TxShape::ValidityThird);
    }

    #[cfg(feature = "unstable")]
    #[test]
    fn a_three_element_transaction_is_the_mempool_shape() {
        let dijkstra = first_tx_bytes(include_str!("../../test_data/dijkstra3.block"));
        let block_tx: pallas_primitives::dijkstra::BlockTransaction =
            minicbor::decode(&dijkstra).unwrap();

        let mempool = minicbor::to_vec(block_tx.to_mempool_transaction()).unwrap();
        assert_eq!(tx_shape(&mempool), TxShape::DijkstraMempool);
    }

    #[cfg(feature = "unstable")]
    #[test]
    fn an_indefinite_array_is_read_by_the_same_rules_as_a_definite_one() {
        let dijkstra = first_tx_bytes(include_str!("../../test_data/dijkstra3.block"));
        let tx: pallas_primitives::dijkstra::BlockTransaction =
            minicbor::decode(&dijkstra).unwrap();

        let body = tx.transaction_body.raw_cbor();
        let witnesses = tx.transaction_witness_set.raw_cbor();
        let aux: &[u8] = match &tx.auxiliary_data {
            pallas_codec::utils::Nullable::Some(aux) => aux.raw_cbor(),
            _ => &[0xf6],
        };

        for (items, shape) in [
            (vec![body, witnesses, aux], TxShape::DijkstraMempool),
            (vec![body, witnesses, &[0xf5], aux], TxShape::ValidityThird),
            (vec![body, witnesses, aux, &[0xf5]], TxShape::DijkstraBlock),
        ] {
            let indefinite = [&[0x9f][..], &items.concat(), &[0xff]].concat();
            assert_eq!(tx_shape(&indefinite), shape);
        }
    }

    #[cfg(feature = "unstable")]
    #[test]
    fn a_shape_no_rule_describes_is_reported_as_other() {
        for bytes in [
            // an empty array
            vec![0x80],
            // a two element array
            vec![0x82, 0x01, 0x02],
            // a five element array
            vec![0x85, 0x01, 0x02, 0x03, 0x04, 0x05],
            // a map rather than an array
            vec![0xa0],
            // a four element array with no bool at either position
            vec![0x84, 0x01, 0x02, 0x03, 0x04],
            // truncated after the array head
            vec![0x84],
            // an empty indefinite array
            vec![0x9f, 0xff],
            // an indefinite array of two elements
            vec![0x9f, 0x01, 0x02, 0xff],
            // an indefinite array of three elements with a bool third
            vec![0x9f, 0x01, 0x02, 0xf5, 0xff],
            // an indefinite array of four elements with no bool at either position
            vec![0x9f, 0x01, 0x02, 0x03, 0x04, 0xff],
            // an indefinite array of five elements
            vec![0x9f, 0x01, 0x02, 0x03, 0x04, 0xf5, 0xff],
            // an indefinite array with no break
            vec![0x9f, 0x01, 0x02, 0x03],
        ] {
            assert_eq!(tx_shape(&bytes), TxShape::Other, "{}", hex::encode(&bytes));
        }
    }
}
