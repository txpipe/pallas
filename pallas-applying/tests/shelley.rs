use pallas_applying::{
    types::{Environment, MultiEraProtParams, ShelleyProtParams},
    validate, UTxOs,
};
use pallas_traverse::{Era, MultiEraTx};

#[cfg(test)]
mod byron_tests {
    use super::*;

    fn cbor_to_bytes(input: &str) -> Vec<u8> {
        hex::decode(input).unwrap()
    }

    fn tx_from_cbor<'a>(tx_cbor: &'a Vec<u8>) -> MultiEraTx<'a> {
        MultiEraTx::decode_for_era(Era::Shelley, &tx_cbor[..]).unwrap()
    }

    #[test]
    fn successful_mainnet_tx() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/shelley1.tx"));
        let metx: MultiEraTx = tx_from_cbor(&cbor_bytes);
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Shelley(ShelleyProtParams),
            prot_magic: 764824073,
        };
        let utxos: UTxOs = UTxOs::new();
        match validate(&metx, &utxos, &env) {
            Ok(()) => (),
            Err(err) => assert!(false, "Unexpected error ({:?}).", err),
        }
    }
}
