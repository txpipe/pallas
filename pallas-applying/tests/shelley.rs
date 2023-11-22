use std::borrow::Cow;

use pallas_addresses::{Address, Network, ShelleyAddress};
use pallas_applying::{
    types::{Environment, MultiEraProtParams, ShelleyProtParams, ValidationError},
    validate, UTxOs,
};
use pallas_codec::{
    minicbor::{
        decode::{Decode, Decoder},
        encode,
    },
    utils::Bytes,
};
use pallas_crypto::hash::Hash;
use pallas_primitives::alonzo::{
    MintedTx, TransactionBody, TransactionInput, TransactionOutput, Value,
};
use pallas_traverse::{Era, MultiEraInput, MultiEraOutput, MultiEraTx};

#[cfg(test)]
mod shelley_tests {
    use super::*;

    fn cbor_to_bytes(input: &str) -> Vec<u8> {
        hex::decode(input).unwrap()
    }

    fn minted_tx_from_cbor<'a>(tx_cbor: &'a Vec<u8>) -> MintedTx<'a> {
        pallas_codec::minicbor::decode::<MintedTx>(&tx_cbor[..]).unwrap()
    }

    // Careful: this function assumes tx_body has exactly one input.
    fn mk_utxo_for_single_input_tx<'a>(
        tx_body: &TransactionBody,
        address: String,
        amount: Value,
        datum_hash: Option<Hash<32>>,
    ) -> UTxOs<'a> {
        let tx_ins: &Vec<TransactionInput> = &tx_body.inputs;
        assert_eq!(tx_ins.len(), 1, "Unexpected number of inputs.");
        let tx_in: TransactionInput = tx_ins.first().unwrap().clone();
        let address_bytes: Bytes = match hex::decode(address) {
            Ok(bytes_vec) => Bytes::from(bytes_vec),
            _ => panic!("Unable to decode input address."),
        };
        let tx_out: TransactionOutput = TransactionOutput {
            address: address_bytes,
            amount,
            datum_hash,
        };
        let mut utxos: UTxOs = UTxOs::new();
        add_to_utxo(&mut utxos, tx_in, tx_out);
        utxos
    }

    #[test]
    // Transaction hash: 50eba65e73c8c5f7b09f4ea28cf15dce169f3d1c322ca3deff03725f51518bb2
    fn successful_mainnet_tx() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/shelley1.tx"));
        let mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Shelley);
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Shelley(ShelleyProtParams),
            prot_magic: 764824073,
            block_slot: 5281340,
            network_id: 1,
        };
        let utxos: UTxOs = mk_utxo_for_single_input_tx(
            &mtx.transaction_body,
            String::from(include_str!("../../test_data/shelley1.address")),
            Value::Coin(2332267427205),
            None,
        );
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
        let utxos: UTxOs = mk_utxo_for_single_input_tx(
            &mtx.transaction_body,
            String::from(include_str!("../../test_data/shelley1.address")),
            Value::Coin(2332267427205),
            None,
        );
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
            block_slot: 5281340,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => assert!(false, "Inputs set should not be empty."),
            Err(err) => match err {
                ValidationError::TxInsEmpty => (),
                _ => assert!(false, "Unexpected error ({:?}).", err),
            },
        }
    }

    #[test]
    // The transaction is valid, but the UTxO set is empty.
    fn unfound_utxo() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/shelley1.tx"));
        let mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Shelley);
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Shelley(ShelleyProtParams),
            prot_magic: 764824073,
            block_slot: 5281340,
            network_id: 1,
        };
        let utxos: UTxOs = UTxOs::new();
        match validate(&metx, &utxos, &env) {
            Ok(()) => assert!(false, "All inputs must be within the UTxO set."),
            Err(err) => match err {
                ValidationError::InputMissingInUTxO => (),
                _ => assert!(false, "Unexpected error ({:?}).", err),
            },
        }
    }

    #[test]
    // Time-to-live is removed.
    fn missing_ttl() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/shelley1.tx"));
        let mut mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let utxos: UTxOs = mk_utxo_for_single_input_tx(
            &mtx.transaction_body,
            String::from(include_str!("../../test_data/shelley1.address")),
            Value::Coin(2332267427205),
            None,
        );
        let mut tx_body: TransactionBody = (*mtx.transaction_body).clone();
        tx_body.ttl = None;
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
            block_slot: 5281340,
            network_id: 1,
        };
        match validate(&metx, &utxos, &env) {
            Ok(()) => assert!(false, "TTL must always be present in Shelley transactions."),
            Err(err) => match err {
                ValidationError::AlonzoCompatibleNotShelley => (),
                _ => assert!(false, "Unexpected error ({:?}).", err),
            },
        }
    }

    #[test]
    // Block slot is after transaction's time-to-live.
    fn ttl_exceeded() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/shelley1.tx"));
        let mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_alonzo_compatible(&mtx, Era::Shelley);
        let env: Environment = Environment {
            prot_params: MultiEraProtParams::Shelley(ShelleyProtParams),
            prot_magic: 764824073,
            block_slot: 9999999,
            network_id: 1,
        };
        let utxos: UTxOs = mk_utxo_for_single_input_tx(
            &mtx.transaction_body,
            String::from(include_str!("../../test_data/shelley1.address")),
            Value::Coin(2332267427205),
            None,
        );
        match validate(&metx, &utxos, &env) {
            Ok(()) => assert!(false, "TTL cannot be exceeded."),
            Err(err) => match err {
                ValidationError::TTLExceeded => (),
                _ => assert!(false, "Unexpected error ({:?}).", err),
            },
        }
    }

    #[test]
    // One of the output's address network ID is changed from the mainnet value to the testnet one.
    fn wrong_network_id() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/shelley1.tx"));
        let mut mtx: MintedTx = minted_tx_from_cbor(&cbor_bytes);
        // Modify the first output address.
        let mut tx_body: TransactionBody = (*mtx.transaction_body).clone();
        let (first_output, rest): (&TransactionOutput, &[TransactionOutput]) =
            (&tx_body.outputs).split_first().unwrap();

        let addr: ShelleyAddress =
            match Address::from_bytes(&Vec::<u8>::from(first_output.address.clone())) {
                Ok(Address::Shelley(sa)) => sa,
                Ok(_) => panic!("Decoded output address and found the wrong era."),
                Err(e) => panic!("Unable to parse output address ({:?})", e),
            };
        let altered_address: ShelleyAddress = ShelleyAddress::new(
            Network::Testnet,
            addr.payment().clone(),
            addr.delegation().clone(),
        );
        let altered_output: TransactionOutput = TransactionOutput {
            address: Bytes::from(altered_address.to_vec()),
            amount: first_output.amount.clone(),
            datum_hash: first_output.datum_hash,
        };
        let mut new_outputs = Vec::from(rest);
        new_outputs.insert(0, altered_output);
        tx_body.outputs = new_outputs;

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
            block_slot: 5281340,
            network_id: 1,
        };
        let utxos: UTxOs = mk_utxo_for_single_input_tx(
            &mtx.transaction_body,
            String::from(include_str!("../../test_data/shelley1.address")),
            Value::Coin(2332267427205),
            None,
        );
        match validate(&metx, &utxos, &env) {
            Ok(()) => assert!(false, "Output with wrong network ID should be rejected."),
            Err(err) => match err {
                ValidationError::WrongNetworkID => (),
                _ => assert!(false, "Unexpected error ({:?}).", err),
            },
        }
    }
}

// Helper functions.
fn add_to_utxo<'a>(utxos: &mut UTxOs<'a>, tx_in: TransactionInput, tx_out: TransactionOutput) {
    let multi_era_in: MultiEraInput = MultiEraInput::AlonzoCompatible(Box::new(Cow::Owned(tx_in)));
    let multi_era_out: MultiEraOutput =
        MultiEraOutput::AlonzoCompatible(Box::new(Cow::Owned(tx_out)));
    utxos.insert(multi_era_in, multi_era_out);
}
