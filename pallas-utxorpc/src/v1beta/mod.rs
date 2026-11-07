use std::ops::Deref;

use prost_types::FieldMask;

use pallas_primitives::{alonzo, babbage, conway};
use pallas_traverse as trv;
use trv::OriginalHash;

use crate::LedgerContext;

pub use utxorpc_spec::utxorpc::v1beta as spec;

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

crate::shared::impl_cardano_mapper_shared!(utxorpc_spec::utxorpc::v1beta::cardano);

// ---- v1beta-specific bodies for methods that diverge from v1alpha -----------

impl<C: LedgerContext> Mapper<C> {
    // v1beta names this variant ScriptPubkeyHash; v1alpha names it
    // ScriptPubkey. The rest of map_native_script is identical between
    // versions and lives in shared.rs.
    fn map_native_script_pubkey(bytes: Vec<u8>) -> u5c::native_script::NativeScript {
        u5c::native_script::NativeScript::ScriptPubkeyHash(bytes.into())
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
        Some(match x.datum()? {
            babbage::DatumOption::Data(x) => u5c::Datum {
                hash: x.original_hash().to_vec().into(),
                payload: self.map_plutus_datum(&x.0).into(),
                original_cbor: Some(x.raw_cbor().to_vec().into()),
            },
            babbage::DatumOption::Hash(x) => u5c::Datum {
                hash: x.to_vec().into(),
                payload: tx
                    .and_then(|tx| tx.find_plutus_data(&x))
                    .map(|d| self.map_plutus_datum(d)),
                original_cbor: None,
            },
        })
    }

    pub fn map_tx_output(
        &self,
        x: &trv::MultiEraOutput,
        tx: Option<&trv::MultiEraTx>,
    ) -> u5c::TxOutput {
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
                self.map_tx_datum_opt(x, tx)
            } else {
                Some(self.map_tx_datum(x, tx))
            },
            script: self.map_output_script(x),
            original_cbor: Some(x.encode().into()),
        }
    }

    pub fn map_asset(&self, x: &trv::MultiEraAsset) -> u5c::Asset {
        let quantity = if let Some(v) = x.output_coin() {
            u64_to_bigint(v)
        } else if let Some(v) = x.mint_coin() {
            i64_to_bigint(v)
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
        }
    }

    pub fn map_conway_gov_action(&self, x: &conway::GovAction) -> u5c::GovernanceAction {
        let inner = match x {
            conway::GovAction::ParameterChange(gov_id, params, script) => {
                u5c::governance_action::GovernanceAction::ParameterChangeAction(
                    u5c::ParameterChangeAction {
                        gov_action_id: self.map_gov_action_id(gov_id),
                        protocol_param_update: Some(self.map_conway_pparams_update(params)),
                        policy_hash: match script {
                            Some(x) => x.to_vec().into(),
                            _ => Default::default(),
                        },
                    },
                )
            }
            conway::GovAction::HardForkInitiation(gov_id, version) => {
                u5c::governance_action::GovernanceAction::HardForkInitiationAction(
                    u5c::HardForkInitiationAction {
                        gov_action_id: self.map_gov_action_id(gov_id),
                        protocol_version: Some(u5c::ProtocolVersion {
                            major: version.0 as u32,
                            minor: version.1 as u32,
                        }),
                    },
                )
            }
            conway::GovAction::TreasuryWithdrawals(withdrawals, script) => {
                u5c::governance_action::GovernanceAction::TreasuryWithdrawalsAction(
                    u5c::TreasuryWithdrawalsAction {
                        withdrawals: withdrawals
                            .iter()
                            .map(|(k, v)| u5c::WithdrawalAmount {
                                reward_account: k.to_vec().into(),
                                coin: u64_to_bigint(*v),
                            })
                            .collect(),
                        policy_hash: match script {
                            Some(x) => x.to_vec().into(),
                            _ => Default::default(),
                        },
                    },
                )
            }
            conway::GovAction::NoConfidence(gov_id) => {
                u5c::governance_action::GovernanceAction::NoConfidenceAction(
                    u5c::NoConfidenceAction {
                        gov_action_id: self.map_gov_action_id(gov_id),
                    },
                )
            }
            conway::GovAction::UpdateCommittee(gov_id, remove, add, threshold) => {
                u5c::governance_action::GovernanceAction::UpdateCommitteeAction(
                    u5c::UpdateCommitteeAction {
                        gov_action_id: self.map_gov_action_id(gov_id),
                        remove_committee_credentials: remove
                            .iter()
                            .map(|x| self.map_stake_credential(x))
                            .collect(),
                        new_committee_credentials: add
                            .iter()
                            .map(|(cred, epoch)| u5c::NewCommitteeCredentials {
                                committee_cold_credential: Some(self.map_stake_credential(cred)),
                                expires_epoch: *epoch as u32,
                            })
                            .collect(),
                        new_committee_threshold: Some(rational_number_to_u5c(threshold.clone())),
                    },
                )
            }
            conway::GovAction::NewConstitution(gov_id, constitution) => {
                u5c::governance_action::GovernanceAction::NewConstitutionAction(
                    u5c::NewConstitutionAction {
                        gov_action_id: self.map_gov_action_id(gov_id),
                        constitution: Some(u5c::Constitution {
                            anchor: Some(u5c::Anchor {
                                url: constitution.anchor.url.clone(),
                                content_hash: constitution.anchor.content_hash.to_vec().into(),
                            }),
                            hash: match constitution.guardrail_script {
                                Some(x) => x.to_vec().into(),
                                _ => Default::default(),
                            },
                        }),
                    },
                )
            }
            conway::GovAction::Information => {
                u5c::governance_action::GovernanceAction::InfoAction(u5c::InfoAction {})
            }
        };

        u5c::GovernanceAction {
            governance_action: Some(inner),
        }
    }

    pub fn map_tx(&self, tx: &trv::MultiEraTx) -> u5c::Tx {
        let resolved = self.ledger.as_ref().and_then(|ctx| {
            let to_resolve = self.find_related_inputs(tx);
            ctx.get_utxos(to_resolve.as_slice())
        });

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
                .map(|x| self.map_tx_output(x, Some(tx)))
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
            votes: self.map_votes(tx),
            withdrawals: tx
                .withdrawals_sorted_set()
                .iter()
                .enumerate()
                .map(|(order, x)| self.map_withdrawals(x, tx, order as u32))
                .collect(),
            mint: tx
                .mints_sorted_set()
                .iter()
                .map(|x| self.map_policy_assets(x))
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
                redeemers: tx
                    .redeemers()
                    .iter()
                    .map(|x| self.map_redeemer(x))
                    .collect(),
                bootstrap_witnesses: tx
                    .bootstrap_witnesses()
                    .iter()
                    .map(|x| self.map_bootstrap_witness(x))
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

    // ---- v1beta-only types (no v1alpha counterpart) -------------------------

    pub fn map_bootstrap_witness(&self, x: &alonzo::BootstrapWitness) -> u5c::BootstrapWitness {
        u5c::BootstrapWitness {
            vkey: x.public_key.to_vec().into(),
            signature: x.signature.to_vec().into(),
            chain_code: x.chain_code.to_vec().into(),
            attributes: x.attributes.to_vec().into(),
        }
    }

    pub fn map_vote(&self, x: &conway::Vote) -> u5c::Vote {
        match x {
            conway::Vote::No => u5c::Vote::No,
            conway::Vote::Yes => u5c::Vote::Yes,
            conway::Vote::Abstain => u5c::Vote::Abstain,
        }
    }

    pub fn map_voting_procedure(
        &self,
        gov_action_id: &conway::GovActionId,
        x: &conway::VotingProcedure,
    ) -> u5c::VotingProcedure {
        u5c::VotingProcedure {
            gov_action_id: Some(u5c::GovernanceActionId {
                transaction_id: gov_action_id.transaction_id.to_vec().into(),
                governance_action_index: gov_action_id.action_index,
            }),
            vote: self.map_vote(&x.vote) as i32,
            anchor: x.anchor.as_ref().map(|a| u5c::Anchor {
                url: a.url.clone(),
                content_hash: a.content_hash.to_vec().into(),
            }),
        }
    }

    fn map_voter(&self, voter: &conway::Voter) -> u5c::voter_votes::Voter {
        match voter {
            conway::Voter::ConstitutionalCommitteeKey(hash) => {
                u5c::voter_votes::Voter::ConstitutionalCommittee(u5c::StakeCredential {
                    stake_credential: u5c::stake_credential::StakeCredential::AddrKeyHash(
                        hash.to_vec().into(),
                    )
                    .into(),
                })
            }
            conway::Voter::ConstitutionalCommitteeScript(hash) => {
                u5c::voter_votes::Voter::ConstitutionalCommittee(u5c::StakeCredential {
                    stake_credential: u5c::stake_credential::StakeCredential::ScriptHash(
                        hash.to_vec().into(),
                    )
                    .into(),
                })
            }
            conway::Voter::DRepKey(hash) => u5c::voter_votes::Voter::Drep(u5c::StakeCredential {
                stake_credential: u5c::stake_credential::StakeCredential::AddrKeyHash(
                    hash.to_vec().into(),
                )
                .into(),
            }),
            conway::Voter::DRepScript(hash) => {
                u5c::voter_votes::Voter::Drep(u5c::StakeCredential {
                    stake_credential: u5c::stake_credential::StakeCredential::ScriptHash(
                        hash.to_vec().into(),
                    )
                    .into(),
                })
            }
            conway::Voter::StakePoolKey(hash) => u5c::voter_votes::Voter::Spo(hash.to_vec().into()),
        }
    }

    /// Maps the per-tx voting procedures into the v1beta `votes` field.
    pub fn map_votes(&self, tx: &trv::MultiEraTx) -> Vec<u5c::VoterVotes> {
        let Some(procedures) = tx.voting_procedures() else {
            return Vec::new();
        };

        procedures
            .iter()
            .map(|(voter, ballots)| u5c::VoterVotes {
                voter: Some(self.map_voter(voter)),
                votes: ballots
                    .iter()
                    .map(|(gov_id, procedure)| self.map_voting_procedure(gov_id, procedure))
                    .collect(),
            })
            .collect()
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
    fn snapshot() {
        let test_blocks = [include_str!("../../../test_data/u5c1.block")];
        let test_snapshots = [include_str!("../../../test_data/u5c_v1beta.json")];

        let mapper = Mapper::new(NoLedger);

        for (block_str, json_str) in test_blocks.iter().zip(test_snapshots) {
            let cbor = hex::decode(block_str).unwrap();
            let block = pallas_traverse::MultiEraBlock::decode(&cbor).unwrap();
            let current = serde_json::json!(mapper.map_block(&block));

            // Set REGENERATE_SNAPSHOTS=1 to overwrite the snapshot file in place.
            if std::env::var("REGENERATE_SNAPSHOTS").is_ok() {
                let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../test_data/u5c_v1beta.json");
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
            Some(vec![0x43, 0xd8, 0x79, 0x80].into())
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
        assert_eq!(resolved.original_cbor, None);

        assert_eq!(unresolved.hash, resolved.hash);
        assert_eq!(unresolved.payload, None);
        assert_eq!(unresolved.original_cbor, None);
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
                    Some(u5c::native_script::NativeScript::ScriptPubkeyHash(_))
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
            Some(u5c::native_script::NativeScript::ScriptPubkeyHash(ref b)) if b.as_ref() == [1; 28]
        ));
        assert!(matches!(
            n_of_k.scripts[2].native_script,
            Some(u5c::native_script::NativeScript::ScriptPubkeyHash(ref b)) if b.as_ref() == [3; 28]
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
            Some(u5c::native_script::NativeScript::ScriptPubkeyHash(ref b)) if b.as_ref() == [2; 28]
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

    #[test]
    #[cfg(feature = "unstable")]
    fn dijkstra_votes_map_as_the_same_conway_votes_do() {
        use pallas_primitives::{conway, dijkstra};

        let procedures: conway::VotingProcedures = [(
            conway::Voter::DRepKey([5; 28].into()),
            [(
                conway::GovActionId {
                    transaction_id: [6; 32].into(),
                    action_index: 0,
                },
                conway::VotingProcedure {
                    vote: conway::Vote::Yes,
                    anchor: None,
                },
            )]
            .into_iter()
            .collect(),
        )]
        .into_iter()
        .collect();

        let conway_cbor = hex::decode(include_str!("../../../test_data/conway1.tx")).unwrap();
        let mut conway_tx: conway::Tx = pallas_codec::minicbor::decode(&conway_cbor).unwrap();
        conway_tx.transaction_body.voting_procedures = Some(procedures.clone());

        let block_cbor = hex::decode(include_str!("../../../test_data/dijkstra3.block")).unwrap();
        let block = trv::MultiEraBlock::decode(&block_cbor).unwrap();
        let txs = block.txs();
        let mut dijkstra_tx: dijkstra::BlockTransaction = txs[0]
            .as_dijkstra()
            .expect("dijkstra3 carries a Dijkstra transaction")
            .clone();
        dijkstra_tx.transaction_body.voting_procedures = Some(procedures);

        let mapper = Mapper::new(NoLedger);
        let from_conway = mapper.map_votes(&trv::MultiEraTx::from_conway(&conway_tx));

        assert_eq!(from_conway.len(), 1, "one voter casts one vote");
        assert_eq!(
            mapper.map_votes(&trv::MultiEraTx::from_dijkstra(&dijkstra_tx)),
            from_conway
        );
    }
}
