use pallas::ledger::primitives::{alonzo, byron, probing, Era};

fn main() {
    let blocks = vec![
        include_str!("blocks/byron.block"),
        include_str!("blocks/shelley.block"),
        include_str!("blocks/mary.block"),
        include_str!("blocks/allegra.block"),
        include_str!("blocks/alonzo.block"),
    ];

    for block_str in blocks.iter() {
        let bytes = hex::decode(block_str).expect("invalid hex");

        match probing::probe_block_cbor_era(&bytes) {
            probing::Outcome::Matched(era) => match era {
                Era::Byron => {
                    let (_, block): (u16, byron::MainBlock) =
                        pallas::codec::minicbor::decode(&bytes).expect("invalid cbor");
                    println!("{:?}", block)
                }
                // we use alonzo for everything post-shelly since it's backward compatible
                Era::Shelley | Era::Allegra | Era::Mary | Era::Alonzo => {
                    let (_, block): (u16, alonzo::Block) =
                        pallas::codec::minicbor::decode(&bytes).expect("invalid cbor");
                    println!("{:?}", block)
                }
            },
            _ => println!("couldn't infer block era"),
        };
    }
}
