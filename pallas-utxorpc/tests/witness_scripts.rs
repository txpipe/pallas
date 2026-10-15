use pallas_traverse::{Era, MultiEraTx};
use pallas_utxorpc::{LedgerContext, TxoRef, UtxoMap, v1alpha, v1beta};

#[derive(Clone)]
struct NoLedger;

impl LedgerContext for NoLedger {
    fn get_utxos(&self, _refs: &[TxoRef]) -> Option<UtxoMap> {
        None
    }

    fn get_slot_timestamp(&self, _slot: u64) -> Option<u64> {
        None
    }
}

fn conway_tx(cbor: &[u8]) -> MultiEraTx<'_> {
    MultiEraTx::decode_for_era(Era::Conway, cbor).unwrap()
}

fn v3_scripts_alpha(tx: &MultiEraTx) -> Vec<Vec<u8>> {
    use v1alpha::spec::cardano::script::Script;

    let mapped = v1alpha::Mapper::new(NoLedger).map_tx(tx);
    mapped
        .witnesses
        .unwrap()
        .script
        .into_iter()
        .filter_map(|x| match x.script {
            Some(Script::PlutusV3(bytes)) => Some(bytes.to_vec()),
            _ => None,
        })
        .collect()
}

fn v3_scripts_beta(tx: &MultiEraTx) -> Vec<Vec<u8>> {
    use v1beta::spec::cardano::script::Script;

    let mapped = v1beta::Mapper::new(NoLedger).map_tx(tx);
    mapped
        .witnesses
        .unwrap()
        .script
        .into_iter()
        .filter_map(|x| match x.script {
            Some(Script::PlutusV3(bytes)) => Some(bytes.to_vec()),
            _ => None,
        })
        .collect()
}

#[test]
fn a_conway_transaction_maps_its_plutus_v3_witness_script() {
    let cbor = hex::decode(include_str!("../../test_data/conway9.tx")).unwrap();
    let tx = conway_tx(&cbor);
    let expected = vec![hex::decode("450101002499").unwrap()];

    assert_eq!(v3_scripts_alpha(&tx), expected);
    assert_eq!(v3_scripts_beta(&tx), expected);
}

#[test]
fn a_conway_transaction_without_a_v3_witness_script_maps_no_v3_script() {
    let cbor = hex::decode(include_str!("../../test_data/conway2.tx")).unwrap();
    let tx = conway_tx(&cbor);

    assert!(v3_scripts_alpha(&tx).is_empty());
    assert!(v3_scripts_beta(&tx).is_empty());
}
