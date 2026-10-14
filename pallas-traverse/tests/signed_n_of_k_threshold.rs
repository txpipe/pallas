//! Regression for the preprod tx whose `ScriptNOfK` threshold is negative,
//! which pallas typed `u32`.
use pallas_primitives::alonzo::NativeScript;
use pallas_traverse::{Era, MultiEraTx};

// Fetched from Koios's public preprod API (no credentials required): tx
// b1db2a411cb651a413840d3c8b112895a5bda2519a8ba6a372b8dd1ffc7746c2, block
// 5184855, slot 133902219.
const TX_CBOR: &str = "84a300d9010281825820fa72e22e7661608e75c8837b26bfc7b9fb61ddcf3456f12fb58d57ab7437002f00018182581d60d8188ff2e2bd2384524e2e52b9f0ee0dc19e50ab7baac6d771f6d45e1a001c095b021a00027b25a101d9010281830320828200581c3118644aa21ba172c82732ce80d1c94cdcb5f2e8891e1ad2645707188200581ce07caf4bf751495f75774ace30552441e4df84d141e5d1f5029cb04df5f6";

#[test]
fn preprod_negative_n_of_k_threshold_decodes() {
    let raw = hex::decode(TX_CBOR).unwrap();
    let tx = MultiEraTx::decode_for_era(Era::Conway, &raw).expect("tx should decode");
    assert_eq!(
        tx.hash().to_string(),
        "b1db2a411cb651a413840d3c8b112895a5bda2519a8ba6a372b8dd1ffc7746c2"
    );

    let conway = tx.as_conway().unwrap();
    let scripts = conway
        .transaction_witness_set
        .native_script
        .as_ref()
        .unwrap();
    let script: &NativeScript = &scripts[0];
    let NativeScript::ScriptNOfK(n, sub_scripts) = script else {
        panic!("expected ScriptNOfK, got {script:?}");
    };
    assert_eq!(*n, -1);
    assert_eq!(sub_scripts.len(), 2);
}
