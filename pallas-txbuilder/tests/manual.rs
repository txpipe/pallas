use pallas_txbuilder::prelude::*;

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

    let expected = "83a400818258200000000000000000000000000000000000000000000000000000000000000000000181a20040011a000f4240021a000272870f01a0f5";

    assert_eq!(tx, expected)
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

    let expected = "83a500818258200000000000000000000000000000000000000000000000000000000000000000000181a20040011a000f4240021a00027497031a01555a5d0f01a0f5";

    assert_eq!(tx, expected)
}

#[test]
fn test_build_transaction_with_jalid_after() {
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

    let expected = "83a500818258200000000000000000000000000000000000000000000000000000000000000000000181a20040011a000f4240021a00027497081a01555a5d0f01a0f5";

    assert_eq!(tx, expected)
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

    let expected =  "83a400818258200000000000000000000000000000000000000000000000000000000000000000000181a2004001821a000f4240a1581c00000000000000000000000000000000000000000000000000000000a1474d7941737365741a000f4240021a000282520f01a0f5";

    assert_eq!(tx, expected)
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

    let expected =  "83a500818258200000000000000000000000000000000000000000000000000000000000000000000181a20040011a000f4240021a0002830209a1581c00000000000000000000000000000000000000000000000000000000a1494d79417373657420321a000f42400f01a0f5";

    assert_eq!(tx, expected)
}
