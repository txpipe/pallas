use assert_matches::assert_matches;

use pallas_txbuilder::prelude::*;

#[test]
fn test_transaction_building_fails_without_inputs() {
    assert_matches!(
        TransactionBuilder::new(NetworkParams::mainnet()).build(),
        Err(ValidationError::NoInputs)
    );
}

#[test]
fn test_transaction_building_fails_without_outputs() {
    let input = Input::build([0; 32], 0);
    let resolved = Output::lovelaces(vec![], 1000000).build();

    assert_matches!(
        TransactionBuilder::new(NetworkParams::mainnet())
            .input(input, resolved)
            .build(),
        Err(ValidationError::NoOutputs)
    );
}

#[test]
fn test_transaction_building_fails_with_multiasset_collateral_return() -> Result<(), ValidationError>
{
    let input = Input::build([0; 32], 0);
    let resolved = Output::lovelaces(vec![], 1000000).build();
    let output = Output::lovelaces(vec![], 1000000).build();

    let assets = MultiAsset::new().add([0; 28].into(), "MyAsset", 1000000)?;
    let collateral_return = Output::multiasset(vec![], 1000000, assets).build();

    let tx = TransactionBuilder::new(NetworkParams::mainnet())
        .input(input.clone(), resolved)
        .output(output)
        .collateral_return(collateral_return)
        .build();

    assert_matches!(tx, Err(ValidationError::InvalidCollateralReturn));

    Ok(())
}

#[test]
fn test_transaction_building_fails_with_multiasset_collateral_input() -> Result<(), ValidationError>
{
    let input = Input::build([0; 32], 0);
    let assets = MultiAsset::new().add([0; 28].into(), "MyAsset", 1000000)?;
    let resolved = Output::multiasset(vec![], 1000000, assets).build();
    let output = Output::lovelaces(vec![], 1000000).build();

    let tx = TransactionBuilder::new(NetworkParams::mainnet())
        .input(input.clone(), resolved)
        .output(output)
        .collateral(input)
        .build();

    assert_matches!(tx, Err(ValidationError::InvalidCollateralInput));

    Ok(())
}
