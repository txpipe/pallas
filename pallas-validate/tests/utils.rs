use std::collections::BTreeMap;

use pallas_primitives::{
    AssetName, PolicyId,
    alonzo::{Multiasset, Value},
    conway::Value as ConwayValue,
};
use pallas_validate::utils::{
    PostAlonzoError, ValidationError, add_minted_value, add_values, conway_add_values,
};

fn single_asset<A>(amount: A) -> Multiasset<A> {
    let policy = PolicyId::from([0u8; 28]);
    let asset_name = AssetName::from(b"asset".to_vec());

    let mut assets = BTreeMap::new();
    assets.insert(asset_name, amount);

    let mut multiasset = BTreeMap::new();
    multiasset.insert(policy, assets);
    multiasset
}

#[test]
fn add_minted_value_rejects_burn_of_missing_asset() {
    let err = ValidationError::PostAlonzo(PostAlonzoError::NegativeValue);

    let result = add_minted_value(&Value::Coin(0), &single_asset(-1), &err);

    assert!(matches!(
        result,
        Err(ValidationError::PostAlonzo(PostAlonzoError::NegativeValue))
    ));
}

#[test]
fn add_minted_value_rejects_burn_below_zero() {
    let err = ValidationError::PostAlonzo(PostAlonzoError::NegativeValue);
    let base = Value::Multiasset(0, single_asset(1));

    let result = add_minted_value(&base, &single_asset(-2), &err);

    assert!(matches!(
        result,
        Err(ValidationError::PostAlonzo(PostAlonzoError::NegativeValue))
    ));
}

#[test]
fn add_values_rejects_lovelace_overflow() {
    let err = ValidationError::PostAlonzo(PostAlonzoError::NegativeValue);

    let result = add_values(&Value::Coin(u64::MAX), &Value::Coin(1), &err);

    assert!(matches!(
        result,
        Err(ValidationError::PostAlonzo(PostAlonzoError::NegativeValue))
    ));
}

#[test]
fn conway_add_values_rejects_lovelace_overflow() {
    let err = ValidationError::PostAlonzo(PostAlonzoError::NegativeValue);

    let result = conway_add_values(&ConwayValue::Coin(u64::MAX), &ConwayValue::Coin(1), &err);

    assert!(matches!(
        result,
        Err(ValidationError::PostAlonzo(PostAlonzoError::NegativeValue))
    ));
}
