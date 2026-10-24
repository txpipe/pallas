/// Emits the parts of `Mapper<C>` that every schema version shares, for the schema module given.
macro_rules! impl_cardano_mapper_shared {
    ($u5c:path) => {
        use $u5c as u5c;

        fn rational_number_to_u5c(value: pallas_primitives::RationalNumber) -> u5c::RationalNumber {
            u5c::RationalNumber {
                numerator: value.numerator as i32,
                denominator: value.denominator as u32,
            }
        }

        fn u64_to_bigint(value: u64) -> Option<u5c::BigInt> {
            if value <= i64::MAX as u64 {
                Some(u5c::BigInt {
                    big_int: Some(u5c::big_int::BigInt::Int(value as i64)),
                })
            } else {
                Some(u5c::BigInt {
                    big_int: Some(u5c::big_int::BigInt::BigUInt(
                        value.to_be_bytes().to_vec().into(),
                    )),
                })
            }
        }

        fn i64_to_bigint(value: i64) -> Option<u5c::BigInt> {
            Some(u5c::BigInt {
                big_int: Some(u5c::big_int::BigInt::Int(value)),
            })
        }

        fn execution_prices_to_u5c(value: pallas_primitives::ExUnitPrices) -> u5c::ExPrices {
            u5c::ExPrices {
                steps: Some(rational_number_to_u5c(value.step_price)),
                memory: Some(rational_number_to_u5c(value.mem_price)),
            }
        }

        fn execution_units_to_u5c(value: pallas_primitives::ExUnits) -> u5c::ExUnits {
            u5c::ExUnits {
                memory: value.mem,
                steps: value.steps,
            }
        }

        /// Map an anchor, whose type every era carrying one shares.
        fn map_anchor(x: &pallas_primitives::conway::Anchor) -> u5c::Anchor {
            u5c::Anchor {
                url: x.url.clone(),
                content_hash: x.content_hash.to_vec().into(),
            }
        }

        /// Map the metadata a pool registration points at, whose type every era
        /// carrying one shares.
        fn map_pool_metadata(x: &pallas_primitives::PoolMetadata) -> u5c::PoolMetadata {
            u5c::PoolMetadata {
                url: x.url.clone(),
                hash: x.hash.to_vec().into(),
            }
        }

        /// Maps a credential to its u5c message.
        fn map_credential(x: &pallas_primitives::StakeCredential) -> u5c::StakeCredential {
            use pallas_primitives::StakeCredential;

            let inner = match x {
                StakeCredential::AddrKeyhash(x) => {
                    u5c::stake_credential::StakeCredential::AddrKeyHash(x.to_vec().into())
                }
                StakeCredential::ScriptHash(x) => {
                    u5c::stake_credential::StakeCredential::ScriptHash(x.to_vec().into())
                }
            };

            u5c::StakeCredential {
                stake_credential: inner.into(),
            }
        }

        /// A native script clause without its children, for any era's script type.
        enum NativeClause<'a> {
            Pubkey(&'a pallas_crypto::hash::Hash<28>),
            All,
            Any,
            NOfK(i64),
            InvalidBefore(u64),
            InvalidHereafter(u64),
            #[cfg(feature = "unstable")]
            RequireGuard(&'a pallas_primitives::StakeCredential),
        }

        impl<C: $crate::LedgerContext> Mapper<C> {
            pub fn map_multi_era_purpose(
                &self,
                x: &pallas_traverse::MultiEraRedeemerTag,
            ) -> u5c::RedeemerPurpose {
                use pallas_traverse::MultiEraRedeemerTag;

                match x {
                    MultiEraRedeemerTag::Spend => u5c::RedeemerPurpose::Spend,
                    MultiEraRedeemerTag::Mint => u5c::RedeemerPurpose::Mint,
                    MultiEraRedeemerTag::Cert => u5c::RedeemerPurpose::Cert,
                    MultiEraRedeemerTag::Reward => u5c::RedeemerPurpose::Reward,
                    MultiEraRedeemerTag::Vote => u5c::RedeemerPurpose::Vote,
                    MultiEraRedeemerTag::Propose => u5c::RedeemerPurpose::Propose,
                    #[cfg(feature = "unstable")]
                    MultiEraRedeemerTag::Guarding => guarding_purpose(),
                    _ => unimplemented!("map_redeemer has no arm for this purpose"),
                }
            }

            pub fn map_redeemer(&self, x: &pallas_traverse::MultiEraRedeemer) -> u5c::Redeemer {
                u5c::Redeemer {
                    purpose: self.map_multi_era_purpose(&x.multi_era_tag()).into(),
                    payload: self.map_plutus_datum(x.data()).into(),
                    index: x.index(),
                    ex_units: Some(u5c::ExUnits {
                        steps: x.ex_units().steps,
                        memory: x.ex_units().mem,
                    }),
                    original_cbor: x.encode().into(),
                }
            }

            fn decode_resolved_utxo(
                &self,
                resolved: &Option<$crate::UtxoMap>,
                input: &pallas_traverse::MultiEraInput,
                tx: &pallas_traverse::MultiEraTx,
            ) -> Option<u5c::TxOutput> {
                let as_txref = (*input.hash(), input.index() as u32);

                resolved
                    .as_ref()
                    .and_then(|x| x.get(&as_txref))
                    .and_then(|(era, cbor)| {
                        let o =
                            pallas_traverse::MultiEraOutput::decode(*era, cbor.as_slice()).ok()?;
                        Some(self.map_tx_output(&o, Some(tx)))
                    })
            }

            pub fn map_tx_input(
                &self,
                input: &pallas_traverse::MultiEraInput,
                tx: &pallas_traverse::MultiEraTx,
                order: u32,
                resolved: &Option<$crate::UtxoMap>,
            ) -> u5c::TxInput {
                u5c::TxInput {
                    tx_hash: input.hash().to_vec().into(),
                    output_index: input.index() as u32,
                    as_output: self.decode_resolved_utxo(resolved, input, tx),
                    redeemer: tx.find_spend_redeemer(order).map(|x| self.map_redeemer(&x)),
                }
            }

            pub fn map_tx_reference_input(
                &self,
                input: &pallas_traverse::MultiEraInput,
                resolved: &Option<$crate::UtxoMap>,
                tx: &pallas_traverse::MultiEraTx,
            ) -> u5c::TxInput {
                u5c::TxInput {
                    tx_hash: input.hash().to_vec().into(),
                    output_index: input.index() as u32,
                    as_output: self.decode_resolved_utxo(resolved, input, tx),
                    redeemer: None,
                }
            }

            pub fn map_tx_collateral(
                &self,
                input: &pallas_traverse::MultiEraInput,
                resolved: &Option<$crate::UtxoMap>,
                tx: &pallas_traverse::MultiEraTx,
            ) -> u5c::TxInput {
                u5c::TxInput {
                    tx_hash: input.hash().to_vec().into(),
                    output_index: input.index() as u32,
                    as_output: self.decode_resolved_utxo(resolved, input, tx),
                    redeemer: None,
                }
            }

            pub fn map_native_script(
                x: &pallas_primitives::alonzo::NativeScript,
            ) -> u5c::NativeScript {
                use pallas_primitives::alonzo::NativeScript;

                // Folded bottom-up rather than recursed: scripts nest as deep
                // as a transaction has bytes.
                pallas_codec::tree::fold_tree(x, |x, children| {
                    let clause = match x {
                        NativeScript::ScriptPubkey(x) => NativeClause::Pubkey(x),
                        NativeScript::ScriptAll(_) => NativeClause::All,
                        NativeScript::ScriptAny(_) => NativeClause::Any,
                        NativeScript::ScriptNOfK(n, _) => NativeClause::NOfK(*n),
                        NativeScript::InvalidBefore(s) => NativeClause::InvalidBefore(*s),
                        NativeScript::InvalidHereafter(s) => NativeClause::InvalidHereafter(*s),
                    };
                    Self::map_native_clause(clause, children)
                })
            }

            #[cfg(feature = "unstable")]
            fn map_dijkstra_native_script(
                x: &pallas_primitives::dijkstra::NativeScript,
            ) -> u5c::NativeScript {
                use pallas_primitives::dijkstra::NativeScript;

                pallas_codec::tree::fold_tree(x, |x, children| {
                    let clause = match x {
                        NativeScript::ScriptPubkey(x) => NativeClause::Pubkey(x),
                        NativeScript::ScriptAll(_) => NativeClause::All,
                        NativeScript::ScriptAny(_) => NativeClause::Any,
                        NativeScript::ScriptNOfK(n, _) => NativeClause::NOfK(*n),
                        NativeScript::InvalidBefore(s) => NativeClause::InvalidBefore(*s),
                        NativeScript::InvalidHereafter(s) => NativeClause::InvalidHereafter(*s),
                        NativeScript::ScriptRequireGuard(x) => NativeClause::RequireGuard(x),
                    };
                    Self::map_native_clause(clause, children)
                })
            }

            fn map_native_clause(
                clause: NativeClause,
                children: Vec<u5c::NativeScript>,
            ) -> u5c::NativeScript {
                let inner = match clause {
                    NativeClause::Pubkey(x) => Self::map_native_script_pubkey(x.to_vec()),
                    NativeClause::All => {
                        u5c::native_script::NativeScript::ScriptAll(u5c::NativeScriptList {
                            items: children,
                        })
                    }
                    NativeClause::Any => {
                        u5c::native_script::NativeScript::ScriptAny(u5c::NativeScriptList {
                            items: children,
                        })
                    }
                    NativeClause::NOfK(n) => {
                        u5c::native_script::NativeScript::ScriptNOfK(u5c::ScriptNOfK {
                            // u5c's `k` is wire-fixed at uint32, the ledger's threshold is
                            // i64: clamp rather than cast, or a negative value wraps into
                            // an unsatisfiable one instead of the satisfiable 0 it means.
                            k: n.clamp(0, i64::from(u32::MAX)) as u32,
                            scripts: children,
                        })
                    }
                    NativeClause::InvalidBefore(s) => {
                        u5c::native_script::NativeScript::InvalidBefore(s)
                    }
                    NativeClause::InvalidHereafter(s) => {
                        u5c::native_script::NativeScript::InvalidHereafter(s)
                    }
                    #[cfg(feature = "unstable")]
                    NativeClause::RequireGuard(x) => {
                        return u5c::NativeScript {
                            native_script: native_script_guard(x),
                        };
                    }
                };

                u5c::NativeScript {
                    native_script: Some(inner),
                }
            }

            pub fn map_any_script(&self, x: &pallas_primitives::conway::ScriptRef) -> u5c::Script {
                self.map_script_ref(&pallas_traverse::MultiEraScriptRef::from_conway(x))
            }

            fn map_script_ref(&self, x: &pallas_traverse::MultiEraScriptRef) -> u5c::Script {
                use pallas_traverse::script_ref::ScriptLanguage;

                let inner = match (x.language(), x.native_script(), x.plutus_bytes()) {
                    (_, Some(script), _) => {
                        u5c::script::Script::Native(Self::map_any_native_script(&script))
                    }
                    (ScriptLanguage::PlutusV1, _, Some(bytes)) => {
                        u5c::script::Script::PlutusV1(bytes.to_vec().into())
                    }
                    (ScriptLanguage::PlutusV2, _, Some(bytes)) => {
                        u5c::script::Script::PlutusV2(bytes.to_vec().into())
                    }
                    (ScriptLanguage::PlutusV3, _, Some(bytes)) => {
                        u5c::script::Script::PlutusV3(bytes.to_vec().into())
                    }
                    #[cfg(feature = "unstable")]
                    (ScriptLanguage::PlutusV4, _, Some(bytes)) => {
                        u5c::script::Script::PlutusV4(bytes.to_vec().into())
                    }
                    _ => unimplemented!("map_script_ref has no arm for this reference script"),
                };

                u5c::Script {
                    script: Some(inner),
                }
            }

            fn map_output_script(
                &self,
                x: &pallas_traverse::MultiEraOutput,
            ) -> Option<u5c::Script> {
                x.multi_era_script_ref().map(|x| self.map_script_ref(&x))
            }

            pub fn map_stake_credential(
                &self,
                x: &pallas_primitives::babbage::StakeCredential,
            ) -> u5c::StakeCredential {
                map_credential(x)
            }

            pub fn map_relay(&self, x: &pallas_primitives::alonzo::Relay) -> u5c::Relay {
                use pallas_primitives::babbage;
                match x {
                    babbage::Relay::SingleHostAddr(port, v4, v6) => u5c::Relay {
                        ip_v4: v4.clone().map(|x| x.to_vec().into()).unwrap_or_default(),
                        ip_v6: v6.clone().map(|x| x.to_vec().into()).unwrap_or_default(),
                        dns_name: String::default(),
                        port: (*port).unwrap_or_default(),
                    },
                    babbage::Relay::SingleHostName(port, name) => u5c::Relay {
                        ip_v4: Default::default(),
                        ip_v6: Default::default(),
                        dns_name: name.clone(),
                        port: (*port).unwrap_or_default(),
                    },
                    babbage::Relay::MultiHostName(name) => u5c::Relay {
                        ip_v4: Default::default(),
                        ip_v6: Default::default(),
                        dns_name: name.clone(),
                        port: Default::default(),
                    },
                }
            }

            pub fn map_withdrawals(
                &self,
                x: &(&[u8], u64),
                tx: &pallas_traverse::MultiEraTx,
                order: u32,
            ) -> u5c::Withdrawal {
                u5c::Withdrawal {
                    reward_account: Vec::from(x.0).into(),
                    coin: u64_to_bigint(x.1),
                    redeemer: tx
                        .find_withdrawal_redeemer(order)
                        .map(|x| self.map_redeemer(&x)),
                }
            }

            pub fn map_vkey_witness(
                &self,
                x: &pallas_primitives::alonzo::VKeyWitness,
            ) -> u5c::VKeyWitness {
                u5c::VKeyWitness {
                    vkey: x.vkey.to_vec().into(),
                    signature: x.signature.to_vec().into(),
                }
            }

            fn map_any_native_script(
                x: &pallas_traverse::MultiEraNativeScript,
            ) -> u5c::NativeScript {
                use pallas_traverse::MultiEraNativeScript;

                match x {
                    MultiEraNativeScript::AlonzoCompatible(x) => Self::map_native_script(x),
                    #[cfg(feature = "unstable")]
                    MultiEraNativeScript::Dijkstra(x) => Self::map_dijkstra_native_script(x),
                    _ => unimplemented!("collect_all_scripts has no arm for this native script"),
                }
            }

            fn collect_all_scripts(&self, tx: &pallas_traverse::MultiEraTx) -> Vec<u5c::Script> {
                let ns = tx
                    .multi_era_native_scripts()
                    .into_iter()
                    .map(|x| Self::map_any_native_script(&x))
                    .map(|x| u5c::Script {
                        script: u5c::script::Script::Native(x).into(),
                    })
                    .collect::<Vec<_>>()
                    .into_iter();

                let p1 = tx
                    .plutus_v1_scripts()
                    .iter()
                    .map(|x| x.0.to_vec().into())
                    .map(|x| u5c::Script {
                        script: u5c::script::Script::PlutusV1(x).into(),
                    });

                let p2 = tx
                    .plutus_v2_scripts()
                    .iter()
                    .map(|x| x.0.to_vec().into())
                    .map(|x| u5c::Script {
                        script: u5c::script::Script::PlutusV2(x).into(),
                    });

                ns.chain(p1).chain(p2).collect()
            }

            pub fn map_plutus_constr(
                &self,
                x: &pallas_primitives::alonzo::Constr<pallas_primitives::alonzo::PlutusData>,
            ) -> u5c::Constr {
                u5c::Constr {
                    tag: x.tag as u32,
                    any_constructor: x.any_constructor.unwrap_or_default(),
                    fields: x.fields.iter().map(|x| self.map_plutus_datum(x)).collect(),
                }
            }

            pub fn map_plutus_map(
                &self,
                x: &pallas_codec::utils::KeyValuePairs<
                    pallas_primitives::alonzo::PlutusData,
                    pallas_primitives::alonzo::PlutusData,
                >,
            ) -> u5c::PlutusDataMap {
                u5c::PlutusDataMap {
                    pairs: x
                        .iter()
                        .map(|(k, v)| u5c::PlutusDataPair {
                            key: self.map_plutus_datum(k).into(),
                            value: self.map_plutus_datum(v).into(),
                        })
                        .collect(),
                }
            }

            pub fn map_plutus_array(
                &self,
                x: &[pallas_primitives::alonzo::PlutusData],
            ) -> u5c::PlutusDataArray {
                u5c::PlutusDataArray {
                    items: x.iter().map(|x| self.map_plutus_datum(x)).collect(),
                }
            }

            pub fn map_plutus_bigint(&self, x: &pallas_primitives::alonzo::BigInt) -> u5c::BigInt {
                use pallas_primitives::babbage;
                let inner = match x {
                    babbage::BigInt::Int(x) => u5c::big_int::BigInt::Int(i128::from(x.0) as i64),
                    babbage::BigInt::BigUInt(x) => {
                        u5c::big_int::BigInt::BigUInt(Vec::<u8>::from(x.clone()).into())
                    }
                    babbage::BigInt::BigNInt(x) => {
                        u5c::big_int::BigInt::BigNInt(Vec::<u8>::from(x.clone()).into())
                    }
                };

                u5c::BigInt {
                    big_int: inner.into(),
                }
            }

            pub fn map_plutus_datum(
                &self,
                x: &pallas_primitives::alonzo::PlutusData,
            ) -> u5c::PlutusData {
                use pallas_primitives::babbage;

                // Folded bottom-up rather than recursed: datums nest as deep
                // as a transaction has bytes.
                pallas_codec::tree::fold_tree(x, |x, children: Vec<u5c::PlutusData>| {
                    let inner = match x {
                        babbage::PlutusData::Constr(x) => {
                            u5c::plutus_data::PlutusData::Constr(u5c::Constr {
                                tag: x.tag as u32,
                                any_constructor: x.any_constructor.unwrap_or_default(),
                                fields: children,
                            })
                        }
                        babbage::PlutusData::Map(_) => {
                            let mut children = children.into_iter();
                            let mut pairs = Vec::with_capacity(children.len() / 2);
                            while let (Some(key), Some(value)) = (children.next(), children.next())
                            {
                                pairs.push(u5c::PlutusDataPair {
                                    key: key.into(),
                                    value: value.into(),
                                });
                            }
                            u5c::plutus_data::PlutusData::Map(u5c::PlutusDataMap { pairs })
                        }
                        babbage::PlutusData::Array(_) => {
                            u5c::plutus_data::PlutusData::Array(u5c::PlutusDataArray {
                                items: children,
                            })
                        }
                        babbage::PlutusData::BigInt(x) => {
                            u5c::plutus_data::PlutusData::BigInt(self.map_plutus_bigint(x))
                        }
                        babbage::PlutusData::BoundedBytes(x) => {
                            u5c::plutus_data::PlutusData::BoundedBytes(x.to_vec().into())
                        }
                    };

                    u5c::PlutusData {
                        plutus_data: inner.into(),
                    }
                })
            }

            pub fn map_gov_action_id(
                &self,
                x: &Option<pallas_primitives::conway::GovActionId>,
            ) -> Option<u5c::GovernanceActionId> {
                x.as_ref().map(|inner| u5c::GovernanceActionId {
                    transaction_id: inner.transaction_id.to_vec().into(),
                    governance_action_index: inner.action_index,
                })
            }

            pub fn map_gov_proposal(
                &self,
                x: &pallas_traverse::MultiEraProposal,
            ) -> u5c::GovernanceActionProposal {
                u5c::GovernanceActionProposal {
                    deposit: u64_to_bigint(x.deposit()),
                    reward_account: x.reward_account().to_vec().into(),
                    gov_action: Some(self.map_gov_action(&x.gov_action())),
                    anchor: Some(map_anchor(x.anchor())),
                }
            }

            pub fn map_gov_action(
                &self,
                x: &pallas_traverse::MultiEraGovAction,
            ) -> u5c::GovernanceAction {
                use pallas_traverse::MultiEraGovActionKind;

                let script_hash = |x: Option<&pallas_primitives::conway::ScriptHash>| {
                    x.map(|x| x.to_vec()).unwrap_or_default().into()
                };

                let inner = match x.kind() {
                    MultiEraGovActionKind::ParameterChange(gov_id, params, script) => {
                        u5c::governance_action::GovernanceAction::ParameterChangeAction(
                            u5c::ParameterChangeAction {
                                gov_action_id: self.map_gov_action_id(&gov_id.cloned()),
                                protocol_param_update: Some(self.map_pparams_update(&params)),
                                policy_hash: script_hash(script),
                            },
                        )
                    }
                    MultiEraGovActionKind::HardForkInitiation(gov_id, version) => {
                        u5c::governance_action::GovernanceAction::HardForkInitiationAction(
                            u5c::HardForkInitiationAction {
                                gov_action_id: self.map_gov_action_id(&gov_id.cloned()),
                                protocol_version: Some(u5c::ProtocolVersion {
                                    major: version.0 as u32,
                                    minor: version.1 as u32,
                                }),
                            },
                        )
                    }
                    MultiEraGovActionKind::TreasuryWithdrawals(withdrawals, script) => {
                        u5c::governance_action::GovernanceAction::TreasuryWithdrawalsAction(
                            u5c::TreasuryWithdrawalsAction {
                                withdrawals: withdrawals
                                    .iter()
                                    .map(|(k, v)| u5c::WithdrawalAmount {
                                        reward_account: k.to_vec().into(),
                                        coin: u64_to_bigint(*v),
                                    })
                                    .collect(),
                                policy_hash: script_hash(script),
                            },
                        )
                    }
                    MultiEraGovActionKind::NoConfidence(gov_id) => {
                        u5c::governance_action::GovernanceAction::NoConfidenceAction(
                            u5c::NoConfidenceAction {
                                gov_action_id: self.map_gov_action_id(&gov_id.cloned()),
                            },
                        )
                    }
                    MultiEraGovActionKind::UpdateCommittee(gov_id, remove, add, threshold) => {
                        u5c::governance_action::GovernanceAction::UpdateCommitteeAction(
                            u5c::UpdateCommitteeAction {
                                gov_action_id: self.map_gov_action_id(&gov_id.cloned()),
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
                    MultiEraGovActionKind::NewConstitution(gov_id, constitution) => {
                        u5c::governance_action::GovernanceAction::NewConstitutionAction(
                            u5c::NewConstitutionAction {
                                gov_action_id: self.map_gov_action_id(&gov_id.cloned()),
                                constitution: Some(u5c::Constitution {
                                    anchor: Some(map_anchor(&constitution.anchor)),
                                    hash: script_hash(constitution.guardrail_script.as_ref()),
                                }),
                            },
                        )
                    }
                    MultiEraGovActionKind::Information => information_action(),
                    _ => unimplemented!("map_gov_action has no arm for this governance action"),
                };

                u5c::GovernanceAction {
                    governance_action: Some(inner),
                }
            }

            #[deprecated(since = "1.5.0", note = "use Mapper::map_gov_action")]
            pub fn map_conway_gov_action(
                &self,
                x: &pallas_primitives::conway::GovAction,
            ) -> u5c::GovernanceAction {
                self.map_gov_action(&pallas_traverse::MultiEraGovAction::from_conway(x))
            }

            pub fn map_metadatum(x: &pallas_primitives::alonzo::Metadatum) -> u5c::Metadatum {
                use pallas_primitives::babbage;

                // Folded bottom-up rather than recursed: metadata nests as
                // deep as a transaction has bytes.
                pallas_codec::tree::fold_tree(x, |x, children: Vec<u5c::Metadatum>| {
                    let inner = match x {
                        babbage::Metadatum::Int(x) => {
                            u5c::metadatum::Metadatum::Int(i128::from(x.0) as i64)
                        }
                        babbage::Metadatum::Bytes(x) => {
                            u5c::metadatum::Metadatum::Bytes(Vec::<u8>::from(x.clone()).into())
                        }
                        babbage::Metadatum::Text(x) => u5c::metadatum::Metadatum::Text(x.clone()),
                        babbage::Metadatum::Array(_) => {
                            u5c::metadatum::Metadatum::Array(u5c::MetadatumArray {
                                items: children,
                            })
                        }
                        babbage::Metadatum::Map(_) => {
                            let mut children = children.into_iter();
                            let mut pairs = Vec::with_capacity(children.len() / 2);
                            while let (Some(key), Some(value)) = (children.next(), children.next())
                            {
                                pairs.push(u5c::MetadatumPair {
                                    key: key.into(),
                                    value: value.into(),
                                });
                            }
                            u5c::metadatum::Metadatum::Map(u5c::MetadatumMap { pairs })
                        }
                    };

                    u5c::Metadatum {
                        metadatum: inner.into(),
                    }
                })
            }

            pub fn map_metadata(
                &self,
                label: u64,
                datum: &pallas_primitives::alonzo::Metadatum,
            ) -> u5c::Metadata {
                u5c::Metadata {
                    label,
                    value: Self::map_metadatum(datum).into(),
                }
            }

            fn collect_all_aux_scripts(
                &self,
                tx: &pallas_traverse::MultiEraTx,
            ) -> Vec<u5c::Script> {
                let ns = tx
                    .multi_era_aux_native_scripts()
                    .into_iter()
                    .map(|x| Self::map_any_native_script(&x))
                    .map(|x| u5c::Script {
                        script: u5c::script::Script::Native(x).into(),
                    })
                    .collect::<Vec<_>>()
                    .into_iter();

                let p1 = tx
                    .aux_plutus_v1_scripts()
                    .iter()
                    .map(|x| x.0.to_vec().into())
                    .map(|x| u5c::Script {
                        script: u5c::script::Script::PlutusV1(x).into(),
                    });

                let p2 = tx
                    .aux_plutus_v2_scripts()
                    .iter()
                    .map(|x| x.0.to_vec().into())
                    .map(|x| u5c::Script {
                        script: u5c::script::Script::PlutusV2(x).into(),
                    });

                let p3 = tx
                    .aux_plutus_v3_scripts()
                    .iter()
                    .map(|x| x.0.to_vec().into())
                    .map(|x| u5c::Script {
                        script: u5c::script::Script::PlutusV3(x).into(),
                    });

                let p4 = tx
                    .aux_plutus_v4_scripts()
                    .iter()
                    .map(|x| x.0.to_vec().into())
                    .map(|x| u5c::Script {
                        script: u5c::script::Script::PlutusV4(x).into(),
                    });

                ns.chain(p1).chain(p2).chain(p3).chain(p4).collect()
            }

            fn find_related_inputs(&self, tx: &pallas_traverse::MultiEraTx) -> Vec<$crate::TxoRef> {
                let inputs = tx
                    .inputs()
                    .into_iter()
                    .map(|x| (*x.hash(), x.index() as u32));

                let collateral = tx
                    .collateral()
                    .into_iter()
                    .map(|x| (*x.hash(), x.index() as u32));

                let reference_inputs = tx
                    .reference_inputs()
                    .into_iter()
                    .map(|x| (*x.hash(), x.index() as u32));

                inputs.chain(collateral).chain(reference_inputs).collect()
            }

            #[allow(clippy::needless_update)]
            pub fn map_block(&self, block: &pallas_traverse::MultiEraBlock) -> u5c::Block {
                let mapped = u5c::Block {
                    header: u5c::BlockHeader {
                        slot: block.slot(),
                        hash: block.hash().to_vec().into(),
                        height: block.number(),
                        ..Default::default()
                    }
                    .into(),
                    body: u5c::BlockBody {
                        tx: block.txs().iter().map(|x| self.map_tx(x)).collect(),
                        ..Default::default()
                    }
                    .into(),
                    // The spec declares `Block.timestamp` as milliseconds;
                    // `get_slot_timestamp` returns UNIX seconds.
                    timestamp: self
                        .ledger
                        .as_ref()
                        .and_then(|ledger| ledger.get_slot_timestamp(block.slot()))
                        .map(|seconds| seconds * 1000)
                        .unwrap_or(0),
                };

                with_leios_fields(mapped, block)
            }

            pub fn map_block_cbor(&self, raw: &[u8]) -> u5c::Block {
                let block = pallas_traverse::MultiEraBlock::decode(raw).unwrap();
                self.map_block(&block)
            }
        }

        #[cfg(test)]
        mod script_tests {
            use super::*;

            use pallas_primitives::{alonzo, conway};
            use pretty_assertions::assert_eq;

            #[cfg(feature = "unstable")]
            use pallas_primitives::dijkstra;
            #[cfg(feature = "unstable")]
            use pallas_traverse::{Era, MultiEraBlock, MultiEraNativeScript, MultiEraTx};

            #[derive(Clone)]
            struct NoLedger;

            impl $crate::LedgerContext for NoLedger {
                fn get_utxos(&self, _refs: &[$crate::TxoRef]) -> Option<$crate::UtxoMap> {
                    None
                }

                fn get_slot_timestamp(&self, _slot: u64) -> Option<u64> {
                    None
                }
            }

            #[test]
            fn a_conway_reference_script_maps_to_the_field_of_its_language() {
                let mapper = Mapper::new(NoLedger);
                let pubkey = alonzo::NativeScript::ScriptPubkey([1; 28].into());
                let bytes = vec![0xd8, 0x79, 0x80];

                let cases = [
                    (
                        conway::ScriptRef::NativeScript(pubkey.clone().into()),
                        u5c::script::Script::Native(Mapper::<NoLedger>::map_native_script(&pubkey)),
                    ),
                    (
                        conway::ScriptRef::PlutusV1Script(pallas_primitives::PlutusScript(
                            bytes.clone().into(),
                        )),
                        u5c::script::Script::PlutusV1(bytes.clone().into()),
                    ),
                    (
                        conway::ScriptRef::PlutusV2Script(pallas_primitives::PlutusScript(
                            bytes.clone().into(),
                        )),
                        u5c::script::Script::PlutusV2(bytes.clone().into()),
                    ),
                    (
                        conway::ScriptRef::PlutusV3Script(pallas_primitives::PlutusScript(
                            bytes.clone().into(),
                        )),
                        u5c::script::Script::PlutusV3(bytes.clone().into()),
                    ),
                ];

                for (script, expected) in cases {
                    assert_eq!(
                        mapper.map_any_script(&script),
                        u5c::Script {
                            script: Some(expected),
                        }
                    );
                }
            }

            #[cfg(feature = "unstable")]
            pub(super) fn fixture(name: &str) -> Vec<u8> {
                let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../test_data")
                    .join(name);
                let hex_str = std::fs::read_to_string(&path).expect("the fixture is readable");
                hex::decode(hex_str.trim()).expect("the fixture holds hex")
            }

            #[test]
            #[cfg(feature = "unstable")]
            fn every_dijkstra_block_fixture_maps_every_transaction() {
                let mapper = Mapper::new(NoLedger);
                let mut mapped_txs = 0;

                for n in 1..=15 {
                    let cbor = fixture(&format!("dijkstra{n}.block"));
                    let block = MultiEraBlock::decode(&cbor).expect("the fixture decodes");
                    let expected: Vec<Vec<u8>> =
                        block.txs().iter().map(|tx| tx.hash().to_vec()).collect();

                    let mapped: Vec<Vec<u8>> = mapper
                        .map_block(&block)
                        .body
                        .expect("a mapped block has a body")
                        .tx
                        .iter()
                        .map(|tx| common(tx).hash.to_vec())
                        .collect();

                    assert_eq!(mapped, expected, "dijkstra{n}.block");
                    mapped_txs += mapped.len();
                }

                assert_eq!(mapped_txs, 45, "the fixtures hold 45 transactions");
            }

            #[test]
            #[cfg(feature = "unstable")]
            fn a_dijkstra_reference_script_maps_to_the_field_of_its_language() {
                let cbor = fixture("dijkstra-scripts.tx");
                let tx = MultiEraTx::decode_for_era(Era::Dijkstra, &cbor).expect("the fixture decodes");
                let output = tx.outputs().into_iter().next().expect("the fixture has an output");

                assert_eq!(
                    Mapper::new(NoLedger).map_tx_output(&output, Some(&tx)).script,
                    Some(u5c::Script {
                        script: Some(u5c::script::Script::PlutusV4(vec![0xd8, 0x79, 0x80].into())),
                    })
                );
            }

            #[test]
            #[cfg(feature = "unstable")]
            fn a_dijkstra_transaction_and_its_sub_transaction_map_their_auxiliary_plutus_scripts() {
                use $crate::testing::{
                    AUX_PLUTUS_SCRIPTS, SUB_AUX_PLUTUS_SCRIPTS, dijkstra_tx_with_aux_plutus_scripts,
                };

                let plutus = |[v2, v3, v4]: [&[u8]; 3]| {
                    [
                        u5c::script::Script::PlutusV2(v2.to_vec().into()),
                        u5c::script::Script::PlutusV3(v3.to_vec().into()),
                        u5c::script::Script::PlutusV4(v4.to_vec().into()),
                    ]
                    .map(|x| u5c::Script { script: Some(x) })
                    .to_vec()
                };
                let tx = dijkstra_tx_with_aux_plutus_scripts();
                let mapper = Mapper::new(NoLedger);

                assert_eq!(
                    (
                        mapper.collect_all_aux_scripts(&tx),
                        mapper.collect_all_aux_scripts(&tx.sub_transactions()[0]),
                    ),
                    (plutus(AUX_PLUTUS_SCRIPTS), plutus(SUB_AUX_PLUTUS_SCRIPTS))
                );
            }

            #[test]
            #[cfg(feature = "unstable")]
            fn a_dijkstra_transaction_without_auxiliary_data_maps_no_auxiliary_script() {
                let tx = $crate::testing::dijkstra_tx_with_guards();
                let mapper = Mapper::new(NoLedger);

                assert_eq!(
                    (
                        mapper.collect_all_aux_scripts(&tx),
                        mapper.collect_all_aux_scripts(&tx.sub_transactions()[0]),
                    ),
                    (vec![], vec![])
                );
            }

            #[test]
            #[cfg(feature = "unstable")]
            fn a_dijkstra_native_script_maps_as_the_same_alonzo_script_does() {
                let alonzo = alonzo::NativeScript::ScriptNOfK(
                    2,
                    vec![
                        alonzo::NativeScript::ScriptPubkey([1; 28].into()),
                        alonzo::NativeScript::ScriptAny(vec![
                            alonzo::NativeScript::InvalidBefore(100),
                            alonzo::NativeScript::InvalidHereafter(200),
                        ]),
                        alonzo::NativeScript::ScriptAll(vec![]),
                    ],
                );
                let dijkstra = dijkstra::NativeScript::ScriptNOfK(
                    2,
                    vec![
                        dijkstra::NativeScript::ScriptPubkey([1; 28].into()),
                        dijkstra::NativeScript::ScriptAny(vec![
                            dijkstra::NativeScript::InvalidBefore(100),
                            dijkstra::NativeScript::InvalidHereafter(200),
                        ]),
                        dijkstra::NativeScript::ScriptAll(vec![]),
                    ],
                );

                assert_eq!(
                    Mapper::<NoLedger>::map_any_native_script(
                        &MultiEraNativeScript::from_decoded_dijkstra(&dijkstra)
                    ),
                    Mapper::<NoLedger>::map_native_script(&alonzo)
                );
            }
        }

        #[cfg(all(test, feature = "unstable"))]
        mod gov_tests {
            use super::*;

            use super::cert_tests::{key_credential, NoLedger};
            use super::script_tests::fixture;
            use pallas_primitives::conway;
            use pallas_traverse::{Era, MultiEraTx};
            use pretty_assertions::assert_eq;
            use std::collections::BTreeMap;

            /// The proposals of a Dijkstra transaction fixture, mapped.
            pub(super) fn mapped_proposals(name: &str) -> Vec<u5c::GovernanceActionProposal> {
                let cbor = fixture(name);
                let tx = MultiEraTx::decode_for_era(Era::Dijkstra, &cbor)
                    .expect("the fixture decodes");
                common(&Mapper::new(NoLedger).map_tx(&tx)).proposals.clone()
            }

            #[test]
            fn a_dijkstra_proposal_maps_its_deposit_account_action_and_anchor() {
                let proposals = mapped_proposals("dijkstra-param-change.tx");
                assert_eq!(proposals.len(), 1, "the fixture carries one proposal");
                let proposal = &proposals[0];

                assert_eq!(proposal.deposit, u64_to_bigint(100_000_000_000));
                assert_eq!(
                    proposal.reward_account.to_vec(),
                    hex::decode("e017d6e5dbc905918d1e16b0b2df27029391ac73f13966cf183b8051a9")
                        .unwrap()
                );
                assert_eq!(
                    proposal.anchor,
                    Some(u5c::Anchor {
                        url: "ipfs://QmZz6G5DMHzMRQEVzU1LnaDJhMZS3gbmvRaxvDsBudUms5".into(),
                        content_hash: hex::decode(
                            "d766c3215498282d1927e8e81aafa949e193b3744af4b543657b7ee02311867c"
                        )
                        .unwrap()
                        .into(),
                    })
                );

                let Some(u5c::governance_action::GovernanceAction::ParameterChangeAction(action)) =
                    proposal
                        .gov_action
                        .as_ref()
                        .and_then(|x| x.governance_action.as_ref())
                else {
                    panic!("the proposal maps to a parameter change");
                };
                assert_eq!(action.gov_action_id, None);
                assert_eq!(
                    action.policy_hash.to_vec(),
                    hex::decode("e3172285605e502bde008a7efe2b2eec516958b7bae911947a5c9373")
                        .unwrap()
                );
                assert!(action.protocol_param_update.is_some());
            }

            #[test]
            fn a_conway_action_of_each_kind_maps_to_its_member() {
                let id = conway::GovActionId {
                    transaction_id: [0x11; 32].into(),
                    action_index: 3,
                };
                let mapped_id = Some(u5c::GovernanceActionId {
                    transaction_id: [0x11; 32].to_vec().into(),
                    governance_action_index: 3,
                });
                let policy = [0x44; 28];
                let threshold = pallas_primitives::RationalNumber {
                    numerator: 2,
                    denominator: 3,
                };
                let anchor = conway::Anchor {
                    url: "https://example.invalid/constitution".into(),
                    content_hash: [0x55; 32].into(),
                };

                // {3: 0}, a zero max transaction size.
                let update = crate::testing::conway_update(&[0xa1, 0x03, 0x00]);
                let mapped_update = ParamUpdate {
                    max_tx_size: update_scalar(Some(0)),
                    ..Default::default()
                };

                use u5c::governance_action::GovernanceAction as A;
                let cases = vec![
                    (
                        conway::GovAction::ParameterChange(
                            Some(id.clone()),
                            Box::new(update),
                            Some(policy.into()),
                        ),
                        A::ParameterChangeAction(u5c::ParameterChangeAction {
                            gov_action_id: mapped_id.clone(),
                            protocol_param_update: Some(mapped_update),
                            policy_hash: policy.to_vec().into(),
                        }),
                    ),
                    (
                        conway::GovAction::HardForkInitiation(None, (10, 2)),
                        A::HardForkInitiationAction(u5c::HardForkInitiationAction {
                            gov_action_id: None,
                            protocol_version: Some(u5c::ProtocolVersion {
                                major: 10,
                                minor: 2,
                            }),
                        }),
                    ),
                    (
                        conway::GovAction::TreasuryWithdrawals(
                            BTreeMap::from([(vec![0xe0; 29].into(), 7)]),
                            None,
                        ),
                        A::TreasuryWithdrawalsAction(u5c::TreasuryWithdrawalsAction {
                            withdrawals: vec![u5c::WithdrawalAmount {
                                reward_account: vec![0xe0; 29].into(),
                                coin: u64_to_bigint(7),
                            }],
                            policy_hash: Default::default(),
                        }),
                    ),
                    (
                        conway::GovAction::NoConfidence(Some(id.clone())),
                        A::NoConfidenceAction(u5c::NoConfidenceAction {
                            gov_action_id: mapped_id.clone(),
                        }),
                    ),
                    (
                        conway::GovAction::UpdateCommittee(
                            None,
                            vec![conway::StakeCredential::AddrKeyhash([0x01; 28].into())].into(),
                            BTreeMap::from([(
                                conway::StakeCredential::AddrKeyhash([0x02; 28].into()),
                                90,
                            )]),
                            threshold,
                        ),
                        A::UpdateCommitteeAction(u5c::UpdateCommitteeAction {
                            gov_action_id: None,
                            remove_committee_credentials: vec![key_credential([0x01; 28])],
                            new_committee_credentials: vec![u5c::NewCommitteeCredentials {
                                committee_cold_credential: Some(key_credential([0x02; 28])),
                                expires_epoch: 90,
                            }],
                            new_committee_threshold: Some(u5c::RationalNumber {
                                numerator: 2,
                                denominator: 3,
                            }),
                        }),
                    ),
                    (
                        conway::GovAction::NewConstitution(
                            Some(id),
                            conway::Constitution {
                                anchor,
                                guardrail_script: Some(policy.into()),
                            },
                        ),
                        A::NewConstitutionAction(u5c::NewConstitutionAction {
                            gov_action_id: mapped_id,
                            constitution: Some(u5c::Constitution {
                                anchor: Some(u5c::Anchor {
                                    url: "https://example.invalid/constitution".into(),
                                    content_hash: [0x55; 32].to_vec().into(),
                                }),
                                hash: policy.to_vec().into(),
                            }),
                        }),
                    ),
                ];

                for (action, expected) in cases {
                    assert_eq!(
                        Mapper::new(NoLedger)
                            .map_gov_action(&pallas_traverse::MultiEraGovAction::from_conway(
                                &action
                            ))
                            .governance_action,
                        Some(expected)
                    );
                }
            }
        }

        // ---- certificates ----------------------------------------------------

        impl<C: $crate::LedgerContext> Mapper<C> {
            /// Close a mapped certificate over the redeemer the transaction
            /// pairs with the certificate at this position.
            fn certificate(
                &self,
                inner: Option<u5c::certificate::Certificate>,
                tx: &pallas_traverse::MultiEraTx,
                order: u32,
            ) -> u5c::Certificate {
                u5c::Certificate {
                    certificate: inner,
                    redeemer: tx
                        .find_certificate_redeemer(order)
                        .map(|r| self.map_redeemer(&r)),
                }
            }

            pub fn map_drep(&self, x: &pallas_primitives::conway::DRep) -> u5c::DRep {
                use pallas_primitives::conway;
                u5c::DRep {
                    drep: match x {
                        conway::DRep::Key(x) => {
                            u5c::d_rep::Drep::AddrKeyHash(x.to_vec().into()).into()
                        }
                        conway::DRep::Script(x) => {
                            u5c::d_rep::Drep::ScriptHash(x.to_vec().into()).into()
                        }
                        conway::DRep::Abstain => u5c::d_rep::Drep::Abstain(true).into(),
                        conway::DRep::NoConfidence => u5c::d_rep::Drep::NoConfidence(true).into(),
                    },
                }
            }

            /// Map what a certificate of any era certifies, read through the
            /// era neutral certificate view. Returns None for a kind the u5c
            /// schema names no message for.
            #[allow(clippy::needless_update)]
            pub fn map_cert_kind(
                &self,
                x: &pallas_traverse::MultiEraCertKind,
            ) -> Option<u5c::certificate::Certificate> {
                use pallas_primitives::alonzo;
                use pallas_traverse::MultiEraCertKind;
                let inner = match x {
                    MultiEraCertKind::StakeRegistration(cred) => {
                        u5c::certificate::Certificate::StakeRegistration(
                            self.map_stake_credential(cred),
                        )
                    }
                    MultiEraCertKind::StakeDeregistration(cred) => {
                        u5c::certificate::Certificate::StakeDeregistration(
                            self.map_stake_credential(cred),
                        )
                    }
                    MultiEraCertKind::StakeDelegation(cred, pool) => {
                        u5c::certificate::Certificate::StakeDelegation(u5c::StakeDelegationCert {
                            stake_credential: self.map_stake_credential(cred).into(),
                            pool_keyhash: pool.to_vec().into(),
                        })
                    }
                    MultiEraCertKind::PoolRegistration(pool) => {
                        u5c::certificate::Certificate::PoolRegistration(u5c::PoolRegistrationCert {
                            operator: pool.operator.to_vec().into(),
                            vrf_keyhash: pool.vrf_keyhash.to_vec().into(),
                            pledge: u64_to_bigint(pool.pledge),
                            cost: u64_to_bigint(pool.cost),
                            margin: rational_number_to_u5c(pool.margin.clone()).into(),
                            reward_account: pool.reward_account.to_vec().into(),
                            pool_owners: pool
                                .pool_owners
                                .iter()
                                .map(|x| x.to_vec().into())
                                .collect(),
                            relays: pool.relays.iter().map(|x| self.map_relay(x)).collect(),
                            pool_metadata: pool.pool_metadata.map(map_pool_metadata),
                            ..Default::default()
                        })
                    }
                    MultiEraCertKind::PoolRetirement(pool, epoch) => {
                        u5c::certificate::Certificate::PoolRetirement(u5c::PoolRetirementCert {
                            pool_keyhash: pool.to_vec().into(),
                            epoch: *epoch,
                        })
                    }
                    MultiEraCertKind::Reg(cred, coin) => {
                        u5c::certificate::Certificate::RegCert(u5c::RegCert {
                            stake_credential: self.map_stake_credential(cred).into(),
                            coin: u64_to_bigint(*coin),
                        })
                    }
                    MultiEraCertKind::UnReg(cred, coin) => {
                        u5c::certificate::Certificate::UnregCert(u5c::UnRegCert {
                            stake_credential: self.map_stake_credential(cred).into(),
                            coin: u64_to_bigint(*coin),
                        })
                    }
                    MultiEraCertKind::VoteDeleg(cred, drep) => {
                        u5c::certificate::Certificate::VoteDelegCert(u5c::VoteDelegCert {
                            stake_credential: self.map_stake_credential(cred).into(),
                            drep: self.map_drep(drep).into(),
                        })
                    }
                    MultiEraCertKind::StakeVoteDeleg(stake_cred, pool_id, drep) => {
                        u5c::certificate::Certificate::StakeVoteDelegCert(u5c::StakeVoteDelegCert {
                            stake_credential: self.map_stake_credential(stake_cred).into(),
                            pool_keyhash: pool_id.to_vec().into(),
                            drep: self.map_drep(drep).into(),
                        })
                    }
                    MultiEraCertKind::StakeRegDeleg(stake_cred, pool_id, coin) => {
                        u5c::certificate::Certificate::StakeRegDelegCert(u5c::StakeRegDelegCert {
                            stake_credential: self.map_stake_credential(stake_cred).into(),
                            pool_keyhash: pool_id.to_vec().into(),
                            coin: u64_to_bigint(*coin),
                        })
                    }
                    MultiEraCertKind::VoteRegDeleg(vote_cred, drep, coin) => {
                        u5c::certificate::Certificate::VoteRegDelegCert(u5c::VoteRegDelegCert {
                            stake_credential: self.map_stake_credential(vote_cred).into(),
                            drep: self.map_drep(drep).into(),
                            coin: u64_to_bigint(*coin),
                        })
                    }
                    MultiEraCertKind::StakeVoteRegDeleg(stake_cred, pool_id, drep, coin) => {
                        u5c::certificate::Certificate::StakeVoteRegDelegCert(
                            u5c::StakeVoteRegDelegCert {
                                stake_credential: self.map_stake_credential(stake_cred).into(),
                                pool_keyhash: pool_id.to_vec().into(),
                                drep: self.map_drep(drep).into(),
                                coin: u64_to_bigint(*coin),
                            },
                        )
                    }
                    MultiEraCertKind::AuthCommitteeHot(cold_cred, hot_cred) => {
                        u5c::certificate::Certificate::AuthCommitteeHotCert(
                            u5c::AuthCommitteeHotCert {
                                committee_cold_credential: self
                                    .map_stake_credential(cold_cred)
                                    .into(),
                                committee_hot_credential: self
                                    .map_stake_credential(hot_cred)
                                    .into(),
                            },
                        )
                    }
                    MultiEraCertKind::ResignCommitteeCold(cold_cred, anchor) => {
                        u5c::certificate::Certificate::ResignCommitteeColdCert(
                            u5c::ResignCommitteeColdCert {
                                committee_cold_credential: self
                                    .map_stake_credential(cold_cred)
                                    .into(),
                                anchor: anchor.map(map_anchor),
                            },
                        )
                    }
                    MultiEraCertKind::RegDRep(cred, coin, anchor) => {
                        u5c::certificate::Certificate::RegDrepCert(u5c::RegDRepCert {
                            drep_credential: self.map_stake_credential(cred).into(),
                            coin: u64_to_bigint(*coin),
                            anchor: anchor.map(map_anchor),
                        })
                    }
                    MultiEraCertKind::UnRegDRep(cred, coin) => {
                        u5c::certificate::Certificate::UnregDrepCert(u5c::UnRegDRepCert {
                            drep_credential: self.map_stake_credential(cred).into(),
                            coin: u64_to_bigint(*coin),
                        })
                    }
                    MultiEraCertKind::UpdateDRep(cred, anchor) => {
                        u5c::certificate::Certificate::UpdateDrepCert(u5c::UpdateDRepCert {
                            drep_credential: self.map_stake_credential(cred).into(),
                            anchor: anchor.map(map_anchor),
                        })
                    }
                    MultiEraCertKind::GenesisKeyDelegation(genesis, delegate, vrf) => {
                        u5c::certificate::Certificate::GenesisKeyDelegation(
                            u5c::GenesisKeyDelegationCert {
                                genesis_hash: genesis.to_vec().into(),
                                genesis_delegate_hash: delegate.to_vec().into(),
                                vrf_keyhash: vrf.to_vec().into(),
                            },
                        )
                    }
                    MultiEraCertKind::MoveInstantaneousRewards(rewards) => {
                        u5c::certificate::Certificate::MirCert(u5c::MirCert {
                            from: match &rewards.source {
                                alonzo::InstantaneousRewardSource::Reserves => {
                                    u5c::MirSource::Reserves.into()
                                }
                                alonzo::InstantaneousRewardSource::Treasury => {
                                    u5c::MirSource::Treasury.into()
                                }
                            },
                            to: match &rewards.target {
                                alonzo::InstantaneousRewardTarget::StakeCredentials(x) => x
                                    .iter()
                                    .map(|(k, v)| u5c::MirTarget {
                                        stake_credential: self.map_stake_credential(k).into(),
                                        delta_coin: i64_to_bigint(*v),
                                    })
                                    .collect(),
                                _ => Default::default(),
                            },
                            other_pot: match &rewards.target {
                                alonzo::InstantaneousRewardTarget::OtherAccountingPot(x) => *x,
                                _ => Default::default(),
                            },
                        })
                    }
                    // The u5c schema names no message for a kind this mapper
                    // does not name.
                    _ => return None,
                };

                Some(inner)
            }

            pub fn map_cert(
                &self,
                x: &pallas_traverse::MultiEraCert,
                tx: &pallas_traverse::MultiEraTx,
                order: u32,
            ) -> Option<u5c::Certificate> {
                let inner = self.map_cert_kind(&x.kind()?);
                #[cfg(feature = "unstable")]
                let inner = with_pool_bls_key(inner, x.bls_key());
                Some(self.certificate(inner, tx, order))
            }
        }

        #[cfg(test)]
        mod cert_tests {
            use super::*;

            use pallas_primitives::{alonzo, conway};
            use pallas_traverse::{MultiEraBlock, MultiEraCert, MultiEraTx};
            use pretty_assertions::assert_eq;
            use std::borrow::Cow;
            use std::collections::BTreeMap;

            #[derive(Clone)]
            pub(super) struct NoLedger;

            impl $crate::LedgerContext for NoLedger {
                fn get_utxos(&self, _refs: &[$crate::TxoRef]) -> Option<$crate::UtxoMap> {
                    None
                }

                fn get_slot_timestamp(&self, _slot: u64) -> Option<u64> {
                    None
                }
            }

            pub(super) const CREDENTIAL: [u8; 28] = [0x01; 28];
            pub(super) const POOL: [u8; 28] = [0x02; 28];
            const OWNER: [u8; 28] = [0x04; 28];
            const VRF: [u8; 32] = [0x06; 32];
            const HOT: [u8; 28] = [0x07; 28];
            const GENESIS: [u8; 28] = [0x08; 28];
            const DELEGATE: [u8; 28] = [0x09; 28];
            const REWARD_ACCOUNT: [u8; 29] = [0xe0; 29];
            const ANCHOR_HASH: [u8; 32] = [0x22; 32];
            const METADATA_HASH: [u8; 32] = [0x33; 32];
            const ANCHOR_URL: &str = "https://example.invalid/anchor";
            const METADATA_URL: &str = "https://example.invalid/pool.json";
            const RELAY_NAME: &str = "relay.example.invalid";
            const PLEDGE: u64 = 500;
            const COST: u64 = 340;
            const DEPOSIT: u64 = 5;
            pub(super) const EPOCH: u64 = 9;
            const REWARD: i64 = 7;

            pub(super) fn credential() -> conway::StakeCredential {
                conway::StakeCredential::AddrKeyhash(CREDENTIAL.into())
            }

            fn hot_credential() -> conway::StakeCredential {
                conway::StakeCredential::AddrKeyhash(HOT.into())
            }

            fn anchor() -> conway::Anchor {
                conway::Anchor {
                    url: ANCHOR_URL.into(),
                    content_hash: ANCHOR_HASH.into(),
                }
            }

            fn margin() -> pallas_primitives::RationalNumber {
                pallas_primitives::RationalNumber {
                    numerator: 1,
                    denominator: 50,
                }
            }

            fn relays() -> Vec<pallas_primitives::Relay> {
                vec![pallas_primitives::Relay::MultiHostName(RELAY_NAME.into())]
            }

            fn metadata() -> pallas_primitives::PoolMetadata {
                pallas_primitives::PoolMetadata {
                    url: METADATA_URL.into(),
                    hash: METADATA_HASH.to_vec().into(),
                }
            }

            /// The u5c message for a credential holding the key hash given.
            pub(super) fn key_credential(hash: [u8; 28]) -> u5c::StakeCredential {
                u5c::StakeCredential {
                    stake_credential: Some(u5c::stake_credential::StakeCredential::AddrKeyHash(
                        hash.to_vec().into(),
                    )),
                }
            }

            /// The u5c message for a count that fits in a signed 64 bit field.
            fn bigint(value: i64) -> Option<u5c::BigInt> {
                Some(u5c::BigInt {
                    big_int: Some(u5c::big_int::BigInt::Int(value)),
                })
            }

            fn mapped_anchor() -> u5c::Anchor {
                u5c::Anchor {
                    url: ANCHOR_URL.to_string(),
                    content_hash: ANCHOR_HASH.to_vec().into(),
                }
            }

            fn abstain() -> Option<u5c::DRep> {
                Some(u5c::DRep {
                    drep: Some(u5c::d_rep::Drep::Abstain(true)),
                })
            }

            fn no_confidence() -> Option<u5c::DRep> {
                Some(u5c::DRep {
                    drep: Some(u5c::d_rep::Drep::NoConfidence(true)),
                })
            }

            /// A Conway transaction carrying the redeemers given and nothing
            /// else, to stand as the transaction a certificate is read against.
            pub(super) fn tx_with_redeemers(redeemers: Vec<conway::Redeemer>) -> conway::Tx<'static> {
                let body = conway::TransactionBody {
                    inputs: Vec::new().into(),
                    outputs: Vec::new(),
                    fee: 0,
                    ttl: None,
                    certificates: None,
                    withdrawals: None,
                    auxiliary_data_hash: None,
                    validity_interval_start: None,
                    mint: None,
                    script_data_hash: None,
                    collateral: None,
                    required_signers: None,
                    network_id: None,
                    collateral_return: None,
                    total_collateral: None,
                    reference_inputs: None,
                    voting_procedures: None,
                    proposal_procedures: None,
                    treasury_value: None,
                    donation: None,
                };

                let witness_set = conway::WitnessSet {
                    vkeywitness: None,
                    native_script: None,
                    bootstrap_witness: None,
                    plutus_v1_script: None,
                    plutus_data: None,
                    redeemer: Some(conway::Redeemers::List(redeemers).into()),
                    plutus_v2_script: None,
                    plutus_v3_script: None,
                };

                conway::Tx {
                    transaction_body: body.into(),
                    transaction_witness_set: witness_set.into(),
                    success: true,
                    auxiliary_data: pallas_primitives::Nullable::Null,
                }
            }

            fn babbage10_block() -> Vec<u8> {
                let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../test_data/babbage10.block");
                let hex_str = std::fs::read_to_string(&path).expect("the fixture is readable");
                hex::decode(hex_str.trim()).expect("the fixture holds hex")
            }

            #[test]
            #[allow(clippy::needless_update)]
            fn every_conway_certificate_maps_to_the_u5c_message_naming_it() {
                let cases: Vec<(conway::Certificate, u5c::certificate::Certificate)> = vec![
                    (
                        conway::Certificate::StakeRegistration(credential()),
                        u5c::certificate::Certificate::StakeRegistration(key_credential(
                            CREDENTIAL,
                        )),
                    ),
                    (
                        conway::Certificate::StakeDeregistration(credential()),
                        u5c::certificate::Certificate::StakeDeregistration(key_credential(
                            CREDENTIAL,
                        )),
                    ),
                    (
                        conway::Certificate::StakeDelegation(credential(), POOL.into()),
                        u5c::certificate::Certificate::StakeDelegation(u5c::StakeDelegationCert {
                            stake_credential: Some(key_credential(CREDENTIAL)),
                            pool_keyhash: POOL.to_vec().into(),
                        }),
                    ),
                    (
                        conway::Certificate::PoolRegistration {
                            operator: POOL.into(),
                            vrf_keyhash: VRF.into(),
                            pledge: PLEDGE,
                            cost: COST,
                            margin: margin(),
                            reward_account: REWARD_ACCOUNT.to_vec().into(),
                            pool_owners: vec![OWNER.into()].into(),
                            relays: relays(),
                            pool_metadata: Some(metadata()),
                        },
                        u5c::certificate::Certificate::PoolRegistration(
                            u5c::PoolRegistrationCert {
                                operator: POOL.to_vec().into(),
                                vrf_keyhash: VRF.to_vec().into(),
                                pledge: bigint(PLEDGE as i64),
                                cost: bigint(COST as i64),
                                margin: Some(u5c::RationalNumber {
                                    numerator: 1,
                                    denominator: 50,
                                }),
                                reward_account: REWARD_ACCOUNT.to_vec().into(),
                                pool_owners: vec![OWNER.to_vec().into()],
                                relays: vec![u5c::Relay {
                                    ip_v4: Default::default(),
                                    ip_v6: Default::default(),
                                    dns_name: RELAY_NAME.to_string(),
                                    port: 0,
                                }],
                                pool_metadata: Some(u5c::PoolMetadata {
                                    url: METADATA_URL.to_string(),
                                    hash: METADATA_HASH.to_vec().into(),
                                }),
                                ..Default::default()
                            },
                        ),
                    ),
                    (
                        conway::Certificate::PoolRetirement(POOL.into(), EPOCH),
                        u5c::certificate::Certificate::PoolRetirement(u5c::PoolRetirementCert {
                            pool_keyhash: POOL.to_vec().into(),
                            epoch: EPOCH,
                        }),
                    ),
                    (
                        conway::Certificate::Reg(credential(), DEPOSIT),
                        u5c::certificate::Certificate::RegCert(u5c::RegCert {
                            stake_credential: Some(key_credential(CREDENTIAL)),
                            coin: bigint(DEPOSIT as i64),
                        }),
                    ),
                    (
                        conway::Certificate::UnReg(credential(), DEPOSIT),
                        u5c::certificate::Certificate::UnregCert(u5c::UnRegCert {
                            stake_credential: Some(key_credential(CREDENTIAL)),
                            coin: bigint(DEPOSIT as i64),
                        }),
                    ),
                    (
                        conway::Certificate::VoteDeleg(credential(), conway::DRep::Abstain),
                        u5c::certificate::Certificate::VoteDelegCert(u5c::VoteDelegCert {
                            stake_credential: Some(key_credential(CREDENTIAL)),
                            drep: abstain(),
                        }),
                    ),
                    (
                        conway::Certificate::StakeVoteDeleg(
                            credential(),
                            POOL.into(),
                            conway::DRep::NoConfidence,
                        ),
                        u5c::certificate::Certificate::StakeVoteDelegCert(
                            u5c::StakeVoteDelegCert {
                                stake_credential: Some(key_credential(CREDENTIAL)),
                                pool_keyhash: POOL.to_vec().into(),
                                drep: no_confidence(),
                            },
                        ),
                    ),
                    (
                        conway::Certificate::StakeRegDeleg(credential(), POOL.into(), DEPOSIT),
                        u5c::certificate::Certificate::StakeRegDelegCert(u5c::StakeRegDelegCert {
                            stake_credential: Some(key_credential(CREDENTIAL)),
                            pool_keyhash: POOL.to_vec().into(),
                            coin: bigint(DEPOSIT as i64),
                        }),
                    ),
                    (
                        conway::Certificate::VoteRegDeleg(
                            credential(),
                            conway::DRep::Abstain,
                            DEPOSIT,
                        ),
                        u5c::certificate::Certificate::VoteRegDelegCert(u5c::VoteRegDelegCert {
                            stake_credential: Some(key_credential(CREDENTIAL)),
                            drep: abstain(),
                            coin: bigint(DEPOSIT as i64),
                        }),
                    ),
                    (
                        conway::Certificate::StakeVoteRegDeleg(
                            credential(),
                            POOL.into(),
                            conway::DRep::Abstain,
                            DEPOSIT,
                        ),
                        u5c::certificate::Certificate::StakeVoteRegDelegCert(
                            u5c::StakeVoteRegDelegCert {
                                stake_credential: Some(key_credential(CREDENTIAL)),
                                pool_keyhash: POOL.to_vec().into(),
                                drep: abstain(),
                                coin: bigint(DEPOSIT as i64),
                            },
                        ),
                    ),
                    (
                        conway::Certificate::AuthCommitteeHot(credential(), hot_credential()),
                        u5c::certificate::Certificate::AuthCommitteeHotCert(
                            u5c::AuthCommitteeHotCert {
                                committee_cold_credential: Some(key_credential(CREDENTIAL)),
                                committee_hot_credential: Some(key_credential(HOT)),
                            },
                        ),
                    ),
                    (
                        conway::Certificate::ResignCommitteeCold(credential(), Some(anchor())),
                        u5c::certificate::Certificate::ResignCommitteeColdCert(
                            u5c::ResignCommitteeColdCert {
                                committee_cold_credential: Some(key_credential(CREDENTIAL)),
                                anchor: Some(mapped_anchor()),
                            },
                        ),
                    ),
                    (
                        conway::Certificate::RegDRepCert(credential(), DEPOSIT, Some(anchor())),
                        u5c::certificate::Certificate::RegDrepCert(u5c::RegDRepCert {
                            drep_credential: Some(key_credential(CREDENTIAL)),
                            coin: bigint(DEPOSIT as i64),
                            anchor: Some(mapped_anchor()),
                        }),
                    ),
                    (
                        conway::Certificate::UnRegDRepCert(credential(), DEPOSIT),
                        u5c::certificate::Certificate::UnregDrepCert(u5c::UnRegDRepCert {
                            drep_credential: Some(key_credential(CREDENTIAL)),
                            coin: bigint(DEPOSIT as i64),
                        }),
                    ),
                    (
                        conway::Certificate::UpdateDRepCert(credential(), None),
                        u5c::certificate::Certificate::UpdateDrepCert(u5c::UpdateDRepCert {
                            drep_credential: Some(key_credential(CREDENTIAL)),
                            anchor: None,
                        }),
                    ),
                ];

                assert_eq!(cases.len(), 17, "Conway's type names seventeen certificates");

                let raw = tx_with_redeemers(Vec::new());
                let tx = MultiEraTx::from_conway(&raw);
                let mapper = Mapper::new(NoLedger);

                for (order, (certificate, expected)) in cases.iter().enumerate() {
                    let cert = MultiEraCert::Conway(Box::new(Cow::Borrowed(certificate)));
                    let mapped = mapper
                        .map_cert(&cert, &tx, order as u32)
                        .expect("a Conway certificate maps");

                    assert_eq!(
                        mapped.certificate.as_ref(),
                        Some(expected),
                        "the Conway certificate at position {order}"
                    );
                    assert!(
                        mapped.redeemer.is_none(),
                        "position {order} pairs no redeemer, this transaction carries none"
                    );
                }
            }

            #[test]
            #[allow(clippy::needless_update)]
            fn every_alonzo_certificate_maps_to_the_u5c_message_naming_it() {
                let cases: Vec<(alonzo::Certificate, u5c::certificate::Certificate)> = vec![
                    (
                        alonzo::Certificate::StakeRegistration(credential()),
                        u5c::certificate::Certificate::StakeRegistration(key_credential(
                            CREDENTIAL,
                        )),
                    ),
                    (
                        alonzo::Certificate::StakeDeregistration(credential()),
                        u5c::certificate::Certificate::StakeDeregistration(key_credential(
                            CREDENTIAL,
                        )),
                    ),
                    (
                        alonzo::Certificate::StakeDelegation(credential(), POOL.into()),
                        u5c::certificate::Certificate::StakeDelegation(u5c::StakeDelegationCert {
                            stake_credential: Some(key_credential(CREDENTIAL)),
                            pool_keyhash: POOL.to_vec().into(),
                        }),
                    ),
                    (
                        alonzo::Certificate::PoolRegistration {
                            operator: POOL.into(),
                            vrf_keyhash: VRF.into(),
                            pledge: PLEDGE,
                            cost: COST,
                            margin: margin(),
                            reward_account: REWARD_ACCOUNT.to_vec().into(),
                            pool_owners: vec![OWNER.into()],
                            relays: relays(),
                            pool_metadata: Some(metadata()),
                        },
                        u5c::certificate::Certificate::PoolRegistration(
                            u5c::PoolRegistrationCert {
                                operator: POOL.to_vec().into(),
                                vrf_keyhash: VRF.to_vec().into(),
                                pledge: bigint(PLEDGE as i64),
                                cost: bigint(COST as i64),
                                margin: Some(u5c::RationalNumber {
                                    numerator: 1,
                                    denominator: 50,
                                }),
                                reward_account: REWARD_ACCOUNT.to_vec().into(),
                                pool_owners: vec![OWNER.to_vec().into()],
                                relays: vec![u5c::Relay {
                                    ip_v4: Default::default(),
                                    ip_v6: Default::default(),
                                    dns_name: RELAY_NAME.to_string(),
                                    port: 0,
                                }],
                                pool_metadata: Some(u5c::PoolMetadata {
                                    url: METADATA_URL.to_string(),
                                    hash: METADATA_HASH.to_vec().into(),
                                }),
                                ..Default::default()
                            },
                        ),
                    ),
                    (
                        alonzo::Certificate::PoolRetirement(POOL.into(), EPOCH),
                        u5c::certificate::Certificate::PoolRetirement(u5c::PoolRetirementCert {
                            pool_keyhash: POOL.to_vec().into(),
                            epoch: EPOCH,
                        }),
                    ),
                    (
                        alonzo::Certificate::GenesisKeyDelegation(
                            GENESIS.to_vec().into(),
                            DELEGATE.to_vec().into(),
                            VRF.into(),
                        ),
                        u5c::certificate::Certificate::GenesisKeyDelegation(
                            u5c::GenesisKeyDelegationCert {
                                genesis_hash: GENESIS.to_vec().into(),
                                genesis_delegate_hash: DELEGATE.to_vec().into(),
                                vrf_keyhash: VRF.to_vec().into(),
                            },
                        ),
                    ),
                    (
                        alonzo::Certificate::MoveInstantaneousRewardsCert(
                            alonzo::MoveInstantaneousReward {
                                source: alonzo::InstantaneousRewardSource::Reserves,
                                target: alonzo::InstantaneousRewardTarget::OtherAccountingPot(
                                    REWARD as u64,
                                ),
                            },
                        ),
                        u5c::certificate::Certificate::MirCert(u5c::MirCert {
                            from: u5c::MirSource::Reserves as i32,
                            to: Vec::new(),
                            other_pot: REWARD as u64,
                        }),
                    ),
                ];

                assert_eq!(
                    cases.len(),
                    7,
                    "the type serving Shelley through Babbage names seven certificates"
                );

                let raw = tx_with_redeemers(Vec::new());
                let tx = MultiEraTx::from_conway(&raw);
                let mapper = Mapper::new(NoLedger);

                for (order, (certificate, expected)) in cases.iter().enumerate() {
                    let cert = MultiEraCert::AlonzoCompatible(Box::new(Cow::Borrowed(certificate)));
                    let mapped = mapper
                        .map_cert(&cert, &tx, order as u32)
                        .expect("an Alonzo certificate maps");

                    assert_eq!(
                        mapped.certificate.as_ref(),
                        Some(expected),
                        "the Alonzo certificate at position {order}"
                    );
                    assert!(
                        mapped.redeemer.is_none(),
                        "position {order} pairs no redeemer, this transaction carries none"
                    );
                }
            }

            #[test]
            fn a_move_instantaneous_rewards_certificate_maps_the_pot_and_every_target() {
                let certificate = alonzo::Certificate::MoveInstantaneousRewardsCert(
                    alonzo::MoveInstantaneousReward {
                        source: alonzo::InstantaneousRewardSource::Treasury,
                        target: alonzo::InstantaneousRewardTarget::StakeCredentials(
                            BTreeMap::from([(credential(), REWARD)]),
                        ),
                    },
                );

                let raw = tx_with_redeemers(Vec::new());
                let tx = MultiEraTx::from_conway(&raw);
                let mapper = Mapper::new(NoLedger);
                let cert = MultiEraCert::AlonzoCompatible(Box::new(Cow::Borrowed(&certificate)));

                let mapped = mapper
                    .map_cert(&cert, &tx, 0)
                    .expect("an Alonzo certificate maps");

                assert_eq!(
                    mapped.certificate,
                    Some(u5c::certificate::Certificate::MirCert(u5c::MirCert {
                        from: u5c::MirSource::Treasury as i32,
                        to: vec![u5c::MirTarget {
                            stake_credential: Some(key_credential(CREDENTIAL)),
                            delta_coin: bigint(REWARD),
                        }],
                        other_pot: 0,
                    }))
                );
            }

            #[test]
            #[allow(clippy::needless_update)]
            fn a_pool_registration_read_from_a_block_maps_every_parameter() {
                let cbor = babbage10_block();
                let decoded = MultiEraBlock::decode(&cbor).expect("the fixture decodes");
                let txs = decoded.txs();
                let certs: Vec<_> = txs.iter().flat_map(|tx| tx.certs()).collect();
                assert_eq!(certs.len(), 1, "this fixture writes one certificate");

                let mapper = Mapper::new(NoLedger);
                let mapped = mapper
                    .map_cert(&certs[0], &txs[0], 0)
                    .expect("a pool registration maps");

                assert_eq!(
                    mapped.certificate,
                    Some(u5c::certificate::Certificate::PoolRegistration(
                        u5c::PoolRegistrationCert {
                            operator: hex::decode(
                                "129a187287eb6c65e57af2a1ac5750113ecc1a1e658b960358fcaa59"
                            )
                            .unwrap()
                            .into(),
                            vrf_keyhash: hex::decode(
                                "cf027ebfbfec5c3f964b05341519180003e2ed092829a402f775efec666d78e1"
                            )
                            .unwrap()
                            .into(),
                            // The pledge is above i64::MAX, which the schema
                            // carries as unsigned big endian bytes.
                            pledge: Some(u5c::BigInt {
                                big_int: Some(u5c::big_int::BigInt::BigUInt(
                                    hex::decode("8000000000000001").unwrap().into()
                                )),
                            }),
                            cost: bigint(340_000_000),
                            // The fixture's margin is 9223372036854775809 over
                            // 10000000000000000000 and the schema's two fields
                            // are 32 bits wide, so both arrive cut to their low
                            // 32 bits.
                            margin: Some(u5c::RationalNumber {
                                numerator: 1,
                                denominator: 2_313_682_944,
                            }),
                            reward_account: hex::decode(
                                "e0b04dff59ee3b964a7d9f4fda04d98ef43de3abc832112cc37a35d138"
                            )
                            .unwrap()
                            .into(),
                            pool_owners: vec![
                                hex::decode(
                                    "b04dff59ee3b964a7d9f4fda04d98ef43de3abc832112cc37a35d138"
                                )
                                .unwrap()
                                .into()
                            ],
                            relays: vec![
                                u5c::Relay {
                                    ip_v4: hex::decode("05a14bd4").unwrap().into(),
                                    ip_v6: Default::default(),
                                    dns_name: String::new(),
                                    port: 5003,
                                },
                                u5c::Relay {
                                    ip_v4: hex::decode("64646464").unwrap().into(),
                                    ip_v6: Default::default(),
                                    dns_name: String::new(),
                                    port: 100,
                                },
                                u5c::Relay {
                                    ip_v4: hex::decode("c8c8c8c8").unwrap().into(),
                                    ip_v6: Default::default(),
                                    dns_name: String::new(),
                                    port: 200,
                                },
                            ],
                            pool_metadata: Some(u5c::PoolMetadata {
                                url: "https://raw.githubusercontent.com/stakelovelace/pub/main/s2.json"
                                    .to_string(),
                                hash: hex::decode(
                                    "b3ac275b0568c3b7d63f889f896086fe4cb61d0f156cbfa18b5466a8480e012a"
                                )
                                .unwrap()
                                .into(),
                            }),
                            ..Default::default()
                        }
                    ))
                );
            }

            #[test]
            fn a_certificate_carries_the_redeemer_the_transaction_indexes_at_its_position() {
                let raw = tx_with_redeemers(vec![conway::Redeemer {
                    tag: conway::RedeemerTag::Cert,
                    index: 1,
                    data: pallas_primitives::PlutusData::BoundedBytes(vec![0x0f].into()),
                    ex_units: pallas_primitives::ExUnits {
                        mem: 11,
                        steps: 22,
                    },
                }]);
                let tx = MultiEraTx::from_conway(&raw);
                let mapper = Mapper::new(NoLedger);
                let certificate = conway::Certificate::StakeRegistration(credential());
                let cert = MultiEraCert::Conway(Box::new(Cow::Borrowed(&certificate)));

                let paired = mapper
                    .map_cert(&cert, &tx, 1)
                    .expect("a Conway certificate maps");
                let redeemer = paired
                    .redeemer
                    .expect("position one is indexed by a certificate redeemer");

                assert_eq!(redeemer.index, 1);
                assert_eq!(redeemer.purpose, u5c::RedeemerPurpose::Cert as i32);
                assert_eq!(
                    redeemer.ex_units,
                    Some(u5c::ExUnits {
                        memory: 11,
                        steps: 22,
                    })
                );

                let unpaired = mapper
                    .map_cert(&cert, &tx, 0)
                    .expect("a Conway certificate maps");
                assert!(
                    unpaired.redeemer.is_none(),
                    "position zero is indexed by no redeemer"
                );
            }
        }

        // ---- protocol parameters --------------------------------------------

        impl<C: $crate::LedgerContext> Mapper<C> {
            pub fn map_pparams(
                &self,
                pparams: pallas_validate::utils::MultiEraProtocolParameters,
            ) -> u5c::PParams {
                use pallas_primitives::alonzo::Language;
                use pallas_validate::utils::MultiEraProtocolParameters;
                match pparams {
                    MultiEraProtocolParameters::Alonzo(params) => u5c::PParams {
                        max_tx_size: params.max_transaction_size.into(),
                        max_block_body_size: params.max_block_body_size.into(),
                        max_block_header_size: params.max_block_header_size.into(),
                        min_fee_coefficient: u64_to_bigint(params.minfee_a.into()),
                        min_fee_constant: u64_to_bigint(params.minfee_b.into()),
                        coins_per_utxo_byte: u64_to_bigint(params.ada_per_utxo_byte),
                        stake_key_deposit: u64_to_bigint(params.key_deposit),
                        pool_deposit: u64_to_bigint(params.pool_deposit),
                        desired_number_of_pools: params.desired_number_of_stake_pools.into(),
                        pool_influence: Some(rational_number_to_u5c(params.pool_pledge_influence)),
                        monetary_expansion: Some(rational_number_to_u5c(params.expansion_rate)),
                        treasury_expansion: Some(rational_number_to_u5c(
                            params.treasury_growth_rate,
                        )),
                        min_pool_cost: u64_to_bigint(params.min_pool_cost),
                        protocol_version: Some(u5c::ProtocolVersion {
                            major: params.protocol_version.0 as u32,
                            minor: params.protocol_version.1 as u32,
                        }),
                        max_value_size: params.max_value_size.into(),
                        collateral_percentage: params.collateral_percentage.into(),
                        max_collateral_inputs: params.max_collateral_inputs.into(),
                        prices: Some(execution_prices_to_u5c(params.execution_costs)),
                        max_execution_units_per_transaction: Some(execution_units_to_u5c(
                            params.max_tx_ex_units,
                        )),
                        max_execution_units_per_block: Some(execution_units_to_u5c(
                            params.max_block_ex_units,
                        )),
                        cost_models: u5c::CostModels {
                            plutus_v1: params
                                .cost_models_for_script_languages
                                .get_key_value(&Language::PlutusV1)
                                .map(|(_, data)| u5c::CostModel {
                                    values: data.to_vec(),
                                }),
                            ..Default::default()
                        }
                        .into(),
                        ..Default::default()
                    },
                    MultiEraProtocolParameters::Shelley(params) => u5c::PParams {
                        max_tx_size: params.max_transaction_size.into(),
                        max_block_body_size: params.max_block_body_size.into(),
                        max_block_header_size: params.max_block_header_size.into(),
                        min_fee_coefficient: u64_to_bigint(params.minfee_a.into()),
                        min_fee_constant: u64_to_bigint(params.minfee_b.into()),
                        stake_key_deposit: u64_to_bigint(params.key_deposit),
                        pool_deposit: u64_to_bigint(params.pool_deposit),
                        desired_number_of_pools: params.desired_number_of_stake_pools.into(),
                        pool_influence: Some(rational_number_to_u5c(params.pool_pledge_influence)),
                        monetary_expansion: Some(rational_number_to_u5c(params.expansion_rate)),
                        treasury_expansion: Some(rational_number_to_u5c(
                            params.treasury_growth_rate,
                        )),
                        min_pool_cost: u64_to_bigint(params.min_pool_cost),
                        protocol_version: Some(u5c::ProtocolVersion {
                            major: params.protocol_version.0 as u32,
                            minor: params.protocol_version.1 as u32,
                        }),
                        ..Default::default()
                    },
                    MultiEraProtocolParameters::Babbage(params) => u5c::PParams {
                        max_tx_size: params.max_transaction_size.into(),
                        max_block_body_size: params.max_block_body_size.into(),
                        max_block_header_size: params.max_block_header_size.into(),
                        min_fee_coefficient: u64_to_bigint(params.minfee_a.into()),
                        min_fee_constant: u64_to_bigint(params.minfee_b.into()),
                        coins_per_utxo_byte: u64_to_bigint(params.ada_per_utxo_byte),
                        stake_key_deposit: u64_to_bigint(params.key_deposit),
                        pool_deposit: u64_to_bigint(params.pool_deposit),
                        desired_number_of_pools: params.desired_number_of_stake_pools.into(),
                        pool_influence: Some(rational_number_to_u5c(params.pool_pledge_influence)),
                        monetary_expansion: Some(rational_number_to_u5c(params.expansion_rate)),
                        treasury_expansion: Some(rational_number_to_u5c(
                            params.treasury_growth_rate,
                        )),
                        min_pool_cost: u64_to_bigint(params.min_pool_cost),
                        protocol_version: u5c::ProtocolVersion {
                            major: params.protocol_version.0 as u32,
                            minor: params.protocol_version.1 as u32,
                        }
                        .into(),
                        max_value_size: params.max_value_size.into(),
                        collateral_percentage: params.collateral_percentage.into(),
                        max_collateral_inputs: params.max_collateral_inputs.into(),
                        prices: Some(execution_prices_to_u5c(params.execution_costs)),
                        max_execution_units_per_transaction: Some(execution_units_to_u5c(
                            params.max_tx_ex_units,
                        )),
                        max_execution_units_per_block: Some(execution_units_to_u5c(
                            params.max_block_ex_units,
                        )),
                        cost_models: u5c::CostModels {
                            plutus_v1: params
                                .cost_models_for_script_languages
                                .plutus_v1
                                .map(|values| u5c::CostModel { values }),
                            plutus_v2: params
                                .cost_models_for_script_languages
                                .plutus_v2
                                .map(|values| u5c::CostModel { values }),
                            ..Default::default()
                        }
                        .into(),
                        ..Default::default()
                    },
                    MultiEraProtocolParameters::Byron(params) => u5c::PParams {
                        max_tx_size: params.max_tx_size,
                        max_block_body_size: params.max_block_size - params.max_header_size,
                        max_block_header_size: params.max_header_size,
                        ..Default::default()
                    },
                    MultiEraProtocolParameters::Conway(params) => u5c::PParams {
                        max_tx_size: params.max_transaction_size.into(),
                        max_block_body_size: params.max_block_body_size.into(),
                        max_block_header_size: params.max_block_header_size.into(),
                        min_fee_coefficient: u64_to_bigint(params.minfee_a.into()),
                        min_fee_constant: u64_to_bigint(params.minfee_b.into()),
                        coins_per_utxo_byte: u64_to_bigint(params.ada_per_utxo_byte),
                        stake_key_deposit: u64_to_bigint(params.key_deposit),
                        pool_deposit: u64_to_bigint(params.pool_deposit),
                        desired_number_of_pools: params.desired_number_of_stake_pools.into(),
                        pool_influence: Some(rational_number_to_u5c(params.pool_pledge_influence)),
                        monetary_expansion: Some(rational_number_to_u5c(params.expansion_rate)),
                        treasury_expansion: Some(rational_number_to_u5c(
                            params.treasury_growth_rate,
                        )),
                        min_pool_cost: u64_to_bigint(params.min_pool_cost),
                        protocol_version: u5c::ProtocolVersion {
                            major: params.protocol_version.0 as u32,
                            minor: params.protocol_version.1 as u32,
                        }
                        .into(),
                        max_value_size: params.max_value_size.into(),
                        collateral_percentage: params.collateral_percentage.into(),
                        max_collateral_inputs: params.max_collateral_inputs.into(),
                        prices: Some(execution_prices_to_u5c(params.execution_costs)),
                        max_execution_units_per_transaction: Some(execution_units_to_u5c(
                            params.max_tx_ex_units,
                        )),
                        max_execution_units_per_block: Some(execution_units_to_u5c(
                            params.max_block_ex_units,
                        )),
                        min_fee_script_ref_cost_per_byte: Some(rational_number_to_u5c(
                            params.minfee_refscript_cost_per_byte,
                        )),
                        pool_voting_thresholds: Some(u5c::VotingThresholds {
                            thresholds: vec![
                                rational_number_to_u5c(
                                    params.pool_voting_thresholds.motion_no_confidence,
                                ),
                                rational_number_to_u5c(
                                    params.pool_voting_thresholds.committee_normal,
                                ),
                                rational_number_to_u5c(
                                    params.pool_voting_thresholds.committee_no_confidence,
                                ),
                                rational_number_to_u5c(
                                    params.pool_voting_thresholds.hard_fork_initiation,
                                ),
                                rational_number_to_u5c(
                                    params.pool_voting_thresholds.security_voting_threshold,
                                ),
                            ],
                        }),
                        drep_voting_thresholds: Some(u5c::VotingThresholds {
                            thresholds: vec![
                                rational_number_to_u5c(
                                    params.drep_voting_thresholds.motion_no_confidence,
                                ),
                                rational_number_to_u5c(
                                    params.drep_voting_thresholds.committee_normal,
                                ),
                                rational_number_to_u5c(
                                    params.drep_voting_thresholds.committee_no_confidence,
                                ),
                                rational_number_to_u5c(
                                    params.drep_voting_thresholds.update_constitution,
                                ),
                                rational_number_to_u5c(
                                    params.drep_voting_thresholds.hard_fork_initiation,
                                ),
                                rational_number_to_u5c(
                                    params.drep_voting_thresholds.pp_network_group,
                                ),
                                rational_number_to_u5c(
                                    params.drep_voting_thresholds.pp_economic_group,
                                ),
                                rational_number_to_u5c(
                                    params.drep_voting_thresholds.pp_technical_group,
                                ),
                                rational_number_to_u5c(
                                    params.drep_voting_thresholds.pp_governance_group,
                                ),
                                rational_number_to_u5c(
                                    params.drep_voting_thresholds.treasury_withdrawal,
                                ),
                            ],
                        }),
                        min_committee_size: params.min_committee_size as u32,
                        committee_term_limit: params.committee_term_limit,
                        governance_action_validity_period: params.governance_action_validity_period,
                        governance_action_deposit: u64_to_bigint(params.governance_action_deposit),
                        drep_deposit: u64_to_bigint(params.drep_deposit),
                        drep_inactivity_period: params.drep_inactivity_period,
                        cost_models: u5c::CostModels {
                            plutus_v1: params
                                .cost_models_for_script_languages
                                .plutus_v1
                                .map(|values| u5c::CostModel { values }),
                            plutus_v2: params
                                .cost_models_for_script_languages
                                .plutus_v2
                                .map(|values| u5c::CostModel { values }),
                            plutus_v3: params
                                .cost_models_for_script_languages
                                .plutus_v3
                                .map(|values| u5c::CostModel { values }),
                            ..Default::default()
                        }
                        .into(),
                        ..Default::default()
                    },
                    _ => unimplemented!(),
                }
            }

            #[deprecated(since = "1.5.0", note = "use Mapper::map_pparams_update")]
            pub fn map_conway_pparams_update(
                &self,
                x: &pallas_primitives::conway::ProtocolParamUpdate,
            ) -> ParamUpdate {
                self.map_pparams_update(&pallas_traverse::MultiEraParamUpdate::Conway(
                    Box::new(std::borrow::Cow::Borrowed(x)),
                ))
            }

            pub fn map_pparams_update(
                &self,
                x: &pallas_traverse::MultiEraParamUpdate,
            ) -> ParamUpdate {
                let update = ParamUpdate {
                    coins_per_utxo_byte: x.ada_per_utxo_byte().and_then(u64_to_bigint),
                    max_tx_size: update_scalar(x.max_transaction_size()),
                    min_fee_coefficient: x.minfee_a().and_then(u64_to_bigint),
                    min_fee_constant: x.minfee_b().and_then(u64_to_bigint),
                    max_block_body_size: update_scalar(x.max_block_body_size()),
                    max_block_header_size: update_scalar(x.max_block_header_size()),
                    stake_key_deposit: x.key_deposit().and_then(u64_to_bigint),
                    pool_deposit: x.pool_deposit().and_then(u64_to_bigint),
                    pool_retirement_epoch_bound: update_scalar(x.maximum_epoch()),
                    desired_number_of_pools: update_scalar(x.desired_number_of_stake_pools()),
                    pool_influence: x.pool_pledge_influence().map(rational_number_to_u5c),
                    monetary_expansion: x.expansion_rate().map(rational_number_to_u5c),
                    treasury_expansion: x.treasury_growth_rate().map(rational_number_to_u5c),
                    min_pool_cost: x.min_pool_cost().and_then(u64_to_bigint),
                    max_value_size: update_scalar(x.max_value_size()),
                    collateral_percentage: update_scalar(x.collateral_percentage()),
                    max_collateral_inputs: update_scalar(x.max_collateral_inputs()),
                    cost_models: x.cost_models_for_script_languages().map(|cm| {
                        u5c::CostModels {
                            plutus_v1: cm.plutus_v1.map(|values| u5c::CostModel { values }),
                            plutus_v2: cm.plutus_v2.map(|values| u5c::CostModel { values }),
                            plutus_v3: cm.plutus_v3.map(|values| u5c::CostModel { values }),
                            plutus_v4: cm.plutus_v4.map(|values| u5c::CostModel { values }),
                        }
                    }),
                    prices: x.execution_costs().map(|p| u5c::ExPrices {
                        memory: Some(rational_number_to_u5c(p.mem_price)),
                        steps: Some(rational_number_to_u5c(p.step_price)),
                    }),
                    max_execution_units_per_transaction: x.max_tx_ex_units().map(|u| {
                        u5c::ExUnits {
                            memory: u.mem,
                            steps: u.steps,
                        }
                    }),
                    max_execution_units_per_block: x.max_block_ex_units().map(|u| u5c::ExUnits {
                        memory: u.mem,
                        steps: u.steps,
                    }),
                    min_fee_script_ref_cost_per_byte: x
                        .minfee_refscript_cost_per_byte()
                        .map(rational_number_to_u5c),
                    pool_voting_thresholds: x.pool_voting_thresholds().map(|t| {
                        u5c::VotingThresholds {
                            thresholds: vec![
                                rational_number_to_u5c(t.motion_no_confidence),
                                rational_number_to_u5c(t.committee_normal),
                                rational_number_to_u5c(t.committee_no_confidence),
                                rational_number_to_u5c(t.hard_fork_initiation),
                                rational_number_to_u5c(t.security_voting_threshold),
                            ],
                        }
                    }),
                    drep_voting_thresholds: x.drep_voting_thresholds().map(|t| {
                        u5c::VotingThresholds {
                            thresholds: vec![
                                rational_number_to_u5c(t.motion_no_confidence),
                                rational_number_to_u5c(t.committee_normal),
                                rational_number_to_u5c(t.committee_no_confidence),
                                rational_number_to_u5c(t.update_constitution),
                                rational_number_to_u5c(t.hard_fork_initiation),
                                rational_number_to_u5c(t.pp_network_group),
                                rational_number_to_u5c(t.pp_economic_group),
                                rational_number_to_u5c(t.pp_technical_group),
                                rational_number_to_u5c(t.pp_governance_group),
                                rational_number_to_u5c(t.treasury_withdrawal),
                            ],
                        }
                    }),
                    min_committee_size: update_scalar(x.min_committee_size().map(|x| x as u32)),
                    committee_term_limit: update_scalar(x.committee_term_limit()),
                    governance_action_validity_period: update_scalar(
                        x.governance_action_validity_period(),
                    ),
                    governance_action_deposit: x
                        .governance_action_deposit()
                        .and_then(u64_to_bigint),
                    drep_deposit: x.drep_deposit().and_then(u64_to_bigint),
                    drep_inactivity_period: update_scalar(x.drep_inactivity_period()),
                    ..Default::default()
                };

                with_dijkstra_update(update, x)
            }
        }
    };
}

pub(crate) use impl_cardano_mapper_shared;
