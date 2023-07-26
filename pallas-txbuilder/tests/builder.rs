use pallas_txbuilder::prelude::*;

macro_rules! assert_transaction {
    ($code:expr) => {{
        let bytes = hex::decode($code).expect("Failed to decode transaction CBOR");
        let cbor: serde_cbor::Value =
            serde_cbor::from_slice(&bytes).expect("Failed to parse transaction CBOR");

        insta::assert_yaml_snapshot!(&cbor);
    }};
}

#[test]
fn test_build_simplest_transaction() {
    let input = Input::build([0; 32], 0);
    let resolved = Output::lovelaces(vec![], 1000000).build();
    let output = Output::lovelaces(vec![], 1000000).build();

    let tx = TransactionBuilder::<Manual>::new(NetworkParams::mainnet())
        .input(input, resolved)
        .output(output)
        .build()
        .expect("Failed to create transaction")
        .hex_encoded()
        .expect("Failed to encode transaction to hex");

    assert_transaction!(tx);
}

#[test]
fn test_build_transaction_with_ttl() {
    let input = Input::build([0; 32], 0);
    let resolved = Output::lovelaces(vec![], 1000000).build();
    let output = Output::lovelaces(vec![], 1000000).build();

    let valid_until = 1618430000;

    let tx = TransactionBuilder::<Manual>::new(NetworkParams::mainnet())
        .input(input, resolved)
        .output(output)
        .valid_until(valid_until)
        .build()
        .expect("Failed to create transaction")
        .hex_encoded()
        .expect("Failed to encode transaction to hex");

    assert_transaction!(tx);
}

#[test]
fn test_build_transaction_with_valid_after() {
    let input = Input::build([0; 32], 0);
    let resolved = Output::lovelaces(vec![], 1000000).build();
    let output = Output::lovelaces(vec![], 1000000).build();

    let valid_after = 1618430000;

    let tx = TransactionBuilder::<Manual>::new(NetworkParams::mainnet())
        .input(input, resolved)
        .output(output)
        .valid_after(valid_after)
        .build()
        .expect("Failed to create transaction")
        .hex_encoded()
        .expect("Failed to encode transaction to hex");

    assert_transaction!(tx);
}

#[test]
fn test_build_multiasset_transaction() {
    let input = Input::build([0; 32], 0);

    let assets = MultiAsset::new()
        .add([0; 28].into(), "MyAsset", 1000000)
        .expect("Failed to create asset");

    let resolved = Output::multiasset(vec![], 1000000, assets.clone()).build();
    let output = Output::multiasset(vec![], 1000000, assets).build();

    let tx = TransactionBuilder::<Manual>::new(NetworkParams::mainnet())
        .input(input, resolved)
        .output(output)
        .build()
        .expect("Failed to create transaction")
        .hex_encoded()
        .expect("Failed to encode transaction to hex");

    assert_transaction!(tx);
}

#[test]
fn test_build_mint() {
    let input = Input::build([0; 32], 0);
    let resolved = Output::lovelaces(vec![], 1000000).build();
    let output = Output::lovelaces(vec![], 1000000).build();

    let assets = MultiAsset::new()
        .add([0; 28].into(), "MyAsset 2", 1000000)
        .expect("Failed to create asset");

    let tx = TransactionBuilder::<Manual>::new(NetworkParams::mainnet())
        .input(input, resolved)
        .output(output)
        .mint(assets)
        .build()
        .expect("Failed to create transaction")
        .hex_encoded()
        .expect("Failed to encode transaction to hex");

    assert_transaction!(tx);
}
