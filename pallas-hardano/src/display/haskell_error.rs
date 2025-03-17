use pallas_network::miniprotocols::localtxsubmission::{
    ApplyTxError, ConwayLedgerFailure, ShelleyBasedEra, TxValidationError,
};
use serde::{Serialize, Serializer};

use super::haskell_display::HaskellDisplay;

/// Mimicks the json data structure of the error response from the cardano-submit-api
pub fn wrap_error_response(error: TxValidationError) -> TxSubmitFail {
    TxSubmitFail::TxSubmitFail(TxCmdError::TxCmdTxSubmitValidationError(
        TxValidationErrorInCardanoMode::TxValidationErrorInCardanoMode(error),
    ))
}

/// Generates Haskell 'identical' string for the error response
pub fn as_node_submit_error(error: TxValidationError) -> String {
    serde_json::to_string(&wrap_error_response(error)).unwrap()
}

/// Generates Haskell 'similar' string for the error response in case of decode failure
/// Only difference will be the provided decode failure message, Rust vs Haskell
pub fn as_cbor_decode_failure(message: String, position: u64) -> String {
    let inner_errors = vec![DecoderError::DeserialiseFailure(
        "Shelley Tx".to_string(),
        DeserialiseFailure(position, message),
    )];
    let error = TxSubmitFail::TxSubmitFail(TxCmdError::TxReadError(inner_errors));
    serde_json::to_string(&error).unwrap()
}

pub fn serialize_error(error: TxValidationError) -> serde_json::Value {
    serde_json::to_value(wrap_error_response(error)).unwrap()
}

/// https://github.com/IntersectMBO/cardano-node/blob/9dbf0b141e67ec2dfd677c77c63b1673cf9c5f3e/cardano-submit-api/src/Cardano/TxSubmit/Types.hs#L54
#[derive(Debug, Serialize)]
#[serde(tag = "tag", content = "contents")]
pub enum TxSubmitFail {
    TxSubmitDecodeHex,
    TxSubmitEmpty,
    TxSubmitDecodeFail(DecoderError),
    TxSubmitBadTx(String),
    TxSubmitFail(TxCmdError),
}

// https://github.com/IntersectMBO/cardano-node/blob/9dbf0b141e67ec2dfd677c77c63b1673cf9c5f3e/cardano-submit-api/src/Cardano/TxSubmit/Types.hs#L92
#[derive(Debug, Serialize)]
#[serde(tag = "tag", content = "contents")]
pub enum TxCmdError {
    SocketEnvError(String),
    #[serde(serialize_with = "use_haskell_display", rename = "TxCmdTxReadError")]
    TxReadError(Vec<DecoderError>),
    TxCmdTxSubmitValidationError(TxValidationErrorInCardanoMode),
}

/// https://github.com/IntersectMBO/cardano-api/blob/d7c62a04ebf18d194a6ea70e6765eb7691d57668/cardano-api/internal/Cardano/Api/InMode.hs#L259
#[derive(Debug, Serialize)]
#[serde(tag = "tag", content = "contents")]
pub enum TxValidationErrorInCardanoMode {
    #[serde(with = "TxValidationErrorJson")]
    TxValidationErrorInCardanoMode(TxValidationError),
    EraMismatch(EraMismatch),
}

/// https://github.com/IntersectMBO/ouroboros-consensus/blob/e86b921443bd6e8ea25e7190eb7cb5788e28f4cc/ouroboros-consensus/src/ouroboros-consensus/Ouroboros/Consensus/HardFork/Combinator/AcrossEras.hs#L208
#[derive(Debug, Serialize)]
pub struct EraMismatch {
    ledger: String, //  Name of the era of the ledger ("Byron" or "Shelley").
    other: String,  // Era of the block, header, transaction, or query.
}

/// https://github.com/IntersectMBO/cardano-base/blob/391a2c5cfd30d2234097e000dbd8d9db21ef94d7/cardano-binary/src/Cardano/Binary/FromCBOR.hs#L90
#[derive(Debug, Serialize)]
pub enum DecoderError {
    CanonicityViolation(String),
    Custom(String, String),
    DeserialiseFailure(String, DeserialiseFailure),
    EmptyList(String),
    Leftover(String, Vec<u8>),
    SizeMismatch(String, u64, u64),
    UnknownTag(String, u8),
    Void,
}
/// https://hackage.haskell.org/package/serialise-0.2.6.1/docs/Codec-Serialise.html#t:DeserialiseFailure
#[derive(Debug, Serialize)]
pub struct DeserialiseFailure(pub u64, pub String);

//
// Haskell JSON serializations
//

/// This is copy of TxValidationError from pallas-network/src/miniprotocols/localtxsubmission/primitives.rs for Haskell json serialization
#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
#[serde(remote = "TxValidationError", tag = "kind")]
enum TxValidationErrorJson {
    ByronTxValidationError {
        #[serde(with = "ApplyTxErrorJson")]
        error: ApplyTxError,
    },
    ShelleyTxValidationError {
        #[serde(with = "ApplyTxErrorJson")]
        error: ApplyTxError,
        #[serde(with = "ShelleyBasedEraJson")]
        era: ShelleyBasedEra,
    },
    Plutus(String),
}

/// This is copy of ApplyTxError from pallas-network/src/miniprotocols/localtxsubmission/primitives.rs for Haskell json serialization
#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
#[serde(remote = "ApplyTxError")]
struct ApplyTxErrorJson(
    #[serde(serialize_with = "use_haskell_display")] pub Vec<ConwayLedgerFailure>,
);

/// This is copy of ShelleyBasedEra from pallas-network/src/miniprotocols/localtxsubmission/primitives.rs for Haskell json serialization
#[derive(Debug, Serialize, PartialEq)]
#[serde(remote = "ShelleyBasedEra")]
enum ShelleyBasedEraJson {
    #[serde(rename = "ShelleyBasedEraShelley")]
    Shelley,
    #[serde(rename = "ShelleyBasedEraAllegra")]
    Allegra,
    #[serde(rename = "ShelleyBasedEraMary")]
    Mary,
    #[serde(rename = "ShelleyBasedEraAlonzo")]
    Alonzo,
    #[serde(rename = "ShelleyBasedEraBabbage")]
    Babbage,
    #[serde(rename = "ShelleyBasedEraConway")]
    Conway,
}

fn use_haskell_display<S, T>(fails: &[T], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    T: HaskellDisplay,
{
    let fails_str = fails.iter().map(|fail| fail.to_haskell_str());
    serializer.collect_seq(fails_str)
}

#[test]
#[allow(non_snake_case)]
/// Test the serialization, so it's identical to Cardano node's submit api response
fn test_submit_api_serialization() {
    let error = decode_error("81820681820764f0aab883");

    assert_eq!("{\"tag\":\"TxSubmitFail\",\"contents\":{\"tag\":\"TxCmdTxSubmitValidationError\",\"contents\":{\"tag\":\"TxValidationErrorInCardanoMode\",\"contents\":{\"kind\":\"ShelleyTxValidationError\",\"error\":[\"ConwayMempoolFailure \\\"\\\\175619\\\"\"],\"era\":\"ShelleyBasedEraConway\"}}}}", 
    as_node_submit_error(error));
}

#[test]
#[allow(non_snake_case)]
fn test_submit_api_decode_failure() {
    assert_eq!( "{\"tag\":\"TxSubmitFail\",\"contents\":{\"tag\":\"TxCmdTxReadError\",\"contents\":[\"DecoderErrorDeserialiseFailure \\\"Shelley Tx\\\" (DeserialiseFailure 0 (\\\"expected list len or indef\\\"))\"]}}",   
      as_cbor_decode_failure("expected list len or indef".to_string(), 0));
}

#[cfg(test)]
fn decode_error(cbor: &str) -> TxValidationError {
    use pallas_codec::minicbor;

    let bytes = hex::decode(cbor).unwrap();
    let mut decoder = minicbor::Decoder::new(&bytes);
    decoder.decode().unwrap()
}
