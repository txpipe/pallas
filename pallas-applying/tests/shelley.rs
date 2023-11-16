use pallas_applying::{
    types::{Environment, MultiEraProtParams, ShelleyProtParams, ValidationError},
    validate, UTxOs,
};
use pallas_codec::minicbor::{
    decode::{Decode, Decoder},
    encode,
};
use pallas_primitives::alonzo::{MintedTx, TransactionBody};
use pallas_traverse::{Era, MultiEraTx};

#[cfg(test)]
mod byron_tests {
    use super::*;

    fn cbor_to_bytes(input: &str) -> Vec<u8> {
        hex::decode(input).unwrap()
    }

    fn minted_tx_from_cbor<'a>(tx_cbor: &'a Vec<u8>) -> MintedTx<'a> {
        pallas_codec::minicbor::decode::<MintedTx>(&tx_cbor[..]).unwrap()
    }

    #[test]
    fn successful_mainnet_tx() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/shelley1.tx"));
        let mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Shelley);
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

    #[test]
    // Identical to sucessful_mainnet_tx, except that all inputs are removed.
    fn empty_ins() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/shelley1.tx"));
        let mut mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        // Clear the set of inputs in the transaction.
        let mut tx_body: TransactionBody = (*mtx.transaction_body).clone();
        tx_body.inputs = Vec::new();
        let mut tx_buf: Vec<u8> = Vec::new();
        match encode(tx_body, &mut tx_buf) {
            Ok(_) => (),
            Err(err) => assert!(false, "Unable to encode Tx ({:?}).", err),
        };
        mtx.transaction_body =
            Decode::decode(&mut Decoder::new(&tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Shelley);
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Shelley(ShelleyProtParams),
            prot_magic: 764824073,
        };
        let utxos: UTxOs = UTxOs::new();
        match validate(&metx, &utxos, &env) {
            Ok(()) => assert!(false, "Inputs set should not be empty."),
            Err(err) => match err {
                ValidationError::TxInsEmpty => (),
                _ => assert!(false, "Unexpected error ({:?}).", err),
            },
        }
    }
}
