use pallas_txbuilder::prelude::*;

#[test]
fn test_transaction_building_fails_without_inputs() {
    match TransactionBuilder::<Manual>::new(NetworkParams::mainnet()).build() {
        Ok(_) => panic!("Transaction should be invalid without inputs"),
        Err(e) => assert_eq!(e, ValidationError::NoInputs),
    }
}

#[test]
fn test_transaction_building_fails_without_outputs() {
    let input = Input::new([0; 32], 0).build();
    let resolved = Output::lovelaces(vec![], 1000000).build();

    match TransactionBuilder::<Manual>::new(NetworkParams::mainnet())
        .input(input, resolved)
        .build()
    {
        Ok(_) => panic!("Transaction should be invalid without outputs"),
        Err(e) => assert_eq!(e, ValidationError::NoOutputs),
    }
}
