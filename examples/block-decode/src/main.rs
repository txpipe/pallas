use pallas::ledger::traverse::MultiEraBlock;

fn main() {
    let blocks = [
        include_str!("blocks/byron.block"),
        include_str!("blocks/shelley.block"),
        include_str!("blocks/mary.block"),
        include_str!("blocks/allegra.block"),
        include_str!("blocks/alonzo.block"),
    ];

    for block_str in blocks.iter() {
        let cbor = hex::decode(block_str).expect("invalid hex");

        let block = MultiEraBlock::decode(&cbor).expect("invalid cbor");

        println!("{} {}", block.slot(), block.hash());

        for tx in &block.txs() {
            println!("{tx:?}");
        }
    }
}
