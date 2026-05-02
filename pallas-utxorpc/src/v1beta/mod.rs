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
}

impl<C: LedgerContext> Mapper<C> {
    pub fn new(ledger: C) -> Self {
        Self {
            ledger: Some(ledger),
            _mask: FieldMask { paths: vec![] },
        }
    }

    /// Creates a clone of this mapper using a custom field mask
    pub fn masked(&self, mask: FieldMask) -> Self {
        Self {
            ledger: self.ledger.clone(),
            _mask: mask,
        }
    }
}

crate::shared::impl_cardano_mapper_shared!(utxorpc_spec::utxorpc::v1beta::cardano);

// ---- v1beta-specific bodies for methods that diverge from v1alpha -----------

impl<C: LedgerContext> Mapper<C> {
    pub fn map_native_script(
        x: &pallas_primitives::alonzo::NativeScript,
    ) -> u5c::NativeScript {
        let inner = match x {
            babbage::NativeScript::ScriptPubkey(x) => {
                u5c::native_script::NativeScript::ScriptPubkeyHash(x.to_vec().into())
            }
            babbage::NativeScript::ScriptAll(x) => {
                u5c::native_script::NativeScript::ScriptAll(u5c::NativeScriptList {
                    items: x.iter().map(|x| Self::map_native_script(x)).collect(),
                })
            }
            babbage::NativeScript::ScriptAny(x) => {
                u5c::native_script::NativeScript::ScriptAll(u5c::NativeScriptList {
                    items: x.iter().map(|x| Self::map_native_script(x)).collect(),
                })
            }
            babbage::NativeScript::ScriptNOfK(n, k) => {
                u5c::native_script::NativeScript::ScriptNOfK(u5c::ScriptNOfK {
                    k: *n,
                    scripts: k.iter().map(|x| Self::map_native_script(x)).collect(),
                })
            }
            babbage::NativeScript::InvalidBefore(s) => {
                u5c::native_script::NativeScript::InvalidBefore(*s)
            }
            babbage::NativeScript::InvalidHereafter(s) => {
                u5c::native_script::NativeScript::InvalidHereafter(*s)
            }
        };

        u5c::NativeScript {
            native_script: inner.into(),
        }
    }

    pub fn map_tx_datum(
        &self,
        x: &trv::MultiEraOutput,
        tx: Option<&trv::MultiEraTx>,
    ) -> u5c::Datum {
        u5c::Datum {
            hash: match x.datum() {
                Some(babbage::DatumOption::Data(x)) => x.original_hash().to_vec().into(),
                Some(babbage::DatumOption::Hash(x)) => x.to_vec().into(),
                _ => vec![].into(),
            },
            payload: match x.datum() {
                Some(babbage::DatumOption::Data(x)) => self.map_plutus_datum(&x.0).into(),
                Some(babbage::DatumOption::Hash(x)) => tx
                    .and_then(|tx| tx.find_plutus_data(&x))
                    .map(|d| self.map_plutus_datum(d)),
                _ => None,
            },
            original_cbor: match x.datum() {
                Some(babbage::DatumOption::Data(x)) => Some(x.raw_cbor().to_vec().into()),
                _ => None,
            },
        }
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
            datum: self.map_tx_datum(x, tx).into(),
            script: x.script_ref().map(|x| self.map_any_script(&x)),
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
            conway::Voter::DRepScript(hash) => u5c::voter_votes::Voter::Drep(u5c::StakeCredential {
                stake_credential: u5c::stake_credential::StakeCredential::ScriptHash(
                    hash.to_vec().into(),
                )
                .into(),
            }),
            conway::Voter::StakePoolKey(hash) => u5c::voter_votes::Voter::Spo(hash.to_vec().into()),
        }
    }

    /// Maps the per-tx voting procedures (Conway only) into the v1beta `votes` field.
    pub fn map_votes(&self, tx: &trv::MultiEraTx) -> Vec<u5c::VoterVotes> {
        let Some(conway_tx) = tx.as_conway() else {
            return Vec::new();
        };

        let Some(procedures) = conway_tx.transaction_body.voting_procedures.as_ref() else {
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
}
