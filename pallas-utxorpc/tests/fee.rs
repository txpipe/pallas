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

fn byron_tx() -> Vec<u8> {
    hex::decode(include_str!("../../test_data/byron1.tx")).unwrap()
}

fn conway_tx() -> Vec<u8> {
    hex::decode(include_str!("../../test_data/conway9.tx")).unwrap()
}

#[test]
fn a_byron_transaction_maps_a_zero_fee_before_v1() {
    let cbor = byron_tx();
    let tx = MultiEraTx::decode_for_era(Era::Byron, &cbor).unwrap();

    assert_eq!(
        v1alpha::Mapper::new(NoLedger)
            .map_tx(&tx)
            .fee
            .and_then(|x| x.big_int),
        Some(v1alpha::spec::cardano::big_int::BigInt::Int(0))
    );
    assert_eq!(
        v1beta::Mapper::new(NoLedger)
            .map_tx(&tx)
            .fee
            .and_then(|x| x.big_int),
        Some(v1beta::spec::cardano::big_int::BigInt::Int(0))
    );
}

#[cfg(feature = "unstable")]
#[test]
fn a_byron_transaction_maps_no_fee_in_v1() {
    let cbor = byron_tx();
    let tx = MultiEraTx::decode_for_era(Era::Byron, &cbor).unwrap();

    assert_eq!(
        pallas_utxorpc::v1::Mapper::new(NoLedger).map_tx(&tx).fee,
        None
    );
}

#[test]
fn a_conway_transaction_maps_its_fee() {
    let cbor = conway_tx();
    let tx = MultiEraTx::decode_for_era(Era::Conway, &cbor).unwrap();
    let fee = tx.fee().unwrap() as i64;

    let alpha = v1alpha::Mapper::new(NoLedger).map_tx(&tx).fee;
    let beta = v1beta::Mapper::new(NoLedger).map_tx(&tx).fee;

    assert_eq!(
        alpha.and_then(|x| x.big_int),
        Some(v1alpha::spec::cardano::big_int::BigInt::Int(fee))
    );
    assert_eq!(
        beta.and_then(|x| x.big_int),
        Some(v1beta::spec::cardano::big_int::BigInt::Int(fee))
    );

    #[cfg(feature = "unstable")]
    assert_eq!(
        pallas_utxorpc::v1::Mapper::new(NoLedger)
            .map_tx(&tx)
            .fee
            .and_then(|x| x.big_int),
        Some(pallas_utxorpc::v1::spec::cardano::big_int::BigInt::Int(fee))
    );
}
