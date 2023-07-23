use pallas_txbuilder::prelude::*;

#[test]
fn test_build_manual_simplest_transaction() {
    let input = Input::new([0; 32], 0);
    let resolved = Output::lovelaces(vec![], 1000000).build();
    let output = Output::lovelaces(vec![], 1000000).build();

    let tx = TransactionBuilder::<Manual>::new(NetworkParams::mainnet())
        .input(input.build(), resolved)
        .output(output)
        .build()
        .expect("Failed to create transaction")
        .hex_encoded()
        .expect("Failed to encode transaction to hex");

    let expected =  "83a300818258200000000000000000000000000000000000000000000000000000000000000000000181a20040011a000f4240021a000271d8a0f5";

    assert_eq!(tx, expected)
}

#[test]
fn test_build_manual_transaction_with_ttl() {
    let input = Input::new([0; 32], 0);
    let resolved = Output::lovelaces(vec![], 1000000).build();
    let output = Output::lovelaces(vec![], 1000000).build();

    let valid_until = 1618430000;

    let tx = TransactionBuilder::<Manual>::new(NetworkParams::mainnet())
        .input(input.build(), resolved)
        .output(output)
        .valid_until(valid_until)
        .build()
        .expect("Failed to create transaction")
        .hex_encoded()
        .expect("Failed to encode transaction to hex");

    let expected = "83a400818258200000000000000000000000000000000000000000000000000000000000000000000181a20040011a000f4240021a000273e7031a01555a5da0f5";

    assert_eq!(tx, expected)
}

#[test]
fn test_build_manual_transaction_with_valid_after() {
    let input = Input::new([0; 32], 0);
    let resolved = Output::lovelaces(vec![], 1000000).build();
    let output = Output::lovelaces(vec![], 1000000).build();

    let valid_after = 1618430000;

    let tx = TransactionBuilder::<Manual>::new(NetworkParams::mainnet())
        .input(input.build(), resolved)
        .output(output)
        .valid_after(valid_after)
        .build()
        .expect("Failed to create transaction")
        .hex_encoded()
        .expect("Failed to encode transaction to hex");

    let expected = "83a400818258200000000000000000000000000000000000000000000000000000000000000000000181a20040011a000f4240021a000273e7081a01555a5da0f5";

    assert_eq!(tx, expected)
}

#[test]
fn test_build_manual_multiasset_transaction() {
    let input = Input::new([0; 32], 0);

    let assets = MultiAsset::new(1000000)
        .add([0; 28].into(), "MyAsset", 1000000)
        .expect("Failed to create asset");

    let resolved = Output::multiasset(vec![], assets.clone()).build();
    let output = Output::multiasset(vec![], assets).build();

    let tx = TransactionBuilder::<Manual>::new(NetworkParams::mainnet())
        .input(input.build(), resolved)
        .output(output)
        .build()
        .expect("Failed to create transaction")
        .hex_encoded()
        .expect("Failed to encode transaction to hex");

    let expected =  "83a300818258200000000000000000000000000000000000000000000000000000000000000000000181a2004001821a000f4240a1581c00000000000000000000000000000000000000000000000000000000a1474d7941737365741a000f4240021a000281a3a0f5";

    assert_eq!(tx, expected)
}
