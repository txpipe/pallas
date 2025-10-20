pub mod common;

use common::*;
use pallas_codec::minicbor;
use pallas_codec::minicbor::{
    decode::{Decode, Decoder},
    encode,
};
use pallas_codec::utils::{Bytes, CborWrap, KeepRaw};
use pallas_primitives::conway::{
    CostModels, DatumOption, ExUnits, NetworkId, PlutusScript, RationalNumber, ScriptRef,
    TransactionBody, Tx, Value,
};
use pallas_primitives::{
    conway::{DRepVotingThresholds, PoolVotingThresholds, TransactionOutput},
    Set,
};
use pallas_traverse::MultiEraTx;

use pallas_validate::{
    phase1::validate_txs,
    utils::{
        AccountState, CertState, ConwayProtParams, Environment, MultiEraProtocolParameters,
        PostAlonzoError, UTxOs, ValidationError::*,
    },
};

#[cfg(test)]
mod conway_tests {
    use std::{borrow::Cow, collections::BTreeMap};

    use pallas_addresses::{Address, ShelleyAddress, ShelleyPaymentPart};
    use pallas_primitives::{conway::PostAlonzoTransactionOutput, PositiveCoin};
    use pallas_traverse::{ComputeHash, MultiEraInput, MultiEraOutput};

    use super::*;

    #[test]
    // Transaction hash:
    // 90bd64b133e327daecfa0cc60c26f3b96fc6f0285a6d96cc122819908b3aaf93
    fn successful_mainnet_tx() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/conway3.tx"));
        let mtx: Tx = conway_minted_tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_conway(&mtx);
        let tx_outs_info: &[(
            String,
            Value,
            Option<DatumOption>,
            Option<CborWrap<ScriptRef>>,
        )] = &[(
            String::from("015c5c318d01f729e205c95eb1b02d623dd10e78ea58f72d0c13f892b2e8904edc699e2f0ce7b72be7cec991df651a222e2ae9244eb5975cba"),
            Value::Coin(20000000),
            None,
            None,
        )];
        let utxos: UTxOs = mk_utxo_for_conway_tx(&mtx.transaction_body, tx_outs_info);
        let acnt = AccountState {
            treasury: 261_254_564_000_000,
            reserves: 0,
        };

        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Conway(mk_mainnet_params_epoch_365()),
            prot_magic: 764824073,
            block_slot: 137806612,
            network_id: 1,
            acnt: Some(acnt),
        };
        let mut cert_state: CertState = CertState::default();
        match validate_txs(&[metx], &env, &utxos, &mut cert_state) {
            Ok(()) => (),
            Err(err) => panic!("Unexpected error ({err:?})"),
        }
    }

    #[test]
    //Transaction hash:
    // b41ebebf5234b645f9b0767ac541e1d9ea680b763d9b105554ef3b41acdbd36f
    fn successful_preview_tx_with_plutus_v3_script() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/conway4.tx"));
        let mtx: Tx = conway_minted_tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_conway(&mtx);
        dbg!(&metx.hash());
        let datum_bytes = cbor_to_bytes("d8799f4568656c6c6fff");
        let datum_option = DatumOption::Data(CborWrap(minicbor::decode(&datum_bytes).unwrap()));
        let datum_option = minicbor::to_vec(datum_option).unwrap();
        let datum_option: KeepRaw<'_, DatumOption> = minicbor::decode(&datum_option).unwrap();

        let mut tx_outs_info: Vec<(
            String,
            Value,
            Option<KeepRaw<'_, DatumOption>>,
            Option<CborWrap<ScriptRef>>,
            Vec<u8>,
        )> = vec![
            (
                String::from("005c5c318d01f729e205c95eb1b02d623dd10e78ea58f72d0c13f892b2e8904edc699e2f0ce7b72be7cec991df651a222e2ae9244eb5975cba"),
                Value::Coin(2554710123),
                None,
                None,
                Vec::new(),
            ),
            (
                String::from("70faae60072c45d121b6e58ae35c624693ee3dad9ea8ed765eb6f76f9f"),
                Value::Coin(100270605),
                Some(datum_option),
                None,
                Vec::new(),
            ),
        ];

        let mut utxos: UTxOs =
            mk_codec_safe_utxo_for_conway_tx(&mtx.transaction_body, &mut tx_outs_info);

        let mut ref_info: Vec<(
            String,
            Value,
            Option<KeepRaw<'_, DatumOption>>,
            Option<CborWrap<ScriptRef>>,
            Vec<u8>,
        )> = vec![
            (
                String::from("70faae60072c45d121b6e58ae35c624693ee3dad9ea8ed765eb6f76f9f"),
                Value::Coin(1624870),
                None,
                Some(CborWrap(ScriptRef::PlutusV3Script(PlutusScript::<3>(Bytes::from(hex::decode("58a701010032323232323225333002323232323253330073370e900118041baa0011323322533300a3370e900018059baa00513232533300f30110021533300c3370e900018069baa00313371e6eb8c040c038dd50039bae3010300e37546020601c6ea800c5858dd7180780098061baa00516300c001300c300d001300937540022c6014601600660120046010004601000260086ea8004526136565734aae7555cf2ab9f5742ae89").unwrap()))))),
            Vec::new(),
            ),
        ];

        add_codec_safe_ref_input_conway(&mtx.transaction_body, &mut utxos, &mut ref_info);

        let mut collateral_info: Vec<(
            String,
            Value,
            Option<KeepRaw<'_, DatumOption>>,
            Option<CborWrap<ScriptRef>>,
          Vec<u8>,
        )> = vec![(
            String::from("005c5c318d01f729e205c95eb1b02d623dd10e78ea58f72d0c13f892b2e8904edc699e2f0ce7b72be7cec991df651a222e2ae9244eb5975cba"),
            Value::Coin(2554439518),
            None,
            None,
            Vec::new(),
        )];
        add_codec_safe_collateral_conway(&mtx.transaction_body, &mut utxos, &mut collateral_info);
        let acnt = AccountState {
            treasury: 261_254_564_000_000,
            reserves: 0,
        };

        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Conway(mk_preview_params_epoch_380()),
            prot_magic: 2,
            block_slot: 74735000,
            network_id: 0,
            acnt: Some(acnt),
        };
        let mut cert_state: CertState = CertState::default();

        match validate_txs(&[metx.clone()], &env, &utxos, &mut cert_state) {
            Ok(()) => (),
            Err(err) => panic!("Unexpected error ({err:?})"),
        };

        #[cfg(feature = "phase2")]
        match pallas_validate::phase2::tx::eval_tx(
            &metx,
            env.prot_params(),
            &mk_utxo_for_eval(utxos.clone()),
            &pallas_validate::phase2::script_context::SlotConfig::default(),
        ) {
            Ok(_) => (),
            Err(err) => panic!("Unexpected error ({err:?})"),
        }
    }

    #[test]
    // Transaction hash:
    // 3e1ae85c08b610d5d03e67cf90e78980d1d2f54ffc50c21672e24180b450d354
    fn successful_mainnet_tx_with_plutus_v3_script() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/conway5.tx"));
        let mtx: Tx = conway_minted_tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_conway(&mtx);
        let datum_bytes = cbor_to_bytes("d8799f4568656c6c6fff");
        let datum_option = DatumOption::Data(CborWrap(minicbor::decode(&datum_bytes).unwrap()));
        let datum_option = minicbor::to_vec(datum_option).unwrap();
        let datum_option: KeepRaw<'_, DatumOption> = minicbor::decode(&datum_option).unwrap();

        let mut tx_outs_info: Vec<(
            String,
            Value,
            Option<KeepRaw<'_, DatumOption>>,
            Option<CborWrap<ScriptRef>>,
            Vec<u8>,
        )> = vec![(
            String::from("71faae60072c45d121b6e58ae35c624693ee3dad9ea8ed765eb6f76f9f"),
            Value::Coin(2000000),
            Some(datum_option),
            None,
            Vec::new(),
        )];

        let mut utxos: UTxOs =
            mk_codec_safe_utxo_for_conway_tx(&mtx.transaction_body, &mut tx_outs_info);

        let mut ref_info: Vec<(
            String,
            Value,
            Option<KeepRaw<'_, DatumOption>>,
            Option<CborWrap<ScriptRef>>,
            Vec<u8>,
        )> = vec![
            (
                String::from("71faae60072c45d121b6e58ae35c624693ee3dad9ea8ed765eb6f76f9f"),
                Value::Coin(1624870),
                None,
                Some(CborWrap(ScriptRef::PlutusV3Script(PlutusScript::<3>(Bytes::from(hex::decode("58a701010032323232323225333002323232323253330073370e900118041baa0011323322533300a3370e900018059baa00513232533300f30110021533300c3370e900018069baa00313371e6eb8c040c038dd50039bae3010300e37546020601c6ea800c5858dd7180780098061baa00516300c001300c300d001300937540022c6014601600660120046010004601000260086ea8004526136565734aae7555cf2ab9f5742ae89").unwrap()))))),
                Vec::new(),
            ),
        ];

        add_codec_safe_ref_input_conway(&mtx.transaction_body, &mut utxos, &mut ref_info);

        let mut collateral_info: Vec<(
            String,
            Value,
            Option<KeepRaw<'_, DatumOption>>,
            Option<CborWrap<ScriptRef>>,
            Vec<u8>,
        )> = vec![(
            String::from("015c5c318d01f729e205c95eb1b02d623dd10e78ea58f72d0c13f892b2e8904edc699e2f0ce7b72be7cec991df651a222e2ae9244eb5975cba"),
            Value::Coin(49731771),
            None,
            None,
            Vec::new(),
        )];
        add_codec_safe_collateral_conway(&mtx.transaction_body, &mut utxos, &mut collateral_info);

        let acnt = AccountState {
            treasury: 261_254_564_000_000,
            reserves: 0,
        };

        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Conway(mk_mainnet_params_epoch_380()),
            prot_magic: 764824073,
            block_slot: 149807950,
            network_id: 1,
            acnt: Some(acnt),
        };
        let mut cert_state: CertState = CertState::default();

        match validate_txs(&[metx.clone()], &env, &utxos, &mut cert_state) {
            Ok(()) => (),
            Err(err) => panic!("Unexpected error ({err:?})"),
        };

        #[cfg(feature = "phase2")]
        match pallas_validate::phase2::tx::eval_tx(
            &metx,
            env.prot_params(),
            &mk_utxo_for_eval(utxos.clone()),
            &pallas_validate::phase2::script_context::SlotConfig::default(),
        ) {
            Ok(_) => (),
            Err(err) => panic!("Unexpected error ({err:?})"),
        }
    }

    #[test]
    // Same as successful_mainnet_tx, except that all inputs are removed.
    fn empty_ins() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/conway3.tx"));
        let mut mtx: Tx = conway_minted_tx_from_cbor(&cbor_bytes);
        let tx_outs_info: &[(
            String,
            Value,
            Option<DatumOption>,
            Option<CborWrap<ScriptRef>>,
        )] = &[(
            String::from("015c5c318d01f729e205c95eb1b02d623dd10e78ea58f72d0c13f892b2e8904edc699e2f0ce7b72be7cec991df651a222e2ae9244eb5975cba"),
            Value::Coin(20000000),
            None,
            None,
        )];
        let utxos: UTxOs = mk_utxo_for_conway_tx(&mtx.transaction_body, tx_outs_info);
        let mut tx_body: TransactionBody = (*mtx.transaction_body).clone();
        tx_body.inputs = Set::from(Vec::new());
        let mut tx_buf: Vec<u8> = Vec::new();
        let _ = encode(tx_body, &mut tx_buf);
        mtx.transaction_body =
            Decode::decode(&mut Decoder::new(tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_conway(&mtx);
        let acnt = AccountState {
            treasury: 261_254_564_000_000,
            reserves: 0,
        };

        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Conway(mk_mainnet_params_epoch_365()),
            prot_magic: 764824073,
            block_slot: 72316896,
            network_id: 1,
            acnt: Some(acnt),
        };
        let mut cert_state: CertState = CertState::default();
        match validate_txs(&[metx], &env, &utxos, &mut cert_state) {
            Ok(()) => assert!(false, "Inputs set should not be empty"),
            Err(err) => match err {
                PostAlonzo(PostAlonzoError::TxInsEmpty) => (),
                _ => panic!("Unexpected error ({err:?})"),
            },
        }
    }

    #[test]
    // Same as successful_mainnet_tx, but validation is called with an empty UTxO
    // set.
    fn unfound_utxo_input() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/conway3.tx"));
        let mtx: Tx = conway_minted_tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_conway(&mtx);
        let utxos: UTxOs = UTxOs::new();
        let acnt = AccountState {
            treasury: 261_254_564_000_000,
            reserves: 0,
        };

        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Conway(mk_mainnet_params_epoch_365()),
            prot_magic: 764824073,
            block_slot: 72316896,
            network_id: 1,
            acnt: Some(acnt),
        };
        let mut cert_state: CertState = CertState::default();
        match validate_txs(&[metx], &env, &utxos, &mut cert_state) {
            Ok(()) => assert!(false, "All inputs should be within the UTxO set"),
            Err(err) => match err {
                PostAlonzo(PostAlonzoError::InputNotInUTxO) => (),
                _ => panic!("Unexpected error ({err:?})"),
            },
        }
    }

    #[test]
    // Same as successful_mainnet_tx, except that the lower bound of the validity
    // interval is greater than the block slot.
    fn validity_interval_lower_bound_unreached() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/conway3.tx"));
        let mut mtx: Tx = conway_minted_tx_from_cbor(&cbor_bytes);
        let tx_outs_info: &[(
            String,
            Value,
            Option<DatumOption>,
            Option<CborWrap<ScriptRef>>,
        )] = &[(
            String::from("015c5c318d01f729e205c95eb1b02d623dd10e78ea58f72d0c13f892b2e8904edc699e2f0ce7b72be7cec991df651a222e2ae9244eb5975cba"),
            Value::Coin(20000000),
            None,
            None,
        )];
        let utxos: UTxOs = mk_utxo_for_conway_tx(&mtx.transaction_body, tx_outs_info);
        let mut tx_body: TransactionBody = (*mtx.transaction_body).clone();
        tx_body.validity_interval_start = Some(72316897); // One slot after the block.
        let mut tx_buf: Vec<u8> = Vec::new();
        let _ = encode(tx_body, &mut tx_buf);
        mtx.transaction_body =
            Decode::decode(&mut Decoder::new(tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_conway(&mtx);
        let acnt = AccountState {
            treasury: 261_254_564_000_000,
            reserves: 0,
        };

        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Conway(mk_mainnet_params_epoch_365()),
            prot_magic: 764824073,
            block_slot: 72316896,
            network_id: 1,
            acnt: Some(acnt),
        };
        let mut cert_state: CertState = CertState::default();
        match validate_txs(&[metx], &env, &utxos, &mut cert_state) {
            Ok(()) => assert!(
                false,
                "Validity interval lower bound should have been reached"
            ),
            Err(err) => match err {
                PostAlonzo(PostAlonzoError::BlockPrecedesValInt) => (),
                _ => panic!("Unexpected error ({err:?})"),
            },
        }
    }

    #[test]
    // Same as successful_mainnet_tx, except that the upper bound of the validity
    // interval is lower than the block slot.
    fn validity_interval_upper_bound_surpassed() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/conway3.tx"));
        let mut mtx: Tx = conway_minted_tx_from_cbor(&cbor_bytes);
        let tx_outs_info: &[(
            String,
            Value,
            Option<DatumOption>,
            Option<CborWrap<ScriptRef>>,
        )] = &[(
            String::from("015c5c318d01f729e205c95eb1b02d623dd10e78ea58f72d0c13f892b2e8904edc699e2f0ce7b72be7cec991df651a222e2ae9244eb5975cba"),
            Value::Coin(20000000),
            None,
            None,
        )];
        let utxos: UTxOs = mk_utxo_for_conway_tx(&mtx.transaction_body, tx_outs_info);
        let mut tx_body: TransactionBody = (*mtx.transaction_body).clone();
        tx_body.ttl = Some(72316895); // One slot before the block.
        let mut tx_buf: Vec<u8> = Vec::new();
        let _ = encode(tx_body, &mut tx_buf);
        mtx.transaction_body =
            Decode::decode(&mut Decoder::new(tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_conway(&mtx);
        let acnt = AccountState {
            treasury: 261_254_564_000_000,
            reserves: 0,
        };

        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Conway(mk_mainnet_params_epoch_365()),
            prot_magic: 764824073,
            block_slot: 72316896,
            network_id: 1,
            acnt: Some(acnt),
        };
        let mut cert_state: CertState = CertState::default();
        match validate_txs(&[metx], &env, &utxos, &mut cert_state) {
            Ok(()) => assert!(
                false,
                "Validity interval upper bound should not have been surpassed"
            ),
            Err(err) => match err {
                PostAlonzo(PostAlonzoError::BlockExceedsValInt) => (),
                _ => panic!("Unexpected error ({err:?})"),
            },
        }
    }

    #[test]
    // Same as successful_mainnet_tx, except that validation is called with an
    // Environment requesting fees that exceed those paid by the transaction.
    fn min_fee_unreached() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/conway3.tx"));
        let mtx: Tx = conway_minted_tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_conway(&mtx);
        let tx_outs_info: &[(
            String,
            Value,
            Option<DatumOption>,
            Option<CborWrap<ScriptRef>>,
        )] = &[(
            String::from("015c5c318d01f729e205c95eb1b02d623dd10e78ea58f72d0c13f892b2e8904edc699e2f0ce7b72be7cec991df651a222e2ae9244eb5975cba"),
            Value::Coin(20000000),
            None,
            None,
        )];
        let utxos: UTxOs = mk_utxo_for_conway_tx(&mtx.transaction_body, tx_outs_info);
        let mut conway_prot_params: ConwayProtParams = mk_mainnet_params_epoch_365();
        conway_prot_params.minfee_a = 6000000; // This value was 44 during Babbage on mainnet.
        let acnt = AccountState {
            treasury: 261_254_564_000_000,
            reserves: 0,
        };

        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Conway(conway_prot_params),
            prot_magic: 764824073,
            block_slot: 72316896,
            network_id: 1,
            acnt: Some(acnt),
        };
        let mut cert_state: CertState = CertState::default();
        match validate_txs(&[metx], &env, &utxos, &mut cert_state) {
            Ok(()) => assert!(false, "Fee should not be below minimum"),
            Err(err) => match err {
                PostAlonzo(PostAlonzoError::FeeBelowMin) => (),
                _ => panic!("Unexpected error ({err:?})"),
            },
        }
    }

    #[test]
    // Same as successful_mainnet_tx, except that the fee is reduced by exactly 1,
    // and so the "preservation of value" property doesn't hold.
    fn preservation_of_value() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/conway3.tx"));
        let mut mtx: Tx = conway_minted_tx_from_cbor(&cbor_bytes);
        let tx_outs_info: &[(
            String,
            Value,
            Option<DatumOption>,
            Option<CborWrap<ScriptRef>>,
        )] = &[(
            String::from("015c5c318d01f729e205c95eb1b02d623dd10e78ea58f72d0c13f892b2e8904edc699e2f0ce7b72be7cec991df651a222e2ae9244eb5975cba"),
            Value::Coin(20000000),
            None,
            None,
        )];
        let utxos: UTxOs = mk_utxo_for_conway_tx(&mtx.transaction_body, tx_outs_info);
        let mut tx_body: TransactionBody = (*mtx.transaction_body).clone();
        tx_body.fee -= 1;
        let mut tx_buf: Vec<u8> = Vec::new();
        let _ = encode(tx_body, &mut tx_buf);
        mtx.transaction_body =
            Decode::decode(&mut Decoder::new(tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_conway(&mtx);
        let acnt = AccountState {
            treasury: 261_254_564_000_000,
            reserves: 0,
        };

        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Conway(mk_mainnet_params_epoch_365()),
            prot_magic: 764824073,
            block_slot: 72316896,
            network_id: 1,
            acnt: Some(acnt),
        };
        let mut cert_state: CertState = CertState::default();
        match validate_txs(&[metx], &env, &utxos, &mut cert_state) {
            Ok(()) => assert!(false, "Preservation of value does not hold"),
            Err(err) => match err {
                PostAlonzo(PostAlonzoError::PreservationOfValue) => (),
                _ => panic!("Unexpected error ({err:?})"),
            },
        }
    }

    #[test]
    // Same as successful_mainnet_tx, except that the minimum lovelace in an output
    // is unreached.
    fn min_lovelace_unreached() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/conway3.tx"));
        let mtx: Tx = conway_minted_tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_conway(&mtx);
        let tx_outs_info: &[(
            String,
            Value,
            Option<DatumOption>,
            Option<CborWrap<ScriptRef>>,
        )] = &[(
            String::from("015c5c318d01f729e205c95eb1b02d623dd10e78ea58f72d0c13f892b2e8904edc699e2f0ce7b72be7cec991df651a222e2ae9244eb5975cba"),
            Value::Coin(20000000),
            None,
            None,
        )];
        let utxos: UTxOs = mk_utxo_for_conway_tx(&mtx.transaction_body, tx_outs_info);
        let mut conway_prot_params: ConwayProtParams = mk_mainnet_params_epoch_365();
        conway_prot_params.ada_per_utxo_byte = 10000000; // This was 4310 during Alonzo on mainnet.
        let acnt = AccountState {
            treasury: 261_254_564_000_000,
            reserves: 0,
        };

        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Conway(conway_prot_params),
            prot_magic: 764824073,
            block_slot: 72316896,
            network_id: 1,
            acnt: Some(acnt),
        };
        let mut cert_state: CertState = CertState::default();
        match validate_txs(&[metx], &env, &utxos, &mut cert_state) {
            Ok(()) => assert!(false, "Output minimum lovelace is unreached"),
            Err(err) => match err {
                PostAlonzo(PostAlonzoError::MinLovelaceUnreached) => (),
                _ => panic!("Unexpected error ({err:?})"),
            },
        }
    }

    #[test]
    // Same as successful_mainnet_tx, except that the value size exceeds the
    // environment parameter.
    fn max_val_exceeded() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/conway3.tx"));
        let mtx: Tx = conway_minted_tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_conway(&mtx);
        let tx_outs_info: &[(
            String,
            Value,
            Option<DatumOption>,
            Option<CborWrap<ScriptRef>>,
        )] = &[(
            String::from("015c5c318d01f729e205c95eb1b02d623dd10e78ea58f72d0c13f892b2e8904edc699e2f0ce7b72be7cec991df651a222e2ae9244eb5975cba"),
            Value::Coin(20000000),
            None,
            None,
        )];
        let utxos: UTxOs = mk_utxo_for_conway_tx(&mtx.transaction_body, tx_outs_info);
        let mut conway_prot_params: ConwayProtParams = mk_mainnet_params_epoch_365();
        conway_prot_params.max_value_size = 0; // This value was 5000 during Babbage on mainnet.
        let acnt = AccountState {
            treasury: 261_254_564_000_000,
            reserves: 0,
        };

        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Conway(conway_prot_params),
            prot_magic: 764824073,
            block_slot: 72316896,
            network_id: 1,
            acnt: Some(acnt),
        };
        let mut cert_state: CertState = CertState::default();
        match validate_txs(&[metx], &env, &utxos, &mut cert_state) {
            Ok(()) => assert!(false, "Max value size exceeded"),
            Err(err) => match err {
                PostAlonzo(PostAlonzoError::MaxValSizeExceeded) => (),
                _ => panic!("Unexpected error ({err:?})"),
            },
        }
    }

    #[test]
    // Same as successful_mainnet_tx, except that the transaction's network ID is
    // altered.
    fn tx_network_id() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/conway3.tx"));
        let mut mtx: Tx = conway_minted_tx_from_cbor(&cbor_bytes);
        let tx_outs_info: &[(
            String,
            Value,
            Option<DatumOption>,
            Option<CborWrap<ScriptRef>>,
        )] = &[(
            String::from("015c5c318d01f729e205c95eb1b02d623dd10e78ea58f72d0c13f892b2e8904edc699e2f0ce7b72be7cec991df651a222e2ae9244eb5975cba"),
            Value::Coin(20000000),
            None,
            None,
        )];
        let utxos: UTxOs = mk_utxo_for_conway_tx(&mtx.transaction_body, tx_outs_info);
        let mut tx_body: TransactionBody = (*mtx.transaction_body).clone();
        tx_body.network_id = Some(NetworkId::Testnet);
        let mut tx_buf: Vec<u8> = Vec::new();
        let _ = encode(tx_body, &mut tx_buf);
        mtx.transaction_body =
            Decode::decode(&mut Decoder::new(tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_conway(&mtx);
        let acnt = AccountState {
            treasury: 261_254_564_000_000,
            reserves: 0,
        };

        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Conway(mk_mainnet_params_epoch_365()),
            prot_magic: 764824073,
            block_slot: 72316896,
            network_id: 1,
            acnt: Some(acnt),
        };
        let mut cert_state: CertState = CertState::default();
        match validate_txs(&[metx], &env, &utxos, &mut cert_state) {
            Ok(()) => assert!(
                false,
                "Transaction network ID should match environment network ID"
            ),
            Err(err) => match err {
                PostAlonzo(PostAlonzoError::TxWrongNetworkID) => (),
                _ => panic!("Unexpected error ({err:?})"),
            },
        }
    }

    #[test]
    // Same as successful_mainnet_tx_with_plutus_v3_script, except that all
    // collaterals are removed before calling validation.
    fn no_collateral_inputs() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/conway6.tx"));
        let mut mtx: Tx = conway_minted_tx_from_cbor(&cbor_bytes);
        let datum_bytes = cbor_to_bytes("d8799f4568656c6c6fff");

        let tx_outs_info: &[(
            String,
            Value,
            Option<DatumOption>,
            Option<CborWrap<ScriptRef>>,
        )] = &[(
            String::from("71faae60072c45d121b6e58ae35c624693ee3dad9ea8ed765eb6f76f9f"),
            Value::Coin(2000000),
            Some(DatumOption::Data(CborWrap(
                minicbor::decode(&datum_bytes).unwrap(),
            ))),
            None,
        )];

        let mut utxos: UTxOs = mk_utxo_for_conway_tx(&mtx.transaction_body, tx_outs_info);

        let collateral_info: &[(
            String,
            Value,
            Option<DatumOption>,
            Option<CborWrap<ScriptRef>>,
        )] = &[(
            String::from("015c5c318d01f729e205c95eb1b02d623dd10e78ea58f72d0c13f892b2e8904edc699e2f0ce7b72be7cec991df651a222e2ae9244eb5975cba"),
            Value::Coin(49731771),
            None,
            None,
        )];
        add_collateral_conway(&mtx.transaction_body, &mut utxos, collateral_info);

        let acnt = AccountState {
            treasury: 261_254_564_000_000,
            reserves: 0,
        };

        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Conway(mk_mainnet_params_epoch_380()),
            prot_magic: 764824073,
            block_slot: 149807950,
            network_id: 1,
            acnt: Some(acnt),
        };
        let mut tx_body: TransactionBody = (*mtx.transaction_body).clone();
        tx_body.collateral = None;
        let mut tx_buf: Vec<u8> = Vec::new();
        let _ = encode(tx_body, &mut tx_buf);
        mtx.transaction_body =
            Decode::decode(&mut Decoder::new(tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_conway(&mtx);

        let mut cert_state: CertState = CertState::default();
        match validate_txs(&[metx], &env, &utxos, &mut cert_state) {
            Ok(()) => assert!(false, "No collateral inputs"),
            Err(err) => match err {
                PostAlonzo(PostAlonzoError::CollateralMissing) => (),
                _ => panic!("Unexpected error ({err:?})"),
            },
        }
    }

    #[test]
    // Same as successful_mainnet_tx_with_plutus_v3_script, except that validation
    // is called on an environment which does not allow enough collateral inputs
    // for the transaction to be valid.
    fn too_many_collateral_inputs() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/conway6.tx"));
        let mtx: Tx = conway_minted_tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_conway(&mtx);
        let datum_bytes = cbor_to_bytes("d8799f4568656c6c6fff");

        let tx_outs_info: &[(
            String,
            Value,
            Option<DatumOption>,
            Option<CborWrap<ScriptRef>>,
        )] = &[(
            String::from("71faae60072c45d121b6e58ae35c624693ee3dad9ea8ed765eb6f76f9f"),
            Value::Coin(2000000),
            Some(DatumOption::Data(CborWrap(
                minicbor::decode(&datum_bytes).unwrap(),
            ))),
            None,
        )];

        let mut utxos: UTxOs = mk_utxo_for_conway_tx(&mtx.transaction_body, tx_outs_info);

        let collateral_info: &[(
            String,
            Value,
            Option<DatumOption>,
            Option<CborWrap<ScriptRef>>,
        )] = &[(
            String::from("015c5c318d01f729e205c95eb1b02d623dd10e78ea58f72d0c13f892b2e8904edc699e2f0ce7b72be7cec991df651a222e2ae9244eb5975cba"),
            Value::Coin(49731771),
            None,
            None,
        )];
        add_collateral_conway(&mtx.transaction_body, &mut utxos, collateral_info);

        let acnt = AccountState {
            treasury: 261_254_564_000_000,
            reserves: 0,
        };

        let mut conway_prot_params: ConwayProtParams = mk_mainnet_params_epoch_380();
        conway_prot_params.max_collateral_inputs = 0;

        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Conway(conway_prot_params),
            prot_magic: 764824073,
            block_slot: 149807950,
            network_id: 1,
            acnt: Some(acnt),
        };

        let mut cert_state: CertState = CertState::default();
        match validate_txs(&[metx], &env, &utxos, &mut cert_state) {
            Ok(()) => assert!(false, "Number of collateral inputs should be within limits"),
            Err(err) => match err {
                PostAlonzo(PostAlonzoError::TooManyCollaterals) => (),
                _ => panic!("Unexpected error ({err:?})"),
            },
        }
    }

    #[test]
    // Same as successful_mainnet_tx_with_plutus_v3_script, except that the address
    // of a collateral inputs is altered into a script-locked one.
    fn collateral_is_not_verification_key_locked() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/conway6.tx"));
        let mtx: Tx = conway_minted_tx_from_cbor(&cbor_bytes);
        let datum_bytes = cbor_to_bytes("d8799f4568656c6c6fff");

        let tx_outs_info: &[(
            String,
            Value,
            Option<DatumOption>,
            Option<CborWrap<ScriptRef>>,
        )] = &[(
            String::from("71faae60072c45d121b6e58ae35c624693ee3dad9ea8ed765eb6f76f9f"),
            Value::Coin(2000000),
            Some(DatumOption::Data(CborWrap(
                minicbor::decode(&datum_bytes).unwrap(),
            ))),
            None,
        )];

        let mut utxos: UTxOs = mk_utxo_for_conway_tx(&mtx.transaction_body, tx_outs_info);

        let acnt = AccountState {
            treasury: 261_254_564_000_000,
            reserves: 0,
        };

        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Conway(mk_mainnet_params_epoch_380()),
            prot_magic: 764824073,
            block_slot: 149807950,
            network_id: 1,
            acnt: Some(acnt),
        };
        let old_address: Address = match hex::decode(String::from("015c5c318d01f729e205c95eb1b02d623dd10e78ea58f72d0c13f892b2e8904edc699e2f0ce7b72be7cec991df651a222e2ae9244eb5975cba")) {
            Ok(bytes_vec) => Address::from_bytes(bytes_vec.as_slice()).unwrap(),
            _ => panic!("Unable to parse collateral input address"),
        };
        let old_shelley_address: ShelleyAddress = match old_address {
            Address::Shelley(shelley_addr) => shelley_addr,
            _ => panic!("Unable to parse collateral input address"),
        };
        let altered_address: ShelleyAddress = ShelleyAddress::new(
            old_shelley_address.network(),
            ShelleyPaymentPart::Script(*old_shelley_address.payment().as_hash()),
            old_shelley_address.delegation().clone(),
        );
        let tx_in = mtx
            .transaction_body
            .collateral
            .clone()
            .unwrap()
            .to_vec()
            .pop()
            .unwrap();
        let multi_era_in: MultiEraInput =
            MultiEraInput::AlonzoCompatible(Box::new(Cow::Owned(tx_in.clone())));
        let multi_era_out: MultiEraOutput =
            MultiEraOutput::Conway(Box::new(Cow::Owned(TransactionOutput::PostAlonzo(
                PostAlonzoTransactionOutput {
                    address: Bytes::try_from(altered_address.to_hex()).unwrap(),
                    value: Value::Coin(5000000),
                    datum_option: None,
                    script_ref: None,
                }
                .into(),
            ))));
        utxos.insert(multi_era_in, multi_era_out);
        let metx: MultiEraTx = MultiEraTx::from_conway(&mtx);

        let mut cert_state: CertState = CertState::default();
        match validate_txs(&[metx], &env, &utxos, &mut cert_state) {
            Ok(()) => assert!(false, "Collateral inputs should be verification-key locked"),
            Err(err) => match err {
                PostAlonzo(PostAlonzoError::CollateralNotVKeyLocked) => (),
                _ => panic!("Unexpected error ({err:?})"),
            },
        }
    }

    #[test]
    // Same as successful_mainnet_tx_with_plutus_v3_script, except that the balance
    // between assets in collateral inputs and assets in collateral return output
    // contains assets other than lovelace.
    fn collateral_with_other_assets() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/conway6.tx"));
        let mtx: Tx = conway_minted_tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_conway(&mtx);
        let datum_bytes = cbor_to_bytes("d8799f4568656c6c6fff");

        let tx_outs_info: &[(
            String,
            Value,
            Option<DatumOption>,
            Option<CborWrap<ScriptRef>>,
        )] = &[(
            String::from("71faae60072c45d121b6e58ae35c624693ee3dad9ea8ed765eb6f76f9f"),
            Value::Coin(2000000),
            Some(DatumOption::Data(CborWrap(
                minicbor::decode(&datum_bytes).unwrap(),
            ))),
            None,
        )];

        let mut utxos: UTxOs = mk_utxo_for_conway_tx(&mtx.transaction_body, tx_outs_info);

        let acnt = AccountState {
            treasury: 261_254_564_000_000,
            reserves: 0,
        };

        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Conway(mk_mainnet_params_epoch_380()),
            prot_magic: 764824073,
            block_slot: 149807950,
            network_id: 1,
            acnt: Some(acnt),
        };
        let collateral_info: &[(
            String,
            Value,
            Option<DatumOption>,
            Option<CborWrap<ScriptRef>>,
        )] = &[(
            String::from("01f1e126304308006938d2e8571842ff87302fff95a037b3fd838451b8b3c9396d0680d912487139cb7fc85aa279ea70e8cdacee4c6cae40fd"),
            Value::Multiasset(
                5000000,
                [(
                    "b001076b34a87e7d48ec46703a6f50f93289582ad9bdbeff7f1e3295"
                        .parse().
                        unwrap(),
                    [(
                        Bytes::from(
                            hex::decode("4879706562656173747332343233")
                                .unwrap(),
                        ),
                        PositiveCoin::try_from(1000).ok().unwrap(),
                    )].into()
                )].into()
            ),
            None,
            None,
        )];
        add_collateral_conway(&mtx.transaction_body, &mut utxos, collateral_info);

        let mut cert_state: CertState = CertState::default();
        match validate_txs(&[metx], &env, &utxos, &mut cert_state) {
            Ok(()) => assert!(false, "Collateral balance should contained only lovelace"),
            Err(err) => match err {
                PostAlonzo(PostAlonzoError::NonLovelaceCollateral) => (),
                _ => panic!("Unexpected error ({err:?})"),
            },
        }
    }

    #[test]
    // Same as successful_mainnet_tx_with_plutus_v3_script, except that the number
    // of lovelace in the total collateral balance is insufficient.
    fn collateral_min_lovelace() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/conway6.tx"));
        let mtx: Tx = conway_minted_tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_conway(&mtx);
        let datum_bytes = cbor_to_bytes("d8799f4568656c6c6fff");

        let tx_outs_info: &[(
            String,
            Value,
            Option<DatumOption>,
            Option<CborWrap<ScriptRef>>,
        )] = &[(
            String::from("71faae60072c45d121b6e58ae35c624693ee3dad9ea8ed765eb6f76f9f"),
            Value::Coin(2000000),
            Some(DatumOption::Data(CborWrap(
                minicbor::decode(&datum_bytes).unwrap(),
            ))),
            None,
        )];

        let mut utxos: UTxOs = mk_utxo_for_conway_tx(&mtx.transaction_body, tx_outs_info);

        let collateral_info: &[(
            String,
            Value,
            Option<DatumOption>,
            Option<CborWrap<ScriptRef>>,
        )] = &[(
            String::from("015c5c318d01f729e205c95eb1b02d623dd10e78ea58f72d0c13f892b2e8904edc699e2f0ce7b72be7cec991df651a222e2ae9244eb5975cba"),
            Value::Coin(88118796),
            None,
            None,
        )];
        add_collateral_conway(&mtx.transaction_body, &mut utxos, collateral_info);

        let acnt = AccountState {
            treasury: 261_254_564_000_000,
            reserves: 0,
        };

        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Conway(mk_mainnet_params_epoch_380()),
            prot_magic: 764824073,
            block_slot: 149807950,
            network_id: 1,
            acnt: Some(acnt),
        };
        let mut conway_prot_params: ConwayProtParams = mk_mainnet_params_epoch_380();
        conway_prot_params.collateral_percentage = 10;

        let mut cert_state: CertState = CertState::default();
        match validate_txs(&[metx], &env, &utxos, &mut cert_state) {
            Ok(()) => assert!(
                false,
                "Collateral balance should contained the minimum lovelace"
            ),
            Err(err) => match err {
                PostAlonzo(PostAlonzoError::CollateralMinLovelace) => (),
                _ => panic!("Unexpected error ({err:?})"),
            },
        }
    }

    #[test]
    // Same as successful_mainnet_tx_with_plutus_v3_script, except that the
    // annotated collateral is wrong.
    fn collateral_annotation() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/conway6.tx"));
        let mut mtx: Tx = conway_minted_tx_from_cbor(&cbor_bytes);
        let datum_bytes = cbor_to_bytes("d8799f4568656c6c6fff");

        let tx_outs_info: &[(
            String,
            Value,
            Option<DatumOption>,
            Option<CborWrap<ScriptRef>>,
        )] = &[(
            String::from("71faae60072c45d121b6e58ae35c624693ee3dad9ea8ed765eb6f76f9f"),
            Value::Coin(2000000),
            Some(DatumOption::Data(CborWrap(
                minicbor::decode(&datum_bytes).unwrap(),
            ))),
            None,
        )];

        let mut utxos: UTxOs = mk_utxo_for_conway_tx(&mtx.transaction_body, tx_outs_info);

        let collateral_info: &[(
            String,
            Value,
            Option<DatumOption>,
            Option<CborWrap<ScriptRef>>,
        )] = &[(
            String::from("015c5c318d01f729e205c95eb1b02d623dd10e78ea58f72d0c13f892b2e8904edc699e2f0ce7b72be7cec991df651a222e2ae9244eb5975cba"),
            Value::Coin(100118796),
            None,
            None,
        )];
        add_collateral_conway(&mtx.transaction_body, &mut utxos, collateral_info);

        let acnt = AccountState {
            treasury: 261_254_564_000_000,
            reserves: 0,
        };

        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Conway(mk_mainnet_params_epoch_380()),
            prot_magic: 764824073,
            block_slot: 149807950,
            network_id: 1,
            acnt: Some(acnt),
        };

        let mut tx_body: TransactionBody = (*mtx.transaction_body).clone();
        tx_body.total_collateral = Some(5000001); // This is 1 more than the actual paid collateral
        let mut tx_buf: Vec<u8> = Vec::new();
        let _ = encode(tx_body, &mut tx_buf);
        mtx.transaction_body =
            Decode::decode(&mut Decoder::new(tx_buf.as_slice()), &mut ()).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_conway(&mtx);

        let mut cert_state: CertState = CertState::default();
        match validate_txs(&[metx], &env, &utxos, &mut cert_state) {
            Ok(()) => assert!(false, "Collateral annotation"),
            Err(err) => match err {
                PostAlonzo(PostAlonzoError::CollateralAnnotation) => (),
                _ => panic!("Unexpected error ({err:?})"),
            },
        }
    }

    #[test]
    // Same as successful_mainnet_tx, except that the Environment with which
    // validation is called demands the transaction to be smaller than it
    // actually is.
    fn max_tx_size_exceeded() {
        let cbor_bytes: Vec<u8> = cbor_to_bytes(include_str!("../../test_data/conway3.tx"));
        let mtx: Tx = conway_minted_tx_from_cbor(&cbor_bytes);
        let metx: MultiEraTx = MultiEraTx::from_conway(&mtx);
        let tx_outs_info: &[(
            String,
            Value,
            Option<DatumOption>,
            Option<CborWrap<ScriptRef>>,
        )] = &[(
            String::from("015c5c318d01f729e205c95eb1b02d623dd10e78ea58f72d0c13f892b2e8904edc699e2f0ce7b72be7cec991df651a222e2ae9244eb5975cba"),
            Value::Coin(20000000),
            None,
            None,
        )];
        let utxos: UTxOs = mk_utxo_for_conway_tx(&mtx.transaction_body, tx_outs_info);
        let mut conway_prot_params: ConwayProtParams = mk_mainnet_params_epoch_365();
        conway_prot_params.max_transaction_size = 154; // 1 less than the size of the transaction.
        let acnt = AccountState {
            treasury: 261_254_564_000_000,
            reserves: 0,
        };

        let env: Environment = Environment {
            prot_params: MultiEraProtocolParameters::Conway(conway_prot_params),
            prot_magic: 764824073,
            block_slot: 72316896,
            network_id: 1,
            acnt: Some(acnt),
        };
        let mut cert_state: CertState = CertState::default();
        match validate_txs(&[metx], &env, &utxos, &mut cert_state) {
            Ok(()) => assert!(
                false,
                "Transaction size should not exceed the maximum allowed"
            ),
            Err(err) => match err {
                PostAlonzo(PostAlonzoError::MaxTxSizeExceeded) => (),
                _ => panic!("Unexpected error ({err:?})"),
            },
        }
    }

    fn mk_mainnet_params_epoch_365() -> ConwayProtParams {
        ConwayProtParams {
            system_start: chrono::DateTime::parse_from_rfc3339("2017-09-23T21:44:51Z").unwrap(),
            epoch_length: 432000,
            slot_length: 1,
            minfee_a: 44,
            minfee_b: 155381,
            max_block_body_size: 90112,
            max_transaction_size: 16384,
            max_block_header_size: 1100,
            key_deposit: 2000000,
            pool_deposit: 500000000,
            maximum_epoch: 18,
            desired_number_of_stake_pools: 500,
            pool_pledge_influence: RationalNumber {
                numerator: 3,
                denominator: 10,
            },
            expansion_rate: RationalNumber {
                numerator: 3,
                denominator: 1000,
            },
            treasury_growth_rate: RationalNumber {
                numerator: 2,
                denominator: 10,
            },
            protocol_version: (7, 0),
            min_pool_cost: 340000000,
            ada_per_utxo_byte: 4310,
            cost_models_for_script_languages: CostModels {
                plutus_v1: Some(vec![
                    197209, 0, 1, 1, 396231, 621, 0, 1, 150000, 1000, 0, 1, 150000, 32, 2477736,
                    29175, 4, 29773, 100, 29773, 100, 29773, 100, 29773, 100, 29773, 100, 29773,
                    100, 100, 100, 29773, 100, 150000, 32, 150000, 32, 150000, 32, 150000, 1000, 0,
                    1, 150000, 32, 150000, 1000, 0, 8, 148000, 425507, 118, 0, 1, 1, 150000, 1000,
                    0, 8, 150000, 112536, 247, 1, 150000, 10000, 1, 136542, 1326, 1, 1000, 150000,
                    1000, 1, 150000, 32, 150000, 32, 150000, 32, 1, 1, 150000, 1, 150000, 4,
                    103599, 248, 1, 103599, 248, 1, 145276, 1366, 1, 179690, 497, 1, 150000, 32,
                    150000, 32, 150000, 32, 150000, 32, 150000, 32, 150000, 32, 148000, 425507,
                    118, 0, 1, 1, 61516, 11218, 0, 1, 150000, 32, 148000, 425507, 118, 0, 1, 1,
                    148000, 425507, 118, 0, 1, 1, 2477736, 29175, 4, 0, 82363, 4, 150000, 5000, 0,
                    1, 150000, 32, 197209, 0, 1, 1, 150000, 32, 150000, 32, 150000, 32, 150000, 32,
                    150000, 32, 150000, 32, 150000, 32, 3345831, 1, 1,
                ]),

                plutus_v2: None,
                plutus_v3: None,
                unknown: BTreeMap::default(),
            },
            execution_costs: pallas_primitives::ExUnitPrices {
                mem_price: RationalNumber {
                    numerator: 577,
                    denominator: 10000,
                },
                step_price: RationalNumber {
                    numerator: 721,
                    denominator: 10000000,
                },
            },
            max_tx_ex_units: ExUnits {
                mem: 14000000,
                steps: 10000000000,
            },
            max_block_ex_units: ExUnits {
                mem: 62000000,
                steps: 40000000000,
            },
            max_value_size: 5000,
            collateral_percentage: 150,
            max_collateral_inputs: 3,
            pool_voting_thresholds: PoolVotingThresholds {
                motion_no_confidence: RationalNumber {
                    numerator: 50,
                    denominator: 100,
                },
                committee_normal: RationalNumber {
                    numerator: 60,
                    denominator: 100,
                },
                committee_no_confidence: RationalNumber {
                    numerator: 40,
                    denominator: 100,
                },
                hard_fork_initiation: RationalNumber {
                    numerator: 75,
                    denominator: 100,
                },
                security_voting_threshold: RationalNumber {
                    numerator: 80,
                    denominator: 100,
                },
            },
            drep_voting_thresholds: DRepVotingThresholds {
                motion_no_confidence: RationalNumber {
                    numerator: 10,
                    denominator: 100,
                },
                committee_normal: RationalNumber {
                    numerator: 25,
                    denominator: 100,
                },
                committee_no_confidence: RationalNumber {
                    numerator: 15,
                    denominator: 100,
                },
                update_constitution: RationalNumber {
                    numerator: 50,
                    denominator: 100,
                },
                hard_fork_initiation: RationalNumber {
                    numerator: 60,
                    denominator: 100,
                },
                pp_network_group: RationalNumber {
                    numerator: 55,
                    denominator: 100,
                },
                pp_economic_group: RationalNumber {
                    numerator: 65,
                    denominator: 100,
                },
                pp_technical_group: RationalNumber {
                    numerator: 70,
                    denominator: 100,
                },
                pp_governance_group: RationalNumber {
                    numerator: 85,
                    denominator: 100,
                },
                treasury_withdrawal: RationalNumber {
                    numerator: 90,
                    denominator: 100,
                },
            },
            min_committee_size: 10,
            committee_term_limit: 5,
            governance_action_validity_period: 3600, // in seconds
            governance_action_deposit: 1000,         // arbitrary value
            drep_deposit: 2000,                      // arbitrary value
            drep_inactivity_period: 60,              // in seconds
            minfee_refscript_cost_per_byte: RationalNumber {
                numerator: 10,
                denominator: 100,
            },
        }
    }

    fn mk_mainnet_params_epoch_380() -> ConwayProtParams {
        ConwayProtParams {
            system_start: chrono::DateTime::parse_from_rfc3339("2022-10-25T00:00:00Z").unwrap(),
            epoch_length: 432000,
            slot_length: 1,
            minfee_a: 44,
            minfee_b: 155381,
            max_block_body_size: 90112,
            max_transaction_size: 16384,
            max_block_header_size: 1100,
            key_deposit: 2000000,
            pool_deposit: 500000000,
            maximum_epoch: 18,
            desired_number_of_stake_pools: 500,
            pool_pledge_influence: RationalNumber {
                numerator: 3,
                denominator: 10,
            },
            expansion_rate: RationalNumber {
                numerator: 3,
                denominator: 1000,
            },
            treasury_growth_rate: RationalNumber {
                numerator: 2,
                denominator: 10,
            },
            protocol_version: (7, 0),
            min_pool_cost: 340000000,
            ada_per_utxo_byte: 4310,
            cost_models_for_script_languages: CostModels {
                plutus_v1: Some(vec![
                    205665, 812, 1, 1, 1000, 571, 0, 1, 1000, 24177, 4, 1, 1000, 32, 117366, 10475,
                    4, 23000, 100, 23000, 100, 23000, 100, 23000, 100, 23000, 100, 23000, 100, 100,
                    100, 23000, 100, 19537, 32, 175354, 32, 46417, 4, 221973, 511, 0, 1, 89141, 32,
                    497525, 14068, 4, 2, 196500, 453240, 220, 0, 1, 1, 1000, 28662, 4, 2, 245000,
                    216773, 62, 1, 1060367, 12586, 1, 208512, 421, 1, 187000, 1000, 52998, 1,
                    80436, 32, 43249, 32, 1000, 32, 80556, 1, 57667, 4, 1000, 10, 197145, 156, 1,
                    197145, 156, 1, 204924, 473, 1, 208896, 511, 1, 52467, 32, 64832, 32, 65493,
                    32, 22558, 32, 16563, 32, 76511, 32, 196500, 453240, 220, 0, 1, 1, 69522,
                    11687, 0, 1, 60091, 32, 196500, 453240, 220, 0, 1, 1, 196500, 453240, 220, 0,
                    1, 1, 806990, 30482, 4, 1927926, 82523, 4, 265318, 0, 4, 0, 85931, 32, 205665,
                    812, 1, 1, 41182, 32, 212342, 32, 31220, 32, 32696, 32, 43357, 32, 32247, 32,
                    38314, 32, 9462713, 1021, 10,
                ]),

                plutus_v2: Some(vec![
                    205665,
                    812,
                    1,
                    1,
                    1000,
                    571,
                    0,
                    1,
                    1000,
                    24177,
                    4,
                    1,
                    1000,
                    32,
                    117366,
                    10475,
                    4,
                    23000,
                    100,
                    23000,
                    100,
                    23000,
                    100,
                    23000,
                    100,
                    23000,
                    100,
                    23000,
                    100,
                    100,
                    100,
                    23000,
                    100,
                    19537,
                    32,
                    175354,
                    32,
                    46417,
                    4,
                    221973,
                    511,
                    0,
                    1,
                    89141,
                    32,
                    497525,
                    14068,
                    4,
                    2,
                    196500,
                    453240,
                    220,
                    0,
                    1,
                    1,
                    1000,
                    28662,
                    4,
                    2,
                    245000,
                    216773,
                    62,
                    1,
                    1060367,
                    12586,
                    1,
                    208512,
                    421,
                    1,
                    187000,
                    1000,
                    52998,
                    1,
                    80436,
                    32,
                    43249,
                    32,
                    1000,
                    32,
                    80556,
                    1,
                    57667,
                    4,
                    1000,
                    10,
                    197145,
                    156,
                    1,
                    197145,
                    156,
                    1,
                    204924,
                    473,
                    1,
                    208896,
                    511,
                    1,
                    52467,
                    32,
                    64832,
                    32,
                    65493,
                    32,
                    22558,
                    32,
                    16563,
                    32,
                    76511,
                    32,
                    196500,
                    453240,
                    220,
                    0,
                    1,
                    1,
                    69522,
                    11687,
                    0,
                    1,
                    60091,
                    32,
                    196500,
                    453240,
                    220,
                    0,
                    1,
                    1,
                    196500,
                    453240,
                    220,
                    0,
                    1,
                    1,
                    1159724,
                    392670,
                    0,
                    2,
                    806990,
                    30482,
                    4,
                    1927926,
                    82523,
                    4,
                    265318,
                    0,
                    4,
                    0,
                    85931,
                    32,
                    205665,
                    812,
                    1,
                    1,
                    41182,
                    32,
                    212342,
                    32,
                    31220,
                    32,
                    32696,
                    32,
                    43357,
                    32,
                    32247,
                    32,
                    38314,
                    32,
                    20000000000,
                    20000000000,
                    9462713,
                    1021,
                    10,
                    20000000000,
                    0,
                    20000000000,
                ]),
                plutus_v3: Some(vec![
                    100788, 420, 1, 1, 1000, 173, 0, 1, 1000, 59957, 4, 1, 11183, 32, 201305, 8356,
                    4, 16000, 100, 16000, 100, 16000, 100, 16000, 100, 16000, 100, 16000, 100, 100,
                    100, 16000, 100, 94375, 32, 132994, 32, 61462, 4, 72010, 178, 0, 1, 22151, 32,
                    91189, 769, 4, 2, 85848, 123203, 7305, -900, 1716, 549, 57, 85848, 0, 1, 1,
                    1000, 42921, 4, 2, 24548, 29498, 38, 1, 898148, 27279, 1, 51775, 558, 1, 39184,
                    1000, 60594, 1, 141895, 32, 83150, 32, 15299, 32, 76049, 1, 13169, 4, 22100,
                    10, 28999, 74, 1, 28999, 74, 1, 43285, 552, 1, 44749, 541, 1, 33852, 32, 68246,
                    32, 72362, 32, 7243, 32, 7391, 32, 11546, 32, 85848, 123203, 7305, -900, 1716,
                    549, 57, 85848, 0, 1, 90434, 519, 0, 1, 74433, 32, 85848, 123203, 7305, -900,
                    1716, 549, 57, 85848, 0, 1, 1, 85848, 123203, 7305, -900, 1716, 549, 57, 85848,
                    0, 1, 955506, 213312, 0, 2, 270652, 22588, 4, 1457325, 64566, 4, 20467, 1, 4,
                    0, 141992, 32, 100788, 420, 1, 1, 81663, 32, 59498, 32, 20142, 32, 24588, 32,
                    20744, 32, 25933, 32, 24623, 32, 43053543, 10, 53384111, 14333, 10, 43574283,
                    26308, 10, 16000, 100, 16000, 100, 962335, 18, 2780678, 6, 442008, 1, 52538055,
                    3756, 18, 267929, 18, 76433006, 8868, 18, 52948122, 18, 1995836, 36, 3227919,
                    12, 901022, 1, 166917843, 4307, 36, 284546, 36, 158221314, 26549, 36, 74698472,
                    36, 333849714, 1, 254006273, 72, 2174038, 72, 2261318, 64571, 4, 207616, 8310,
                    4, 1293828, 28716, 63, 0, 1, 1006041, 43623, 251, 0, 1, 100181, 726, 719, 0, 1,
                    100181, 726, 719, 0, 1, 100181, 726, 719, 0, 1, 107878, 680, 0, 1, 95336, 1,
                    281145, 18848, 0, 1, 180194, 159, 1, 1, 158519, 8942, 0, 1, 159378, 8813, 0, 1,
                    107490, 3298, 1, 106057, 655, 1, 1964219, 24520, 3,
                ]),
                unknown: BTreeMap::default(),
            },
            execution_costs: pallas_primitives::ExUnitPrices {
                mem_price: RationalNumber {
                    numerator: 577,
                    denominator: 10000,
                },
                step_price: RationalNumber {
                    numerator: 721,
                    denominator: 10000000,
                },
            },
            max_tx_ex_units: ExUnits {
                mem: 14000000,
                steps: 10000000000,
            },
            max_block_ex_units: ExUnits {
                mem: 62000000,
                steps: 40000000000,
            },
            max_value_size: 5000,
            collateral_percentage: 150,
            max_collateral_inputs: 3,
            pool_voting_thresholds: PoolVotingThresholds {
                motion_no_confidence: RationalNumber {
                    numerator: 0,
                    denominator: 1,
                },
                committee_normal: RationalNumber {
                    numerator: 0,
                    denominator: 1,
                },
                committee_no_confidence: RationalNumber {
                    numerator: 0,
                    denominator: 1,
                },
                hard_fork_initiation: RationalNumber {
                    numerator: 0,
                    denominator: 1,
                },
                security_voting_threshold: RationalNumber {
                    numerator: 0,
                    denominator: 1,
                },
            },
            drep_voting_thresholds: DRepVotingThresholds {
                motion_no_confidence: RationalNumber {
                    numerator: 0,
                    denominator: 1,
                },
                committee_normal: RationalNumber {
                    numerator: 0,
                    denominator: 1,
                },
                committee_no_confidence: RationalNumber {
                    numerator: 0,
                    denominator: 1,
                },
                update_constitution: RationalNumber {
                    numerator: 0,
                    denominator: 1,
                },
                hard_fork_initiation: RationalNumber {
                    numerator: 0,
                    denominator: 1,
                },
                pp_network_group: RationalNumber {
                    numerator: 0,
                    denominator: 1,
                },
                pp_economic_group: RationalNumber {
                    numerator: 0,
                    denominator: 1,
                },
                pp_technical_group: RationalNumber {
                    numerator: 0,
                    denominator: 1,
                },
                pp_governance_group: RationalNumber {
                    numerator: 0,
                    denominator: 1,
                },
                treasury_withdrawal: RationalNumber {
                    numerator: 0,
                    denominator: 1,
                },
            },
            min_committee_size: 0,
            committee_term_limit: 0,
            governance_action_validity_period: 0,
            governance_action_deposit: 0,
            drep_deposit: 0,
            drep_inactivity_period: 0,
            minfee_refscript_cost_per_byte: RationalNumber {
                numerator: 0,
                denominator: 1,
            },
        }
    }

    fn mk_preview_params_epoch_380() -> ConwayProtParams {
        ConwayProtParams {
            system_start: chrono::DateTime::parse_from_rfc3339("2022-10-25T00:00:00Z").unwrap(),
            epoch_length: 432000,
            slot_length: 1,
            minfee_a: 44,
            minfee_b: 155381,
            max_block_body_size: 90112,
            max_transaction_size: 16384,
            max_block_header_size: 1100,
            key_deposit: 2000000,
            pool_deposit: 500000000,
            maximum_epoch: 18,
            desired_number_of_stake_pools: 500,
            pool_pledge_influence: RationalNumber {
                numerator: 3,
                denominator: 10,
            },
            expansion_rate: RationalNumber {
                numerator: 3,
                denominator: 1000,
            },
            treasury_growth_rate: RationalNumber {
                numerator: 2,
                denominator: 10,
            },
            protocol_version: (8, 0),
            min_pool_cost: 340000000,
            ada_per_utxo_byte: 4310,
            cost_models_for_script_languages: CostModels {
                plutus_v1: Some(vec![
                    205665, 812, 1, 1, 1000, 571, 0, 1, 1000, 24177, 4, 1, 1000, 32, 117366, 10475,
                    4, 23000, 100, 23000, 100, 23000, 100, 23000, 100, 23000, 100, 23000, 100, 100,
                    100, 23000, 100, 19537, 32, 175354, 32, 46417, 4, 221973, 511, 0, 1, 89141, 32,
                    497525, 14068, 4, 2, 196500, 453240, 220, 0, 1, 1, 1000, 28662, 4, 2, 245000,
                    216773, 62, 1060367, 12586, 1, 208512, 421, 1, 187000, 1000, 52998, 1, 80436,
                    32, 43249, 32, 1000, 32, 80556, 1, 57667, 4, 1000, 10, 197145, 156, 1, 197145,
                    156, 1, 204924, 473, 1, 208896, 511, 1, 52467, 32, 64832, 32, 65493, 32, 22558,
                    32, 16563, 32, 76511, 32, 196500, 453240, 220, 0, 1, 1, 69522, 11687, 0, 1,
                    60091, 32, 196500, 453240, 220, 0, 1, 1, 196500, 453240, 220, 0, 1, 1, 806990,
                    30482, 4, 1927926, 82523, 4, 265318, 0, 4, 0, 85931, 32, 205665, 812, 1, 1,
                    41182, 32, 212342, 32, 31220, 32, 32696, 32, 43357, 32, 32247, 32, 38314, 32,
                    9462713, 1021, 10,
                ]),

                plutus_v2: Some(vec![
                    205665, 812, 1, 1, 1000, 571, 0, 1, 1000, 24177, 4, 1, 1000, 32, 117366, 10475,
                    4, 23000, 100, 23000, 100, 23000, 100, 23000, 100, 23000, 100, 23000, 100, 100,
                    100, 23000, 100, 19537, 32, 175354, 32, 46417, 4, 221973, 511, 0, 1, 89141, 32,
                    497525, 14068, 4, 2, 196500, 453240, 220, 0, 1, 1, 1000, 28662, 4, 2, 245000,
                    216773, 62, 1, 1060367, 12586, 1, 208512, 421, 1, 187000, 1000, 52998, 1,
                    80436, 32, 43249, 32, 1000, 32, 80556, 1, 57667, 4, 1000, 10, 197145, 156, 1,
                    197145, 156, 1, 204924, 473, 1, 208896, 511, 1, 52467, 32, 64832, 32, 65493,
                    32, 22558, 32, 16563, 32, 76511, 32, 196500, 453240, 220, 0, 1, 1, 69522,
                    11687, 0, 1, 60091, 32, 196500, 453240, 220, 0, 1, 1, 196500, 453240, 220, 0,
                    1, 1, 1159724, 392670, 0, 2, 806990, 30482, 4, 1927926, 82523, 4, 265318, 0, 4,
                    0, 85931, 32, 205665, 812, 1, 1, 41182, 32, 212342, 32, 31220, 32, 32696, 32,
                    43357, 32, 32247, 32, 38314, 32, 35892428, 10, 9462713, 1021, 10, 38887044,
                    32947, 10,
                ]),
                plutus_v3: Some(vec![
                    100788, 420, 1, 1, 1000, 173, 0, 1, 1000, 59957, 4, 1, 11183, 32, 201305, 8356,
                    4, 16000, 100, 16000, 100, 16000, 100, 16000, 100, 16000, 100, 16000, 100, 100,
                    100, 16000, 100, 94375, 32, 132994, 32, 61462, 4, 72010, 178, 0, 1, 22151, 32,
                    91189, 769, 4, 2, 85848, 123203, 7305, -900, 1716, 549, 57, 85848, 0, 1, 1,
                    1000, 42921, 4, 2, 24548, 29498, 38, 1, 898148, 27279, 1, 51775, 558, 1, 39184,
                    1000, 60594, 1, 141895, 32, 83150, 32, 15299, 32, 76049, 1, 13169, 4, 22100,
                    10, 28999, 74, 1, 28999, 74, 1, 43285, 552, 1, 44749, 541, 1, 33852, 32, 68246,
                    32, 72362, 32, 7243, 32, 7391, 32, 11546, 32, 85848, 123203, 7305, -900, 1716,
                    549, 57, 85848, 0, 1, 90434, 519, 0, 1, 74433, 32, 85848, 123203, 7305, -900,
                    1716, 549, 57, 85848, 0, 1, 1, 85848, 123203, 7305, -900, 1716, 549, 57, 85848,
                    0, 1, 955506, 213312, 0, 2, 270652, 22588, 4, 1457325, 64566, 4, 20467, 1, 4,
                    0, 141992, 32, 100788, 420, 1, 1, 81663, 32, 59498, 32, 20142, 32, 24588, 32,
                    20744, 32, 25933, 32, 24623, 32, 43053543, 10, 53384111, 14333, 10, 43574283,
                    26308, 10, 16000, 100, 16000, 100, 962335, 18, 2780678, 6, 442008, 1, 52538055,
                    3756, 18, 267929, 18, 76433006, 8868, 18, 52948122, 18, 1995836, 36, 3227919,
                    12, 901022, 1, 166917843, 4307, 36, 284546, 36, 158221314, 26549, 36, 74698472,
                    36, 333849714, 1, 254006273, 72, 2174038, 72, 2261318, 64571, 4, 207616, 8310,
                    4, 1293828, 28716, 63, 0, 1, 1006041, 43623, 251, 0, 1, 100181, 726, 719, 0, 1,
                    100181, 726, 719, 0, 1, 100181, 726, 719, 0, 1, 107878, 680, 0, 1, 95336, 1,
                    281145, 18848, 0, 1, 180194, 159, 1, 1, 158519, 8942, 0, 1, 159378, 8813, 0, 1,
                    107490, 3298, 1, 106057, 655, 1, 1964219, 24520, 3,
                ]),
                unknown: BTreeMap::default(),
            },
            execution_costs: pallas_primitives::ExUnitPrices {
                mem_price: RationalNumber {
                    numerator: 577,
                    denominator: 10000,
                },
                step_price: RationalNumber {
                    numerator: 721,
                    denominator: 10000000,
                },
            },
            max_tx_ex_units: ExUnits {
                mem: 14000000,
                steps: 10000000000,
            },
            max_block_ex_units: ExUnits {
                mem: 62000000,
                steps: 40000000000,
            },
            max_value_size: 5000,
            collateral_percentage: 150,
            max_collateral_inputs: 3,
            pool_voting_thresholds: PoolVotingThresholds {
                motion_no_confidence: RationalNumber {
                    numerator: 0,
                    denominator: 1,
                },
                committee_normal: RationalNumber {
                    numerator: 0,
                    denominator: 1,
                },
                committee_no_confidence: RationalNumber {
                    numerator: 0,
                    denominator: 1,
                },
                hard_fork_initiation: RationalNumber {
                    numerator: 0,
                    denominator: 1,
                },
                security_voting_threshold: RationalNumber {
                    numerator: 0,
                    denominator: 1,
                },
            },
            drep_voting_thresholds: DRepVotingThresholds {
                motion_no_confidence: RationalNumber {
                    numerator: 0,
                    denominator: 1,
                },
                committee_normal: RationalNumber {
                    numerator: 0,
                    denominator: 1,
                },
                committee_no_confidence: RationalNumber {
                    numerator: 0,
                    denominator: 1,
                },
                update_constitution: RationalNumber {
                    numerator: 0,
                    denominator: 1,
                },
                hard_fork_initiation: RationalNumber {
                    numerator: 0,
                    denominator: 1,
                },
                pp_network_group: RationalNumber {
                    numerator: 0,
                    denominator: 1,
                },
                pp_economic_group: RationalNumber {
                    numerator: 0,
                    denominator: 1,
                },
                pp_technical_group: RationalNumber {
                    numerator: 0,
                    denominator: 1,
                },
                pp_governance_group: RationalNumber {
                    numerator: 0,
                    denominator: 1,
                },
                treasury_withdrawal: RationalNumber {
                    numerator: 0,
                    denominator: 1,
                },
            },
            min_committee_size: 0,
            committee_term_limit: 0,
            governance_action_validity_period: 0,
            governance_action_deposit: 0,
            drep_deposit: 0,
            drep_inactivity_period: 0,
            minfee_refscript_cost_per_byte: RationalNumber {
                numerator: 0,
                denominator: 1,
            },
        }
    }
}
