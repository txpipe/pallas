use pallas_primitives::babbage::{
    PseudoPostAlonzoTransactionOutput, TransactionInput, TransactionOutput, Value,
};
use pallas_txbuilder::prelude::*;

#[test]
fn build_basic() {
    let input = TransactionInput {
        transaction_id: [0; 32].into(),
        index: 0,
    };

    let resolved = TransactionOutput::PostAlonzo(PseudoPostAlonzoTransactionOutput {
        address: vec![].into(),
        value: Value::Coin(1000000),
        datum_option: None,
        script_ref: None,
    });

    let output = TransactionOutput::PostAlonzo(PseudoPostAlonzoTransactionOutput {
        address: vec![].into(),
        value: Value::Coin(1000000),
        datum_option: None,
        script_ref: None,
    });

    let tx = TransactionBuilder::<Manual>::new(NetworkParams::default())
        .input(input, resolved)
        .output(output)
        .build()
        .unwrap();

    let bytes = tx.encode_fragment().expect("encoding failed");

    assert_eq!(hex::encode(bytes), "83a300818258200000000000000000000000000000000000000000000000000000000000000000000181a20040011a000f42400200a0f5")
}
