use pallas_txbuilder::prelude::*;

#[test]
fn test_build_manual_simplest_transaction() {
    let input = Input::new([0; 32], 0);
    let resolved = Output::lovelaces(vec![], 1000000).build();
    let output = Output::lovelaces(vec![], 1000000).build();

    let tx = TransactionBuilder::<Manual>::new(NetworkParams::default())
        .input(input.build(), resolved)
        .output(output)
        .build()
        .expect("Failed to create transaction");

    let expected =  "83a300818258200000000000000000000000000000000000000000000000000000000000000000000181a20040011a000f42400200a0f5";

    assert_eq!(
        hex::encode(tx.encode_fragment().expect("encoding failed")),
        expected
    )
}
