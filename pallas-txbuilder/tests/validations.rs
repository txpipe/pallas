use pallas_txbuilder::prelude::*;

#[test]
fn test_transaction_building_fails_without_inputs() -> Result<(), ValidationError> {
    match TransactionBuilder::new(NetworkParams::mainnet()).build() {
        Ok(_) => panic!("Transaction should be invalid without inputs"),
        Err(e) => assert_eq!(e, ValidationError::NoInputs),
    }

    Ok(())
}

#[test]
fn test_transaction_building_fails_without_outputs() -> Result<(), ValidationError> {
    let input = Input::build([0; 32], 0);
    let resolved = Output::lovelaces(vec![], 1000000).build();

    match TransactionBuilder::new(NetworkParams::mainnet())
        .input(input, resolved)
        .build()
    {
        Ok(_) => panic!("Transaction should be invalid without outputs"),
        Err(e) => assert_eq!(e, ValidationError::NoOutputs),
    }

    Ok(())
}

#[test]
fn test_transaction_building_fails_with_multiasset_collateral_returns(
) -> Result<(), ValidationError> {
    let input = Input::build([0; 32], 0);
    let resolved = Output::lovelaces(vec![], 1000000).build();
    let output = Output::lovelaces(vec![], 1000000).build();

    let assets = MultiAsset::new().add([0; 28].into(), "MyAsset", 1000000)?;
    let collateral_return = Output::multiasset(vec![], 1000000, assets).build();

    match TransactionBuilder::new(NetworkParams::mainnet())
        .input(input, resolved)
        .output(output)
        .collateral_return(collateral_return)
        .build()
    {
        Ok(_) => panic!("Transaction should be invalid without outputs"),
        Err(e) => assert_eq!(e, ValidationError::InvalidCollateral),
    }

    Ok(())
}
