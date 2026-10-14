//! Regression for the preprod block that stopped Dolos on 2026-09-16.
use pallas_codec::minicbor;
use pallas_primitives::alonzo::NativeScript;
use pallas_traverse::{ComputeHash, MultiEraBlock, OriginalHash};

#[test]
fn preprod_deep_native_script_block_on_default_sized_stack() {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let cbor =
                hex::decode(include_str!("../../test_data/preprod-5183974.block").trim()).unwrap();
            let block = MultiEraBlock::decode(&cbor).unwrap();
            assert_eq!(block.slot(), 133_883_340);
            assert_eq!(block.number(), 5_183_974);
            assert_eq!(
                block.hash().to_string(),
                "bac1660208e7a8f63fc7caf97cfefcd3066a517d3ca4d971e1b316d1e43708e5"
            );
            let txs = block.txs();
            assert_eq!(txs.len(), 23);
            assert_eq!(
                txs[3].hash().to_string(),
                "f90dce5765108da976abdbb9fc618f9a6ffd9fa4d93b2f288eed1808545424c9"
            );
            let conway = txs[3].as_conway().unwrap();
            let scripts = conway
                .transaction_witness_set
                .native_script
                .as_ref()
                .unwrap();
            let script = &scripts[0];
            assert_eq!(
                script.original_hash().to_string(),
                "ff3efca65569f6b0b868a3d34abdb1ad8eccf745e0da71fa94fb4f18"
            );
            assert_eq!(script.compute_hash(), script.original_hash());
            assert_eq!(minicbor::to_vec(script).unwrap(), script.raw_cbor());

            let mut cursor: &NativeScript = script;
            let mut depth = 0;
            while let NativeScript::ScriptAll(children) = cursor {
                assert_eq!(children.len(), 1);
                depth += 1;
                cursor = &children[0];
            }
            assert_eq!(depth, 5_383);
            assert!(matches!(cursor, NativeScript::ScriptPubkey(_)));
            let cloned = block.clone();
            assert_eq!(cloned.hash(), block.hash());
            // Block, transaction views and cloned witness trees all drop here.
        })
        .unwrap()
        .join()
        .unwrap();
}
