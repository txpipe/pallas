use std::collections::BTreeMap;

use pallas_primitives::{
    AssetName, PolicyId,
    alonzo::{Multiasset, VKeyWitness, Value},
    conway::Value as ConwayValue,
};
use pallas_validate::utils::{
    PostAlonzoError, ValidationError, add_minted_value, add_values, conway_add_values,
    verify_signature,
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

fn witness(key: &str, signature: &str) -> VKeyWitness {
    VKeyWitness {
        vkey: hex::decode(key).unwrap().into(),
        signature: hex::decode(signature).unwrap().into(),
    }
}

const RFC8032_KEY: &str = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
const STRICT_MESSAGE: &str = "70616c6c6173206564323535313920726570726f";

#[test]
fn verify_signature_accepts_rfc8032_witness() {
    let wit = witness(
        RFC8032_KEY,
        "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b",
    );

    assert!(verify_signature(&wit, &[]));
}

#[test]
fn verify_signature_rejects_identity_key_and_r_with_zero_s() {
    let wit = witness(
        "0100000000000000000000000000000000000000000000000000000000000000",
        "01000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
    );

    assert!(!verify_signature(
        &wit,
        &hex::decode(STRICT_MESSAGE).unwrap()
    ));
}

#[test]
fn verify_signature_rejects_identity_r() {
    let wit = witness(
        RFC8032_KEY,
        "010000000000000000000000000000000000000000000000000000000000000024a865c766f6ef9110dfc32bde1b8e44c55b90e7328a71aedcaf8e4f00748d05",
    );

    assert!(!verify_signature(
        &wit,
        &hex::decode(STRICT_MESSAGE).unwrap()
    ));
}

#[test]
fn verify_signature_rejects_small_order_key_with_sign_bit_set() {
    let wit = witness(
        "ecffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
        "a9d55260f765261eb9b84e106f665e00b867287a761990d7135963ee0a7d59dca5bb704786be79fc476f91d3f3f89b03984d8068dcf1bb7dfc6637b45450ac04",
    );
    let message =
        hex::decode("39a591f5321bbe07fd5a23dc2f39d025d74526615746727ceefd6e82ae65c06f").unwrap();

    assert!(!verify_signature(&wit, &message));
}

#[test]
fn verify_signature_rejects_key_off_the_curve() {
    let wit = witness(
        "0200000000000000000000000000000000000000000000000000000000000000",
        "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b",
    );

    assert!(!verify_signature(&wit, &[]));
}
