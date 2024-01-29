pub mod common;

use common::*;
use pallas_applying::{
    utils::{BabbageProtParams, Environment, MultiEraProtParams},
    validate, UTxOs,
};
use pallas_codec::utils::CborWrap;
use pallas_primitives::babbage::{MintedDatumOption, MintedScriptRef, MintedTx, Value};
use pallas_traverse::MultiEraTx;

#[cfg(test)]
mod babbage_tests {
    use super::*;

    #[test]
    // Transaction hash:
    // b17d685c42e714238c1fb3abcd40e5c6291ebbb420c9c69b641209607bd00c7d
    fn successful_mainnet_tx() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/babbage3.tx"));
        let mtx: MintedTx = babbage_minted_tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_babbage(&mtx);
        let tx_outs_info: &[(
            String,
            Value,
            Option<MintedDatumOption>,
            Option<CborWrap<MintedScriptRef>>,
        )] = &[(
            String::from(include_str!("../../test_data/babbage3.address")),
            Value::Coin(103324335),
            None,
            None,
        )];
        let utxos: UTxOs = mk_utxo_for_babbage_tx(&mtx.transaction_body, tx_outs_info);
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Babbage(BabbageProtParams {}),
            prot_magic: 764824073,
            block_slot: 72316896,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => (),
            Err(err) => assert!(false, "Unexpected error ({:?})", err),
        }
    }
}
