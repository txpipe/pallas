use std::ops::Deref;

use prost_types::FieldMask;

use pallas_primitives::babbage;
use pallas_traverse as trv;
use trv::OriginalHash;

use crate::LedgerContext;

pub use utxorpc_spec::utxorpc::v1alpha as spec;

#[derive(Default, Clone)]
pub struct Mapper<C: LedgerContext> {
    pub(crate) ledger: Option<C>,
    pub(crate) _mask: FieldMask,
    pub(crate) omit_absent_datums: bool,
}

impl<C: LedgerContext> Mapper<C> {
    pub fn new(ledger: C) -> Self {
        Self {
            ledger: Some(ledger),
            _mask: FieldMask { paths: vec![] },
            omit_absent_datums: false,
        }
    }

    /// Creates a clone of this mapper using a custom field mask
    pub fn masked(&self, mask: FieldMask) -> Self {
        Self {
            ledger: self.ledger.clone(),
            _mask: mask,
            omit_absent_datums: self.omit_absent_datums,
        }
    }

    /// Leaves `TxOutput.datum` unset on outputs with no datum.
    ///
    /// By default those outputs carry an empty `Datum` message. This applies to
    /// every output this mapper produces, including those inside a mapped
    /// transaction or block.
    pub fn omit_absent_datums(mut self) -> Self {
        self.omit_absent_datums = true;
        self
    }
}

crate::shared::impl_cardano_mapper_shared!(utxorpc_spec::utxorpc::v1alpha::cardano);
crate::before_v1::impl_cardano_mapper_before_v1!();

// ---- v1alpha-specific bodies for methods that diverge from v1beta -----------

/// The u5c member of an information action, which this schema holds as the uint32 placeholder 6.
fn information_action() -> u5c::governance_action::GovernanceAction {
    u5c::governance_action::GovernanceAction::InfoAction(6)
}

impl<C: LedgerContext> Mapper<C> {
    // v1alpha names this variant by what it holds; v1beta names it
    // ScriptPubkeyHash. The rest of map_native_script is identical between
    // versions and lives in shared.rs.
    fn map_native_script_pubkey(bytes: Vec<u8>) -> u5c::native_script::NativeScript {
        u5c::native_script::NativeScript::ScriptPubkey(bytes.into())
    }

    /// Maps the datum of an output, or an empty message when the output has none.
    pub fn map_tx_datum(
        &self,
        x: &trv::MultiEraOutput,
        tx: Option<&trv::MultiEraTx>,
    ) -> u5c::Datum {
        self.map_tx_datum_opt(x, tx).unwrap_or_default()
    }

    /// Maps the datum of an output, or `None` when the output has none.
    pub fn map_tx_datum_opt(
        &self,
        x: &trv::MultiEraOutput,
        tx: Option<&trv::MultiEraTx>,
    ) -> Option<u5c::Datum> {
        self.map_tx_datum_from(x, tx.as_slice())
    }

    /// Maps the datum of an output, looking a datum hash up in the witnesses of `txs`.
    fn map_tx_datum_from(
        &self,
        x: &trv::MultiEraOutput,
        txs: &[&trv::MultiEraTx],
    ) -> Option<u5c::Datum> {
        Some(match x.datum()? {
            babbage::DatumOption::Data(x) => u5c::Datum {
                hash: x.original_hash().to_vec().into(),
                payload: self.map_plutus_datum(&x.0).into(),
                original_cbor: x.raw_cbor().to_vec().into(),
            },
            babbage::DatumOption::Hash(x) => u5c::Datum {
                hash: x.to_vec().into(),
                payload: txs
                    .iter()
                    .find_map(|tx| tx.find_plutus_data(&x))
                    .map(|d| self.map_plutus_datum(d)),
                original_cbor: vec![].into(),
            },
        })
    }

    pub fn map_tx_output(
        &self,
        x: &trv::MultiEraOutput,
        tx: Option<&trv::MultiEraTx>,
    ) -> u5c::TxOutput {
        self.map_tx_output_from(x, tx.as_slice())
    }

    /// Maps an output, looking its datum hash up in the witnesses of `txs`.
    fn map_tx_output_from(
        &self,
        x: &trv::MultiEraOutput,
        txs: &[&trv::MultiEraTx],
    ) -> u5c::TxOutput {
        let datum = self.map_tx_datum_from(x, txs);

        u5c::TxOutput {
            address: x.address().map(|a| a.to_vec()).unwrap_or_default().into(),
            coin: u64_to_bigint(x.value().coin()),
            // TODO: this is wrong, we're crating a new item for each asset even if they share
            // the same policy id. We need to adjust Pallas' interface to make this mapping more
            // ergonomic.
            assets: x
                .value()
                .assets()
                .iter()
                .map(|x| self.map_policy_assets(x))
                .collect(),
            datum: if self.omit_absent_datums {
                datum
            } else {
                Some(datum.unwrap_or_default())
            },
            script: self.map_output_script(x),
        }
    }

    pub fn map_asset(&self, x: &trv::MultiEraAsset) -> u5c::Asset {
        let quantity = if let Some(v) = x.output_coin() {
            u64_to_bigint(v).map(u5c::asset::Quantity::OutputCoin)
        } else if let Some(v) = x.mint_coin() {
            i64_to_bigint(v).map(u5c::asset::Quantity::MintCoin)
        } else {
            None
        };
        u5c::Asset {
            name: x.name().to_vec().into(),
            quantity,
        }
    }

    pub fn map_policy_assets(&self, x: &trv::MultiEraPolicyAssets) -> u5c::Multiasset {
        u5c::Multiasset {
            policy_id: x.policy().to_vec().into(),
            assets: x.assets().iter().map(|x| self.map_asset(x)).collect(),
            redeemer: None,
        }
    }

    pub fn map_tx(&self, tx: &trv::MultiEraTx) -> u5c::Tx {
        self.map_tx_with_top(tx, None)
    }

    /// Maps the sub transaction `sub` of the top level transaction `top`.
    #[cfg(feature = "unstable")]
    pub fn map_sub_tx<'b>(&self, sub: &trv::MultiEraTx<'b>, top: &trv::MultiEraTx<'b>) -> u5c::Tx {
        self.map_tx_with_top(sub, Some(top))
    }

    /// Maps a transaction, given its top level transaction when it is a sub transaction.
    fn map_tx_with_top<'b>(
        &self,
        tx: &trv::MultiEraTx<'b>,
        top: Option<&trv::MultiEraTx<'b>>,
    ) -> u5c::Tx {
        let resolved = self.ledger.as_ref().and_then(|ctx| {
            let to_resolve = self.find_related_inputs(tx);
            ctx.get_utxos(to_resolve.as_slice())
        });
        // The ledger accepts the datum for a sub transaction's output hash from the top level witnesses too.
        let datums: Vec<_> = std::iter::once(tx).chain(top).collect();

        u5c::Tx {
            hash: tx.hash().to_vec().into(),
            inputs: tx
                .inputs_sorted_set()
                .iter()
                .enumerate()
                .map(|(order, i)| self.map_tx_input(i, tx, order as u32, &resolved))
                .collect(),
            outputs: tx
                .outputs()
                .iter()
                .map(|x| self.map_tx_output_from(x, &datums))
                .collect(),
            certificates: tx
                .certs()
                .iter()
                .enumerate()
                .filter_map(|(order, x)| self.map_cert(x, tx, order as u32))
                .collect(),
            proposals: tx
                .gov_proposals()
                .iter()
                .map(|x| self.map_gov_proposal(x))
                .collect(),
            withdrawals: tx
                .withdrawals_sorted_set()
                .iter()
                .enumerate()
                .map(|(order, x)| self.map_withdrawals(x, tx, order as u32))
                .collect(),
            mint: tx
                .mints_sorted_set()
                .iter()
                .enumerate()
                .map(|(order, x)| {
                    let mut ma = self.map_policy_assets(x);

                    ma.redeemer = tx
                        .find_mint_redeemer(order as u32)
                        .map(|r| self.map_redeemer(&r));

                    ma
                })
                .collect(),
            reference_inputs: tx
                .reference_inputs()
                .iter()
                .map(|x| self.map_tx_reference_input(x, &resolved, tx))
                .collect(),
            witnesses: u5c::WitnessSet {
                vkeywitness: tx
                    .vkey_witnesses()
                    .iter()
                    .map(|x| self.map_vkey_witness(x))
                    .collect(),
                script: self.collect_all_scripts(tx),
                plutus_datums: tx
                    .plutus_data()
                    .iter()
                    .map(|x| self.map_plutus_datum(x.deref()))
                    .collect(),
            }
            .into(),
            collateral: u5c::Collateral {
                collateral: tx
                    .collateral()
                    .iter()
                    .map(|x| self.map_tx_collateral(x, &resolved, tx))
                    .collect(),
                collateral_return: tx
                    .collateral_return()
                    .map(|x| self.map_tx_output(&x, Some(tx))),
                total_collateral: u64_to_bigint(tx.total_collateral().unwrap_or_default()),
            }
            .into(),
            fee: u64_to_bigint(tx.fee().unwrap_or_default()),
            validity: u5c::TxValidity {
                start: tx.validity_start().unwrap_or_default(),
                ttl: tx.ttl().unwrap_or_default(),
            }
            .into(),
            successful: tx.is_valid(),
            auxiliary: u5c::AuxData {
                metadata: tx
                    .metadata()
                    .collect::<Vec<_>>()
                    .into_iter()
                    .map(|(l, d)| self.map_metadata(l, d))
                    .collect(),
                scripts: self.collect_all_aux_scripts(tx),
            }
            .into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{TxoRef, UtxoMap};
    use pretty_assertions::assert_eq;

    #[derive(Clone)]
    struct NoLedger;

    impl LedgerContext for NoLedger {
        fn get_utxos(&self, _refs: &[TxoRef]) -> Option<UtxoMap> {
            None
        }

        fn get_slot_timestamp(&self, _slot: u64) -> Option<u64> {
            None
        }
    }

    #[test]
    fn an_information_action_maps_to_the_placeholder_6() {
        let action = pallas_primitives::conway::GovAction::Information;
        let mapped =
            Mapper::new(NoLedger).map_gov_action(&trv::MultiEraGovAction::from_conway(&action));

        assert_eq!(
            mapped.governance_action,
            Some(u5c::governance_action::GovernanceAction::InfoAction(6))
        );
    }

    #[test]
    fn snapshot() {
        let test_blocks = [include_str!("../../../test_data/u5c1.block")];
        let test_snapshots = [include_str!("../../../test_data/u5c_v1alpha.json")];

        let mapper = Mapper::new(NoLedger);

        for (block_str, json_str) in test_blocks.iter().zip(test_snapshots) {
            let cbor = hex::decode(block_str).unwrap();
            let block = pallas_traverse::MultiEraBlock::decode(&cbor).unwrap();
            let current = serde_json::json!(mapper.map_block(&block));

            // Set REGENERATE_SNAPSHOTS=1 to overwrite the snapshot file in place.
            if std::env::var("REGENERATE_SNAPSHOTS").is_ok() {
                let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../test_data/u5c_v1alpha.json");
                std::fs::write(&path, serde_json::to_string_pretty(&current).unwrap()).unwrap();
                eprintln!("regenerated {}", path.display());
                continue;
            }

            let expected: serde_json::Value = serde_json::from_str(json_str).unwrap();

            assert_eq!(expected, current)
        }
    }

    // In u5c1.block, transaction 0 opens with an output holding an inline datum, 1 with one
    // holding no datum, and 2 with a hash its witnesses resolve. No output holds a hash its own
    // witnesses lack; a test gets that shape by pairing an output with another transaction.
    fn datum_fixture() -> Vec<u8> {
        hex::decode(include_str!("../../../test_data/u5c1.block")).unwrap()
    }

    #[test]
    fn an_output_with_no_datum_maps_to_an_empty_message_by_default() {
        let cbor = datum_fixture();
        let block = pallas_traverse::MultiEraBlock::decode(&cbor).unwrap();
        let tx = &block.txs()[1];
        let output = &tx.outputs()[0];
        let mapper = Mapper::new(NoLedger);
        let empty = u5c::Datum::default();

        assert_eq!(mapper.map_tx_datum(output, Some(tx)), empty);
        assert_eq!(mapper.map_tx_datum_opt(output, Some(tx)), None);
        assert_eq!(mapper.map_tx_output(output, Some(tx)).datum, Some(empty));
    }

    #[test]
    fn an_output_with_no_datum_leaves_the_field_unset_when_opted_in() {
        let cbor = datum_fixture();
        let block = pallas_traverse::MultiEraBlock::decode(&cbor).unwrap();
        let tx = &block.txs()[1];
        let output = &tx.outputs()[0];
        let mapper = Mapper::new(NoLedger).omit_absent_datums();

        assert_eq!(mapper.map_tx_datum_opt(output, Some(tx)), None);
        assert_eq!(mapper.map_tx_datum(output, Some(tx)), u5c::Datum::default());
        assert_eq!(mapper.map_tx_output(output, Some(tx)).datum, None);

        let masked = mapper.masked(FieldMask::default());
        assert_eq!(masked.map_tx_output(output, Some(tx)).datum, None);
    }

    #[test]
    fn opting_in_only_unsets_empty_datums_across_a_block() {
        let cbor = datum_fixture();
        let block = pallas_traverse::MultiEraBlock::decode(&cbor).unwrap();
        let mapper = Mapper::new(NoLedger);

        let mut expected = mapper.map_block(&block);
        let mut unset = 0;
        for tx in &mut expected.body.as_mut().unwrap().tx {
            let collateral_return = tx
                .collateral
                .as_mut()
                .and_then(|x| x.collateral_return.as_mut());

            for output in tx.outputs.iter_mut().chain(collateral_return) {
                if output.datum == Some(u5c::Datum::default()) {
                    output.datum = None;
                    unset += 1;
                }
            }
        }

        assert!(unset > 0);
        assert_eq!(expected, mapper.omit_absent_datums().map_block(&block));
    }

    #[test]
    fn a_resolved_input_with_no_datum_follows_the_opt_in() {
        let cbor = datum_fixture();
        let block = pallas_traverse::MultiEraBlock::decode(&cbor).unwrap();
        let tx = &block.txs()[1];
        let output = &tx.outputs()[0];
        let input = &tx.inputs()[0];
        let resolved = Some(UtxoMap::from([(
            (*input.hash(), input.index() as u32),
            (output.era(), output.encode()),
        )]));
        let mapper = Mapper::new(NoLedger);

        let datum = mapper
            .map_tx_input(input, tx, 0, &resolved)
            .as_output
            .unwrap()
            .datum;
        assert_eq!(datum, Some(u5c::Datum::default()));

        let datum = mapper
            .omit_absent_datums()
            .map_tx_input(input, tx, 0, &resolved)
            .as_output
            .unwrap()
            .datum;
        assert_eq!(datum, None);
    }

    #[test]
    fn an_inline_datum_maps_its_hash_payload_and_cbor() {
        let cbor = datum_fixture();
        let block = pallas_traverse::MultiEraBlock::decode(&cbor).unwrap();
        let tx = &block.txs()[0];
        let output = &tx.outputs()[0];

        let datum = Mapper::new(NoLedger)
            .map_tx_output(output, Some(tx))
            .datum
            .unwrap();

        assert_eq!(
            hex::encode(&datum.hash),
            "58260815eb79c0e8168502dbe5c6e73ac0fa37b57fe2b0582f765778c9c63550"
        );
        assert_eq!(
            datum.payload,
            Some(u5c::PlutusData {
                plutus_data: Some(u5c::plutus_data::PlutusData::BoundedBytes(
                    vec![0xd8, 0x79, 0x80].into()
                )),
            })
        );
        assert_eq!(
            datum.original_cbor,
            prost::bytes::Bytes::from(vec![0x43, 0xd8, 0x79, 0x80])
        );
    }

    #[test]
    fn a_datum_hash_maps_with_or_without_a_resolvable_payload() {
        let cbor = datum_fixture();
        let block = pallas_traverse::MultiEraBlock::decode(&cbor).unwrap();
        let tx = &block.txs()[2];
        let output = &tx.outputs()[0];
        let mapper = Mapper::new(NoLedger);

        let resolved = mapper.map_tx_output(output, Some(tx)).datum.unwrap();
        let unresolved = mapper.map_tx_output(output, None).datum.unwrap();

        assert_eq!(
            hex::encode(&resolved.hash),
            "c0b7784ae9fd96e36ae5eae00f1ab5572b540213f1d8e3d727dec251c578ec39"
        );
        assert!(resolved.payload.is_some());
        assert_eq!(resolved.original_cbor, prost::bytes::Bytes::new());

        assert_eq!(unresolved.hash, resolved.hash);
        assert_eq!(unresolved.payload, None);
        assert_eq!(unresolved.original_cbor, prost::bytes::Bytes::new());
    }

    #[test]
    fn a_datum_hash_its_transaction_does_not_witness_keeps_its_hash() {
        let cbor = datum_fixture();
        let block = pallas_traverse::MultiEraBlock::decode(&cbor).unwrap();
        let txs = block.txs();
        let output = &txs[2].outputs()[0];
        let mapper = Mapper::new(NoLedger);
        let expected = mapper.map_tx_output(output, None).datum;

        assert!(expected.as_ref().is_some_and(|x| !x.hash.is_empty()));
        assert_eq!(mapper.map_tx_output(output, Some(&txs[5])).datum, expected);
        assert_eq!(
            mapper
                .omit_absent_datums()
                .map_tx_output(output, Some(&txs[5]))
                .datum,
            expected
        );
    }

    #[test]
    fn negative_n_of_k_threshold_maps_to_zero() {
        let mapped = Mapper::<NoLedger>::map_native_script(
            &pallas_primitives::alonzo::NativeScript::ScriptNOfK(-1, vec![]),
        );
        assert!(matches!(
            mapped.native_script,
            Some(u5c::native_script::NativeScript::ScriptNOfK(
                u5c::ScriptNOfK { k: 0, .. }
            ))
        ));
    }

    #[test]
    #[allow(deprecated)]
    fn the_legacy_purpose_mapper_agrees_with_the_multi_era_one() {
        use pallas_primitives::conway::RedeemerTag;
        use pallas_traverse::MultiEraRedeemerTag;

        let mapper = Mapper::new(NoLedger);
        let tags = [
            RedeemerTag::Spend,
            RedeemerTag::Mint,
            RedeemerTag::Cert,
            RedeemerTag::Reward,
            RedeemerTag::Vote,
            RedeemerTag::Propose,
        ];

        let mut seen = Vec::new();
        for tag in tags {
            let legacy = mapper.map_purpose(&tag);
            assert_eq!(
                legacy,
                mapper.map_multi_era_purpose(&MultiEraRedeemerTag::from(tag))
            );
            seen.push(legacy);
        }

        seen.dedup();
        assert_eq!(seen.len(), 6, "each tag maps to a purpose of its own");
    }

    #[test]
    fn oversized_n_of_k_threshold_maps_to_u32_max() {
        let mapped = Mapper::<NoLedger>::map_native_script(
            &pallas_primitives::alonzo::NativeScript::ScriptNOfK(i64::MAX, vec![]),
        );
        assert!(matches!(
            mapped.native_script,
            Some(u5c::native_script::NativeScript::ScriptNOfK(
                u5c::ScriptNOfK { k: u32::MAX, .. }
            ))
        ));
    }

    #[test]
    fn map_metadatum_handles_deeply_nested_metadata_on_a_small_stack() {
        use pallas_primitives::alonzo::Metadatum;
        use pallas_primitives::{Int, KeyValuePairs};

        std::thread::Builder::new()
            .stack_size(128 * 1024)
            .spawn(|| {
                let depth = 20_000;
                let mut datum = Metadatum::Int(Int::from(0));
                for level in 0..depth {
                    datum = if level % 2 == 0 {
                        Metadatum::Array(vec![datum])
                    } else {
                        Metadatum::Map(KeyValuePairs::Def(vec![(
                            Metadatum::Int(Int::from(1)),
                            datum,
                        )]))
                    };
                }
                // The source Drop still recurses; leak it like the result.
                let datum = std::mem::ManuallyDrop::new(datum);

                let mapped = Mapper::<NoLedger>::map_metadatum(&datum);

                let mut seen = 0;
                let mut cursor = &mapped;
                loop {
                    cursor = match cursor.metadatum.as_ref().expect("mapped node") {
                        u5c::metadatum::Metadatum::Array(list) => list.items.first(),
                        u5c::metadatum::Metadatum::Map(map) => {
                            let pair = map.pairs.first().expect("map carries its pair");
                            assert!(matches!(
                                pair.key.as_ref().and_then(|k| k.metadatum.as_ref()),
                                Some(u5c::metadatum::Metadatum::Int(1))
                            ));
                            pair.value.as_ref()
                        }
                        u5c::metadatum::Metadatum::Int(0) => break,
                        other => panic!("unexpected node {other:?}"),
                    }
                    .expect("container carries a child");
                    seen += 1;
                }
                assert_eq!(seen, depth);

                // u5c's generated type has no custom Drop, so a chain this
                // deep would abort on the way out. Leak it deliberately.
                std::mem::forget(mapped);
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn map_plutus_datum_handles_deeply_nested_data_on_a_small_stack() {
        use pallas_primitives::alonzo::{BigInt, Constr, PlutusData};
        use pallas_primitives::{Int, KeyValuePairs, MaybeIndefArray};

        std::thread::Builder::new()
            .stack_size(128 * 1024)
            .spawn(|| {
                let depth = 21_000;
                let mut datum = PlutusData::BigInt(BigInt::Int(Int::from(0)));
                for level in 0..depth {
                    datum = match level % 3 {
                        0 => PlutusData::Constr(Constr {
                            tag: 121,
                            any_constructor: None,
                            fields: MaybeIndefArray::Def(vec![datum]),
                        }),
                        1 => PlutusData::Map(KeyValuePairs::Def(vec![(
                            PlutusData::BigInt(BigInt::Int(Int::from(1))),
                            datum,
                        )])),
                        _ => PlutusData::Array(MaybeIndefArray::Def(vec![datum])),
                    };
                }
                // The source Drop still recurses; leak it like the result.
                let datum = std::mem::ManuallyDrop::new(datum);

                let mapped = Mapper::new(NoLedger).map_plutus_datum(&datum);

                let mut seen = 0;
                let mut cursor = &mapped;
                loop {
                    cursor = match cursor.plutus_data.as_ref().expect("mapped node") {
                        u5c::plutus_data::PlutusData::Constr(c) => c.fields.first(),
                        u5c::plutus_data::PlutusData::Map(m) => m
                            .pairs
                            .first()
                            .expect("map carries its pair")
                            .value
                            .as_ref(),
                        u5c::plutus_data::PlutusData::Array(a) => a.items.first(),
                        u5c::plutus_data::PlutusData::BigInt(_) => break,
                        other => panic!("unexpected node {other:?}"),
                    }
                    .expect("container carries a child");
                    seen += 1;
                }
                assert_eq!(seen, depth);

                // u5c's generated type has no custom Drop, so a chain this
                // deep would abort on the way out. Leak it deliberately.
                std::mem::forget(mapped);
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn map_native_script_handles_deeply_nested_scripts_on_a_small_stack() {
        // Depth and stack size are load-bearing, not just generous: both must
        // stay far enough apart that the old recursive mapping (one call
        // frame per level) would abort here, or this test stops proving
        // anything the moment either constant drifts.
        std::thread::Builder::new()
            .stack_size(128 * 1024)
            .spawn(|| {
                let mut script =
                    pallas_primitives::alonzo::NativeScript::ScriptPubkey([0; 28].into());
                for _ in 0..20_000 {
                    script = pallas_primitives::alonzo::NativeScript::ScriptAll(vec![script]);
                }

                let mapped = Mapper::<NoLedger>::map_native_script(&script);

                let mut depth = 0;
                let mut cursor = &mapped;
                while let Some(u5c::native_script::NativeScript::ScriptAll(list)) =
                    &cursor.native_script
                {
                    cursor = list.items.first().expect("ScriptAll must carry a child");
                    depth += 1;
                }
                assert_eq!(depth, 20_000);
                assert!(matches!(
                    cursor.native_script,
                    Some(u5c::native_script::NativeScript::ScriptPubkey(_))
                ));

                // u5c's generated type has no custom Drop (unlike the source
                // NativeScript, stack-safe since pallas#802), so dropping a
                // chain this deep would abort the same way the unfixed mapping
                // did. Leak it deliberately: this test is only about the
                // mapping, and the leak is a few MB, thread-local, test-only.
                std::mem::forget(mapped);
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn map_native_script_preserves_mixed_shape_trees() {
        use pallas_primitives::alonzo::NativeScript;

        // Width > 1 at more than one level: the iterative rewrite pairs
        // mapped children with source children positionally, so a bug there
        // would only show up once a node has more than one child.
        let script = NativeScript::ScriptNOfK(
            2,
            vec![
                NativeScript::ScriptPubkey([1; 28].into()),
                NativeScript::ScriptAll(vec![
                    NativeScript::ScriptPubkey([2; 28].into()),
                    NativeScript::ScriptAny(vec![
                        NativeScript::InvalidBefore(100),
                        NativeScript::InvalidHereafter(200),
                    ]),
                ]),
                NativeScript::ScriptPubkey([3; 28].into()),
            ],
        );

        let mapped = Mapper::<NoLedger>::map_native_script(&script);
        let Some(u5c::native_script::NativeScript::ScriptNOfK(n_of_k)) = &mapped.native_script
        else {
            panic!("expected ScriptNOfK, got {:?}", mapped.native_script);
        };
        assert_eq!(n_of_k.k, 2);
        assert_eq!(n_of_k.scripts.len(), 3);

        assert!(matches!(
            n_of_k.scripts[0].native_script,
            Some(u5c::native_script::NativeScript::ScriptPubkey(ref b)) if b.as_ref() == [1; 28]
        ));
        assert!(matches!(
            n_of_k.scripts[2].native_script,
            Some(u5c::native_script::NativeScript::ScriptPubkey(ref b)) if b.as_ref() == [3; 28]
        ));

        let Some(u5c::native_script::NativeScript::ScriptAll(all)) =
            &n_of_k.scripts[1].native_script
        else {
            panic!(
                "expected ScriptAll, got {:?}",
                n_of_k.scripts[1].native_script
            );
        };
        assert_eq!(all.items.len(), 2);
        assert!(matches!(
            all.items[0].native_script,
            Some(u5c::native_script::NativeScript::ScriptPubkey(ref b)) if b.as_ref() == [2; 28]
        ));

        let Some(u5c::native_script::NativeScript::ScriptAny(any)) = &all.items[1].native_script
        else {
            panic!("expected ScriptAny, got {:?}", all.items[1].native_script);
        };
        assert_eq!(any.items.len(), 2);
        assert!(matches!(
            any.items[0].native_script,
            Some(u5c::native_script::NativeScript::InvalidBefore(100))
        ));
        assert!(matches!(
            any.items[1].native_script,
            Some(u5c::native_script::NativeScript::InvalidHereafter(200))
        ));
    }

    /// The datum hash and the datum payload mapped on the first output of a transaction's first sub transaction.
    #[cfg(feature = "unstable")]
    fn sub_output_datum(cbor: &str) -> (Vec<u8>, Option<u5c::PlutusData>) {
        let tx = crate::testing::dijkstra_tx(cbor);
        let sub = &tx.sub_transactions()[0];
        let mapped = Mapper::new(NoLedger).map_sub_tx(sub, &tx);
        let datum = mapped.outputs[0]
            .datum
            .clone()
            .expect("the output carries a datum");

        (datum.hash.to_vec(), datum.payload)
    }

    #[test]
    #[cfg(feature = "unstable")]
    fn a_sub_transaction_output_hash_maps_the_datum_its_top_level_transaction_holds() {
        let cbor = include_str!("../../../test_data/dijkstra-sub-output-top-datum.tx").trim();

        assert_eq!(
            sub_output_datum(cbor),
            (
                hex::decode("45262f40ee42b0f71855608c466ff80dec3728859a2de93ef71e56d9105510a6")
                    .unwrap(),
                Some(u5c::PlutusData {
                    plutus_data: Some(u5c::plutus_data::PlutusData::BigInt(u5c::BigInt {
                        big_int: Some(u5c::big_int::BigInt::Int(43)),
                    })),
                }),
            ),
            "the ledger accepts the datum for a sub transaction's output hash from the top level witnesses"
        );
    }

    #[test]
    #[cfg(feature = "unstable")]
    fn a_sub_transaction_output_hash_the_batch_holds_no_datum_for_maps_the_hash_alone() {
        let cbor = include_str!("../../../test_data/dijkstra-sub-output-top-datum.tx")
            .trim()
            .replacen("04d9010281182b", "04d9010281182a", 1);

        assert_eq!(
            sub_output_datum(&cbor),
            (
                hex::decode("45262f40ee42b0f71855608c466ff80dec3728859a2de93ef71e56d9105510a6")
                    .unwrap(),
                None,
            ),
            "with the top level datum changed from 43 to 42, no datum in the batch hashes to the output's hash"
        );
    }
}
