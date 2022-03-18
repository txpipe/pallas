use std::fmt::Debug;

use net2::TcpStreamExt;

use pallas::ledger::primitives::{alonzo, byron, probing, Era, Fragment};

fn pretty_print(block: impl Debug) {
    println!("{:?}", block)
}

fn main() {
    let blocks = vec![
        include_str!("blocks/byron.block"),
        include_str!("blocks/shelley.block"),
        include_str!("blocks/mary.block"),
        include_str!("blocks/allegra.block"),
        include_str!("blocks/alonzo.block"),
    ];

    for (idx, block_str) in blocks.iter().enumerate() {
        let bytes = hex::decode(block_str).expect("valid hex");

        match probing::probe_block_cbor_era(&bytes) {
            probing::Outcome::Matched(era) => match era {
                Era::Byron => pretty_print(byron::Block::decode_fragment(&bytes)),
                // we use alonzo for everything post-shelly since it's backward compatible
                Era::Shelley => pretty_print(alonzo::BlockWrapper::decode_fragment(&bytes)),
                Era::Allegra => pretty_print(alonzo::BlockWrapper::decode_fragment(&bytes)),
                Era::Mary => pretty_print(alonzo::BlockWrapper::decode_fragment(&bytes)),
                Era::Alonzo => pretty_print(alonzo::BlockWrapper::decode_fragment(&bytes)),
            },
            _ => println!("couldn't infer block era"),
        };
    }
}
