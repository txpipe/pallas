/// Emits the version-agnostic body of `Mapper<C: LedgerContext>` for a given
/// `utxorpc_spec::utxorpc::vXxxx::cardano` module path. Methods that diverge
/// between v1alpha and v1beta (`map_tx_datum`, `map_tx_output`, `map_asset`,
/// `map_policy_assets`, `map_conway_gov_action`, `map_tx`) are NOT emitted
/// here — each version's `mod.rs` defines them in a separate impl block.
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

        impl<C: $crate::LedgerContext> Mapper<C> {
            pub fn map_purpose(
                &self,
                x: &pallas_primitives::conway::RedeemerTag,
            ) -> u5c::RedeemerPurpose {
                use pallas_primitives::conway;
                match x {
                    conway::RedeemerTag::Spend => u5c::RedeemerPurpose::Spend,
                    conway::RedeemerTag::Mint => u5c::RedeemerPurpose::Mint,
                    conway::RedeemerTag::Cert => u5c::RedeemerPurpose::Cert,
                    conway::RedeemerTag::Reward => u5c::RedeemerPurpose::Reward,
                    conway::RedeemerTag::Vote => u5c::RedeemerPurpose::Vote,
                    conway::RedeemerTag::Propose => u5c::RedeemerPurpose::Propose,
                }
            }

            pub fn map_redeemer(&self, x: &pallas_traverse::MultiEraRedeemer) -> u5c::Redeemer {
                u5c::Redeemer {
                    purpose: self.map_purpose(&x.tag()).into(),
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

            pub fn map_any_script(&self, x: &pallas_primitives::conway::ScriptRef) -> u5c::Script {
                use pallas_primitives::conway;
                match x {
                    conway::ScriptRef::NativeScript(x) => u5c::Script {
                        script: u5c::script::Script::Native(Self::map_native_script(x)).into(),
                    },
                    conway::ScriptRef::PlutusV1Script(x) => u5c::Script {
                        script: u5c::script::Script::PlutusV1(x.0.to_vec().into()).into(),
                    },
                    conway::ScriptRef::PlutusV2Script(x) => u5c::Script {
                        script: u5c::script::Script::PlutusV2(x.0.to_vec().into()).into(),
                    },
                    conway::ScriptRef::PlutusV3Script(x) => u5c::Script {
                        script: u5c::script::Script::PlutusV3(x.0.to_vec().into()).into(),
                    },
                }
            }

            pub fn map_stake_credential(
                &self,
                x: &pallas_primitives::babbage::StakeCredential,
            ) -> u5c::StakeCredential {
                use pallas_primitives::babbage;
                let inner = match x {
                    babbage::StakeCredential::AddrKeyhash(x) => {
                        u5c::stake_credential::StakeCredential::AddrKeyHash(x.to_vec().into())
                    }
                    babbage::StakeCredential::ScriptHash(x) => {
                        u5c::stake_credential::StakeCredential::ScriptHash(x.to_vec().into())
                    }
                };

                u5c::StakeCredential {
                    stake_credential: inner.into(),
                }
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

            fn collect_all_scripts(&self, tx: &pallas_traverse::MultiEraTx) -> Vec<u5c::Script> {
                use std::ops::Deref;
                let ns = tx
                    .native_scripts()
                    .iter()
                    .map(|x| Self::map_native_script(x.deref()))
                    .map(|x| u5c::Script {
                        script: u5c::script::Script::Native(x).into(),
                    });

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
                let inner = match x {
                    babbage::PlutusData::Constr(x) => {
                        u5c::plutus_data::PlutusData::Constr(self.map_plutus_constr(x))
                    }
                    babbage::PlutusData::Map(x) => {
                        u5c::plutus_data::PlutusData::Map(self.map_plutus_map(x))
                    }
                    babbage::PlutusData::Array(x) => {
                        u5c::plutus_data::PlutusData::Array(self.map_plutus_array(x))
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
                    gov_action: x
                        .as_conway()
                        .map(|x| self.map_conway_gov_action(&x.gov_action)),
                    anchor: Some(u5c::Anchor {
                        url: x.anchor().url.clone(),
                        content_hash: x.anchor().content_hash.to_vec().into(),
                    }),
                }
            }

            pub fn map_metadatum(x: &pallas_primitives::alonzo::Metadatum) -> u5c::Metadatum {
                use pallas_primitives::babbage;
                let inner = match x {
                    babbage::Metadatum::Int(x) => {
                        u5c::metadatum::Metadatum::Int(i128::from(x.0) as i64)
                    }
                    babbage::Metadatum::Bytes(x) => {
                        u5c::metadatum::Metadatum::Bytes(Vec::<u8>::from(x.clone()).into())
                    }
                    babbage::Metadatum::Text(x) => u5c::metadatum::Metadatum::Text(x.clone()),
                    babbage::Metadatum::Array(x) => {
                        u5c::metadatum::Metadatum::Array(u5c::MetadatumArray {
                            items: x.iter().map(|x| Self::map_metadatum(x)).collect(),
                        })
                    }
                    babbage::Metadatum::Map(x) => {
                        u5c::metadatum::Metadatum::Map(u5c::MetadatumMap {
                            pairs: x
                                .iter()
                                .map(|(k, v)| u5c::MetadatumPair {
                                    key: Self::map_metadatum(k).into(),
                                    value: Self::map_metadatum(v).into(),
                                })
                                .collect(),
                        })
                    }
                };

                u5c::Metadatum {
                    metadatum: inner.into(),
                }
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
                    .aux_native_scripts()
                    .iter()
                    .map(|x| Self::map_native_script(x))
                    .map(|x| u5c::Script {
                        script: u5c::script::Script::Native(x).into(),
                    });

                let p1 = tx
                    .aux_plutus_v1_scripts()
                    .iter()
                    .map(|x| x.0.to_vec().into())
                    .map(|x| u5c::Script {
                        script: u5c::script::Script::PlutusV1(x).into(),
                    });

                ns.chain(p1).collect()
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

            pub fn map_block(&self, block: &pallas_traverse::MultiEraBlock) -> u5c::Block {
                u5c::Block {
                    header: u5c::BlockHeader {
                        slot: block.slot(),
                        hash: block.hash().to_vec().into(),
                        height: block.number(),
                    }
                    .into(),
                    body: u5c::BlockBody {
                        tx: block.txs().iter().map(|x| self.map_tx(x)).collect(),
                    }
                    .into(),
                    timestamp: self
                        .ledger
                        .as_ref()
                        .and_then(|ledger| ledger.get_slot_timestamp(block.slot()))
                        .unwrap_or(0),
                }
            }

            pub fn map_block_cbor(&self, raw: &[u8]) -> u5c::Block {
                let block = pallas_traverse::MultiEraBlock::decode(raw).unwrap();
                self.map_block(&block)
            }
        }

        // ---- certificates ----------------------------------------------------

        impl<C: $crate::LedgerContext> Mapper<C> {
            pub fn map_alonzo_compatible_cert(
                &self,
                x: &pallas_primitives::alonzo::Certificate,
                tx: &pallas_traverse::MultiEraTx,
                order: u32,
            ) -> u5c::Certificate {
                use pallas_primitives::{alonzo, babbage};
                let inner = match x {
                    alonzo::Certificate::StakeRegistration(a) => {
                        u5c::certificate::Certificate::StakeRegistration(
                            self.map_stake_credential(a),
                        )
                    }
                    alonzo::Certificate::StakeDeregistration(a) => {
                        u5c::certificate::Certificate::StakeDeregistration(
                            self.map_stake_credential(a),
                        )
                    }
                    alonzo::Certificate::StakeDelegation(a, b) => {
                        u5c::certificate::Certificate::StakeDelegation(u5c::StakeDelegationCert {
                            stake_credential: self.map_stake_credential(a).into(),
                            pool_keyhash: b.to_vec().into(),
                        })
                    }
                    alonzo::Certificate::PoolRegistration {
                        operator,
                        vrf_keyhash,
                        pledge,
                        cost,
                        margin,
                        reward_account,
                        pool_owners,
                        relays,
                        pool_metadata,
                    } => {
                        u5c::certificate::Certificate::PoolRegistration(u5c::PoolRegistrationCert {
                            operator: operator.to_vec().into(),
                            vrf_keyhash: vrf_keyhash.to_vec().into(),
                            pledge: u64_to_bigint(*pledge),
                            cost: u64_to_bigint(*cost),
                            margin: u5c::RationalNumber {
                                numerator: margin.numerator as i32,
                                denominator: margin.denominator as u32,
                            }
                            .into(),
                            reward_account: reward_account.to_vec().into(),
                            pool_owners: pool_owners.iter().map(|x| x.to_vec().into()).collect(),
                            relays: relays.iter().map(|x| self.map_relay(x)).collect(),
                            pool_metadata: pool_metadata.clone().map(|x| u5c::PoolMetadata {
                                url: x.url.clone(),
                                hash: x.hash.to_vec().into(),
                            }),
                        })
                    }
                    alonzo::Certificate::PoolRetirement(a, b) => {
                        u5c::certificate::Certificate::PoolRetirement(u5c::PoolRetirementCert {
                            pool_keyhash: a.to_vec().into(),
                            epoch: *b,
                        })
                    }
                    alonzo::Certificate::GenesisKeyDelegation(a, b, c) => {
                        u5c::certificate::Certificate::GenesisKeyDelegation(
                            u5c::GenesisKeyDelegationCert {
                                genesis_hash: a.to_vec().into(),
                                genesis_delegate_hash: b.to_vec().into(),
                                vrf_keyhash: c.to_vec().into(),
                            },
                        )
                    }
                    alonzo::Certificate::MoveInstantaneousRewardsCert(a) => {
                        u5c::certificate::Certificate::MirCert(u5c::MirCert {
                            from: match &a.source {
                                babbage::InstantaneousRewardSource::Reserves => {
                                    u5c::MirSource::Reserves.into()
                                }
                                babbage::InstantaneousRewardSource::Treasury => {
                                    u5c::MirSource::Treasury.into()
                                }
                            },
                            to: match &a.target {
                                babbage::InstantaneousRewardTarget::StakeCredentials(x) => x
                                    .iter()
                                    .map(|(k, v)| u5c::MirTarget {
                                        stake_credential: self.map_stake_credential(k).into(),
                                        delta_coin: i64_to_bigint(*v),
                                    })
                                    .collect(),
                                _ => Default::default(),
                            },
                            other_pot: match &a.target {
                                babbage::InstantaneousRewardTarget::OtherAccountingPot(x) => *x,
                                _ => Default::default(),
                            },
                        })
                    }
                };

                u5c::Certificate {
                    certificate: inner.into(),
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

            pub fn map_conway_cert(
                &self,
                x: &pallas_primitives::conway::Certificate,
                tx: &pallas_traverse::MultiEraTx,
                order: u32,
            ) -> u5c::Certificate {
                use pallas_primitives::conway;
                let inner = match x {
                    conway::Certificate::StakeRegistration(a) => {
                        u5c::certificate::Certificate::StakeRegistration(
                            self.map_stake_credential(a),
                        )
                    }
                    conway::Certificate::StakeDeregistration(a) => {
                        u5c::certificate::Certificate::StakeDeregistration(
                            self.map_stake_credential(a),
                        )
                    }
                    conway::Certificate::StakeDelegation(a, b) => {
                        u5c::certificate::Certificate::StakeDelegation(u5c::StakeDelegationCert {
                            stake_credential: self.map_stake_credential(a).into(),
                            pool_keyhash: b.to_vec().into(),
                        })
                    }
                    conway::Certificate::PoolRegistration {
                        operator,
                        vrf_keyhash,
                        pledge,
                        cost,
                        margin,
                        reward_account,
                        pool_owners,
                        relays,
                        pool_metadata,
                    } => {
                        u5c::certificate::Certificate::PoolRegistration(u5c::PoolRegistrationCert {
                            operator: operator.to_vec().into(),
                            vrf_keyhash: vrf_keyhash.to_vec().into(),
                            pledge: u64_to_bigint(*pledge),
                            cost: u64_to_bigint(*cost),
                            margin: u5c::RationalNumber {
                                numerator: margin.numerator as i32,
                                denominator: margin.denominator as u32,
                            }
                            .into(),
                            reward_account: reward_account.to_vec().into(),
                            pool_owners: pool_owners.iter().map(|x| x.to_vec().into()).collect(),
                            relays: relays.iter().map(|x| self.map_relay(x)).collect(),
                            pool_metadata: pool_metadata.clone().map(|x| u5c::PoolMetadata {
                                url: x.url.clone(),
                                hash: x.hash.to_vec().into(),
                            }),
                        })
                    }
                    conway::Certificate::PoolRetirement(a, b) => {
                        u5c::certificate::Certificate::PoolRetirement(u5c::PoolRetirementCert {
                            pool_keyhash: a.to_vec().into(),
                            epoch: *b,
                        })
                    }
                    conway::Certificate::Reg(cred, coin) => {
                        u5c::certificate::Certificate::RegCert(u5c::RegCert {
                            stake_credential: self.map_stake_credential(cred).into(),
                            coin: u64_to_bigint(*coin),
                        })
                    }
                    conway::Certificate::UnReg(cred, coin) => {
                        u5c::certificate::Certificate::UnregCert(u5c::UnRegCert {
                            stake_credential: self.map_stake_credential(cred).into(),
                            coin: u64_to_bigint(*coin),
                        })
                    }
                    conway::Certificate::VoteDeleg(cred, drep) => {
                        u5c::certificate::Certificate::VoteDelegCert(u5c::VoteDelegCert {
                            stake_credential: self.map_stake_credential(cred).into(),
                            drep: self.map_drep(drep).into(),
                        })
                    }
                    conway::Certificate::StakeVoteDeleg(stake_cred, pool_id, drep) => {
                        u5c::certificate::Certificate::StakeVoteDelegCert(u5c::StakeVoteDelegCert {
                            stake_credential: self.map_stake_credential(stake_cred).into(),
                            pool_keyhash: pool_id.to_vec().into(),
                            drep: self.map_drep(drep).into(),
                        })
                    }
                    conway::Certificate::StakeRegDeleg(stake_cred, pool_id, coin) => {
                        u5c::certificate::Certificate::StakeRegDelegCert(u5c::StakeRegDelegCert {
                            stake_credential: self.map_stake_credential(stake_cred).into(),
                            pool_keyhash: pool_id.to_vec().into(),
                            coin: u64_to_bigint(*coin),
                        })
                    }
                    conway::Certificate::VoteRegDeleg(vote_cred, drep, coin) => {
                        u5c::certificate::Certificate::VoteRegDelegCert(u5c::VoteRegDelegCert {
                            stake_credential: self.map_stake_credential(vote_cred).into(),
                            drep: self.map_drep(drep).into(),
                            coin: u64_to_bigint(*coin),
                        })
                    }
                    conway::Certificate::StakeVoteRegDeleg(stake_cred, pool_id, drep, coin) => {
                        u5c::certificate::Certificate::StakeVoteRegDelegCert(
                            u5c::StakeVoteRegDelegCert {
                                stake_credential: self.map_stake_credential(stake_cred).into(),
                                pool_keyhash: pool_id.to_vec().into(),
                                drep: self.map_drep(drep).into(),
                                coin: u64_to_bigint(*coin),
                            },
                        )
                    }
                    conway::Certificate::AuthCommitteeHot(cold_cred, hot_cred) => {
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
                    conway::Certificate::ResignCommitteeCold(cold_cred, anchor) => {
                        u5c::certificate::Certificate::ResignCommitteeColdCert(
                            u5c::ResignCommitteeColdCert {
                                committee_cold_credential: self
                                    .map_stake_credential(cold_cred)
                                    .into(),
                                anchor: anchor.clone().map(|a| u5c::Anchor {
                                    url: a.url,
                                    content_hash: a.content_hash.to_vec().into(),
                                }),
                            },
                        )
                    }
                    conway::Certificate::RegDRepCert(cred, coin, anchor) => {
                        u5c::certificate::Certificate::RegDrepCert(u5c::RegDRepCert {
                            drep_credential: self.map_stake_credential(cred).into(),
                            coin: u64_to_bigint(*coin),
                            anchor: anchor.clone().map(|a| u5c::Anchor {
                                url: a.url,
                                content_hash: a.content_hash.to_vec().into(),
                            }),
                        })
                    }
                    conway::Certificate::UnRegDRepCert(cred, coin) => {
                        u5c::certificate::Certificate::UnregDrepCert(u5c::UnRegDRepCert {
                            drep_credential: self.map_stake_credential(cred).into(),
                            coin: u64_to_bigint(*coin),
                        })
                    }
                    conway::Certificate::UpdateDRepCert(cred, anchor) => {
                        u5c::certificate::Certificate::UpdateDrepCert(u5c::UpdateDRepCert {
                            drep_credential: self.map_stake_credential(cred).into(),
                            anchor: anchor.clone().map(|a| u5c::Anchor {
                                url: a.url,
                                content_hash: a.content_hash.to_vec().into(),
                            }),
                        })
                    }
                };

                u5c::Certificate {
                    certificate: inner.into(),
                    redeemer: tx
                        .find_certificate_redeemer(order)
                        .map(|r| self.map_redeemer(&r)),
                }
            }

            pub fn map_cert(
                &self,
                x: &pallas_traverse::MultiEraCert,
                tx: &pallas_traverse::MultiEraTx,
                order: u32,
            ) -> Option<u5c::Certificate> {
                match x {
                    pallas_traverse::MultiEraCert::AlonzoCompatible(x) => {
                        self.map_alonzo_compatible_cert(x, tx, order).into()
                    }
                    pallas_traverse::MultiEraCert::Conway(x) => {
                        self.map_conway_cert(x, tx, order).into()
                    }
                    _ => None,
                }
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

            pub fn map_conway_pparams_update(
                &self,
                x: &pallas_primitives::conway::ProtocolParamUpdate,
            ) -> u5c::PParams {
                u5c::PParams {
                    coins_per_utxo_byte: x.ada_per_utxo_byte.and_then(u64_to_bigint),
                    max_tx_size: x.max_transaction_size.unwrap_or_default(),
                    min_fee_coefficient: x.minfee_a.and_then(u64_to_bigint),
                    min_fee_constant: x.minfee_b.and_then(u64_to_bigint),
                    max_block_body_size: x.max_block_body_size.unwrap_or_default(),
                    max_block_header_size: x.max_block_header_size.unwrap_or_default(),
                    stake_key_deposit: x.key_deposit.and_then(u64_to_bigint),
                    pool_deposit: x.pool_deposit.and_then(u64_to_bigint),
                    pool_retirement_epoch_bound: x.maximum_epoch.unwrap_or_default(),
                    desired_number_of_pools: x.desired_number_of_stake_pools.unwrap_or_default(),
                    pool_influence: x.pool_pledge_influence.clone().map(rational_number_to_u5c),
                    monetary_expansion: x.expansion_rate.clone().map(rational_number_to_u5c),
                    treasury_expansion: x.treasury_growth_rate.clone().map(rational_number_to_u5c),
                    min_pool_cost: x.min_pool_cost.and_then(u64_to_bigint),
                    protocol_version: None,
                    max_value_size: x.max_value_size.unwrap_or_default(),
                    collateral_percentage: x.collateral_percentage.unwrap_or_default(),
                    max_collateral_inputs: x.max_collateral_inputs.unwrap_or_default(),
                    cost_models: x.cost_models_for_script_languages.clone().map(|cm| {
                        u5c::CostModels {
                            plutus_v1: cm.plutus_v1.map(|values| u5c::CostModel { values }),
                            plutus_v2: cm.plutus_v2.map(|values| u5c::CostModel { values }),
                            plutus_v3: cm.plutus_v3.map(|values| u5c::CostModel { values }),
                            ..Default::default()
                        }
                    }),
                    prices: x.execution_costs.clone().map(|p| u5c::ExPrices {
                        memory: Some(rational_number_to_u5c(p.mem_price)),
                        steps: Some(rational_number_to_u5c(p.step_price)),
                    }),
                    max_execution_units_per_transaction: x.max_tx_ex_units.map(|u| u5c::ExUnits {
                        memory: u.mem,
                        steps: u.steps,
                    }),
                    max_execution_units_per_block: x.max_block_ex_units.map(|u| u5c::ExUnits {
                        memory: u.mem,
                        steps: u.steps,
                    }),
                    min_fee_script_ref_cost_per_byte: x
                        .minfee_refscript_cost_per_byte
                        .clone()
                        .map(rational_number_to_u5c),
                    pool_voting_thresholds: x.pool_voting_thresholds.clone().map(|t| {
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
                    drep_voting_thresholds: x.drep_voting_thresholds.clone().map(|t| {
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
                    min_committee_size: x.min_committee_size.unwrap_or_default() as u32,
                    committee_term_limit: x.committee_term_limit.unwrap_or_default(),
                    governance_action_validity_period: x
                        .governance_action_validity_period
                        .unwrap_or_default(),
                    governance_action_deposit: x.governance_action_deposit.and_then(u64_to_bigint),
                    drep_deposit: x.drep_deposit.and_then(u64_to_bigint),
                    drep_inactivity_period: x.drep_inactivity_period.unwrap_or_default(),
                }
            }
        }
    };
}

pub(crate) use impl_cardano_mapper_shared;
