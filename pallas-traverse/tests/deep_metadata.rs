//! Real Preview metadata must decode, encode and clone. `Drop` still recurses,
//! so the deep values are leaked.
use pallas_codec::tree::IndexedNode;
use pallas_traverse::MultiEraBlock;
use std::mem::ManuallyDrop;

#[test]
fn preview_deep_metadata_decodes_and_clones_on_a_default_sized_stack() {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let bytes =
                hex::decode(include_str!("../../test_data/preview-4821137.block").trim()).unwrap();
            let block = ManuallyDrop::new(MultiEraBlock::decode(&bytes).unwrap());
            assert_eq!(block.number(), 4_821_137);
            assert_eq!(block.slot(), 127_055_999);
            assert_eq!(
                block.hash().to_string(),
                "e5a2152e974d69023be7e7bb8e96141e65d28c2473836285ef06f4575833cb87"
            );
            let txs = ManuallyDrop::new(block.txs());
            assert_eq!(
                txs[7].hash().to_string(),
                "ede279c9cf7f9d4bb909f03ae06af5afedada6aaa9ca305c2cd6df3a4fd179e5"
            );
            let metadata = txs[7].metadata();
            let mut node = metadata.as_alonzo().unwrap().get(&1).unwrap();
            let mut depth = 0;
            while node.child_count() > 0 {
                assert_eq!(node.child_count(), 1);
                node = node.child(0);
                depth += 1;
            }
            assert_eq!(depth, 16_053);
            let encoded = txs[7].encode();
            let decoded = ManuallyDrop::new(pallas_traverse::MultiEraTx::decode(&encoded).unwrap());
            assert_eq!(decoded.hash(), txs[7].hash());
            let cloned = ManuallyDrop::new(MultiEraBlock::clone(&block));
            assert_eq!(cloned.hash(), block.hash());
        })
        .unwrap()
        .join()
        .unwrap();
}
