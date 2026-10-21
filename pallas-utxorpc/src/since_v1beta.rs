/// Emits the parts of `Mapper<C>` that v1beta and the schemas after it share.
macro_rules! impl_cardano_mapper_since_v1beta {
    () => {
        impl<C: $crate::LedgerContext> Mapper<C> {
            fn map_native_script_pubkey(bytes: Vec<u8>) -> u5c::native_script::NativeScript {
                u5c::native_script::NativeScript::ScriptPubkeyHash(bytes.into())
            }

            pub fn map_tx_datum(
                &self,
                x: &pallas_traverse::MultiEraOutput,
                tx: Option<&pallas_traverse::MultiEraTx>,
            ) -> u5c::Datum {
                use pallas_primitives::babbage;
                use pallas_traverse::OriginalHash;

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
                x: &pallas_traverse::MultiEraOutput,
                tx: Option<&pallas_traverse::MultiEraTx>,
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
                    script: self.map_output_script(x),
                    original_cbor: Some(x.encode().into()),
                }
            }

            pub fn map_asset(&self, x: &pallas_traverse::MultiEraAsset) -> u5c::Asset {
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

            pub fn map_policy_assets(
                &self,
                x: &pallas_traverse::MultiEraPolicyAssets,
            ) -> u5c::Multiasset {
                u5c::Multiasset {
                    policy_id: x.policy().to_vec().into(),
                    assets: x.assets().iter().map(|x| self.map_asset(x)).collect(),
                }
            }

            pub fn map_conway_gov_action(
                &self,
                x: &pallas_primitives::conway::GovAction,
            ) -> u5c::GovernanceAction {
                use pallas_primitives::conway;

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
                                        committee_cold_credential: Some(
                                            self.map_stake_credential(cred),
                                        ),
                                        expires_epoch: *epoch as u32,
                                    })
                                    .collect(),
                                new_committee_threshold: Some(rational_number_to_u5c(
                                    threshold.clone(),
                                )),
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
                                        content_hash: constitution
                                            .anchor
                                            .content_hash
                                            .to_vec()
                                            .into(),
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

            pub fn map_bootstrap_witness(
                &self,
                x: &pallas_primitives::alonzo::BootstrapWitness,
            ) -> u5c::BootstrapWitness {
                u5c::BootstrapWitness {
                    vkey: x.public_key.to_vec().into(),
                    signature: x.signature.to_vec().into(),
                    chain_code: x.chain_code.to_vec().into(),
                    attributes: x.attributes.to_vec().into(),
                }
            }

            pub fn map_vote(&self, x: &pallas_primitives::conway::Vote) -> u5c::Vote {
                use pallas_primitives::conway;

                match x {
                    conway::Vote::No => u5c::Vote::No,
                    conway::Vote::Yes => u5c::Vote::Yes,
                    conway::Vote::Abstain => u5c::Vote::Abstain,
                }
            }

            pub fn map_voting_procedure(
                &self,
                gov_action_id: &pallas_primitives::conway::GovActionId,
                x: &pallas_primitives::conway::VotingProcedure,
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

            fn map_voter(
                &self,
                voter: &pallas_primitives::conway::Voter,
            ) -> u5c::voter_votes::Voter {
                use pallas_primitives::conway;

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
                    conway::Voter::DRepKey(hash) => {
                        u5c::voter_votes::Voter::Drep(u5c::StakeCredential {
                            stake_credential: u5c::stake_credential::StakeCredential::AddrKeyHash(
                                hash.to_vec().into(),
                            )
                            .into(),
                        })
                    }
                    conway::Voter::DRepScript(hash) => {
                        u5c::voter_votes::Voter::Drep(u5c::StakeCredential {
                            stake_credential: u5c::stake_credential::StakeCredential::ScriptHash(
                                hash.to_vec().into(),
                            )
                            .into(),
                        })
                    }
                    conway::Voter::StakePoolKey(hash) => {
                        u5c::voter_votes::Voter::Spo(hash.to_vec().into())
                    }
                }
            }

            /// Maps the per-tx voting procedures into the `votes` field.
            pub fn map_votes(&self, tx: &pallas_traverse::MultiEraTx) -> Vec<u5c::VoterVotes> {
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
    };
}

pub(crate) use impl_cardano_mapper_since_v1beta;
