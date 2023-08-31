use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use pallas_txbuilder::prelude::*;

fn unix_epoch() -> Instant {
    let instant = Instant::now();

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|d| instant.checked_sub(d))
        .unwrap()
}

fn beginning_of_2023() -> Instant {
    let start = Duration::new(1672531200, 0);
    unix_epoch() + start
}

macro_rules! assert_transaction {
    ($code:expr) => {{
        let bytes = hex::decode($code).expect("Failed to decode transaction CBOR");
        let cbor: serde_cbor::Value =
            serde_cbor::from_slice(&bytes).expect("Failed to parse transaction CBOR");

        insta::assert_yaml_snapshot!(&cbor);

        Ok(())
    }};
}

#[test]
fn test_build_simplest_transaction() -> Result<(), ValidationError> {
    let input = Input::build([0; 32], 0);
    let resolved = Output::lovelaces(vec![], 1000000).build();
    let output = Output::lovelaces(vec![], 1000000).build();

    let tx = TransactionBuilder::new(NetworkParams::mainnet())
        .input(input, resolved)
        .output(output)
        .build()?
        .hex_encoded()?;

    assert_transaction!(tx)
}

#[test]
fn test_build_transaction_with_multiple_inputs() -> Result<(), ValidationError> {
    let input_a = Input::build([0; 32], 0);
    let resolved_a = Output::lovelaces(vec![], 1000000).build();

    let input_b = Input::build([0; 32], 1);
    let resolved_b = Output::lovelaces(vec![], 1000001).build();

    let output = Output::lovelaces(vec![], 1000000).build();

    let tx = TransactionBuilder::new(NetworkParams::mainnet())
        .input(input_a, resolved_a)
        .input(input_b, resolved_b)
        .output(output)
        .build()?
        .hex_encoded()?;

    assert_transaction!(tx)
}

#[test]
fn test_build_transaction_with_multiple_outputs() -> Result<(), ValidationError> {
    let input = Input::build([0; 32], 0);
    let resolved = Output::lovelaces(vec![], 1000000).build();

    let output_a = Output::lovelaces(vec![], 499999).build();
    let output_b = Output::lovelaces(vec![], 500001).build();

    let tx = TransactionBuilder::new(NetworkParams::mainnet())
        .input(input, resolved)
        .output(output_a)
        .output(output_b)
        .build()?
        .hex_encoded()?;

    assert_transaction!(tx)
}

#[test]
fn test_build_transaction_with_ttl() -> Result<(), ValidationError> {
    let input = Input::build([0; 32], 0);
    let resolved = Output::lovelaces(vec![], 1000000).build();
    let output = Output::lovelaces(vec![], 1000000).build();

    let slot = 101938047;

    let tx = TransactionBuilder::new(NetworkParams::mainnet())
        .input(input, resolved)
        .output(output)
        .valid_until_slot(slot)
        .build()?
        .hex_encoded()?;

    assert_transaction!(tx)
}

#[test]
fn test_build_transaction_with_timestamp_ttl() -> Result<(), ValidationError> {
    let input = Input::build([0; 32], 0);
    let resolved = Output::lovelaces(vec![], 1000000).build();
    let output = Output::lovelaces(vec![], 1000000).build();

    let valid_until = beginning_of_2023();

    let tx = TransactionBuilder::new(NetworkParams::mainnet())
        .input(input, resolved)
        .output(output)
        .valid_until(valid_until)?
        .build()?
        .hex_encoded()?;

    assert_transaction!(tx)
}

#[test]
fn test_build_transaction_with_valid_after() -> Result<(), ValidationError> {
    let input = Input::build([0; 32], 0);
    let resolved = Output::lovelaces(vec![], 1000000).build();
    let output = Output::lovelaces(vec![], 1000000).build();

    let slot = 101938047;

    let tx = TransactionBuilder::new(NetworkParams::mainnet())
        .input(input, resolved)
        .output(output)
        .valid_after_slot(slot)
        .build()?
        .hex_encoded()?;

    assert_transaction!(tx)
}

#[test]
fn test_build_transaction_with_timestamp_valid_after() -> Result<(), ValidationError> {
    let input = Input::build([0; 32], 0);
    let resolved = Output::lovelaces(vec![], 1000000).build();
    let output = Output::lovelaces(vec![], 1000000).build();

    let valid_after = beginning_of_2023();

    let tx = TransactionBuilder::new(NetworkParams::mainnet())
        .input(input, resolved)
        .output(output)
        .valid_after(valid_after)?
        .build()?
        .hex_encoded()?;

    assert_transaction!(tx)
}

#[test]
fn test_build_multiasset_transaction() -> Result<(), ValidationError> {
    let input = Input::build([0; 32], 0);

    let assets = MultiAsset::new().add([0; 28].into(), "MyAsset", 1000000)?;

    let resolved = Output::multiasset(vec![], 1000000, assets.clone()).build();
    let output = Output::multiasset(vec![], 1000000, assets).build();

    let tx = TransactionBuilder::new(NetworkParams::mainnet())
        .input(input, resolved)
        .output(output)
        .build()?
        .hex_encoded()?;

    assert_transaction!(tx)
}

#[test]
fn test_build_mint() -> Result<(), ValidationError> {
    let input = Input::build([0; 32], 0);
    let resolved = Output::lovelaces(vec![], 1000000).build();
    let output = Output::lovelaces(vec![], 1000000).build();

    let assets = MultiAsset::new().add([0; 28].into(), "MyAsset 2", 1000000)?;

    let tx = TransactionBuilder::new(NetworkParams::mainnet())
        .input(input, resolved)
        .output(output)
        .mint(assets)
        .build()?
        .hex_encoded()?;

    assert_transaction!(tx)
}

#[test]
fn test_build_with_reference_inputs() -> Result<(), ValidationError> {
    let input = Input::build([0; 32], 0);
    let resolved = Output::lovelaces(vec![], 1000000).build();
    let output = Output::lovelaces(vec![], 1000000).build();

    let tx = TransactionBuilder::new(NetworkParams::mainnet())
        .input(input.clone(), resolved)
        .output(output)
        .reference_input(input)
        .build()?
        .hex_encoded()?;

    assert_transaction!(tx)
}

#[test]
fn test_build_with_collateral_inputs() -> Result<(), ValidationError> {
    let input = Input::build([0; 32], 0);
    let resolved = Output::lovelaces(vec![], 1000000).build();
    let output = Output::lovelaces(vec![], 999998).build();

    let collateral = Input::build([0; 32], 1);
    let collateral_return = Output::lovelaces(vec![], 2).build();

    let tx = TransactionBuilder::new(NetworkParams::mainnet())
        .input(input.clone(), resolved)
        .output(output)
        .collateral(collateral)
        .collateral_return(collateral_return)
        .build()?
        .hex_encoded()?;

    assert_transaction!(tx)
}

#[test]
fn test_build_with_plutus_data() -> Result<(), ValidationError> {
    use plutus::*;

    let input = Input::build([0; 32], 0);
    let resolved = Output::lovelaces(vec![], 1000000).build();
    let output = Output::lovelaces(vec![], 1000000).build();

    let data = map().item(int(1), int(2));

    let tx = TransactionBuilder::new(NetworkParams::mainnet())
        .input(input, resolved)
        .output(output)
        .plutus_data(data)
        .build()?
        .hex_encoded()?;

    assert_transaction!(tx)
}
