use pallas_primitives::babbage::{PlutusV1Script, PlutusV2Script};
use pallas_txbuilder::prelude::*;

#[test]
fn test_build_v1_from_hex() -> Result<(), PlutusScriptError> {
    let data = "460100002224a3";
    let script = PlutusScript::v1().from_hex(&data)?;

    assert_eq!(script.build(), PlutusV1Script(hex::decode(&data)?.into()));

    Ok(())
}

#[test]
fn test_build_v2_from_hex() -> Result<(), PlutusScriptError> {
    let data = "460100002224a3";
    let script = PlutusScript::v2().from_hex(&data)?;

    assert_eq!(script.build(), PlutusV2Script(hex::decode(&data)?.into()));

    Ok(())
}

#[test]
fn test_build_v1_from_bytes() -> Result<(), PlutusScriptError> {
    let data = hex::decode("460100002224a3")?;
    let script = PlutusScript::v1().from_bytes(&*data);

    assert_eq!(script.build(), PlutusV1Script(data.into()));

    Ok(())
}

#[test]
fn test_build_v2_from_bytes() -> Result<(), PlutusScriptError> {
    let data = hex::decode("460100002224a3")?;
    let script = PlutusScript::v2().from_bytes(&*data);

    assert_eq!(script.build(), PlutusV2Script(data.into()));

    Ok(())
}
