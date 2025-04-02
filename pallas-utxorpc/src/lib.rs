use std::{collections::HashMap, ops::Deref};

use pallas_codec::utils::KeyValuePairs;
use pallas_crypto::hash::Hash;
use pallas_primitives::{alonzo, babbage, conway};
use pallas_traverse::{self as trv};

use prost_types::FieldMask;
use trv::OriginalHash;

pub use utxorpc_spec::utxorpc::v1alpha as spec;

use utxorpc_spec::utxorpc::v1alpha::cardano as u5c;

mod certs;
mod params;

pub type TxHash = Hash<32>;
pub type TxoIndex = u32;
pub type TxoRef = (TxHash, TxoIndex);
pub type Cbor = Vec<u8>;
pub type EraCbor = (trv::Era, Cbor);
pub type UtxoMap = HashMap<TxoRef, EraCbor>;
pub type DatumMap = HashMap<Hash<32>, alonzo::PlutusData>;

fn rational_number_to_u5c(value: pallas_primitives::RationalNumber) -> u5c::RationalNumber {
    u5c::RationalNumber {
        numerator: value.numerator as i32,
        denominator: value.denominator as u32,
    }
}

pub trait LedgerContext: Clone {
    fn get_utxos(&self, refs: &[TxoRef]) -> Option<UtxoMap>;
}

#[derive(Default, Clone)]
pub struct Mapper<C: LedgerContext> {
    ledger: Option<C>,
    _mask: FieldMask,
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

impl<C: LedgerContext> Mapper<C> {
    pub fn map_purpose(&self, x: &conway::RedeemerTag) -> u5c::RedeemerPurpose {
        match x {
            conway::RedeemerTag::Spend => u5c::RedeemerPurpose::Spend,
            conway::RedeemerTag::Mint => u5c::RedeemerPurpose::Mint,
            conway::RedeemerTag::Cert => u5c::RedeemerPurpose::Cert,
            conway::RedeemerTag::Reward => u5c::RedeemerPurpose::Reward,
            conway::RedeemerTag::Vote => u5c::RedeemerPurpose::Vote,
            conway::RedeemerTag::Propose => u5c::RedeemerPurpose::Propose,
        }
    }

    pub fn map_redeemer(&self, x: &trv::MultiEraRedeemer) -> u5c::Redeemer {
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
        resolved: &Option<UtxoMap>,
        input: &trv::MultiEraInput,
        tx: &trv::MultiEraTx,
    ) -> Option<u5c::TxOutput> {
        let as_txref = (*input.hash(), input.index() as u32);

        resolved
            .as_ref()
            .and_then(|x| x.get(&as_txref))
            .and_then(|(era, cbor)| {
                let o = trv::MultiEraOutput::decode(*era, cbor.as_slice()).ok()?;
                Some(self.map_tx_output(&o, Some(tx)))
            })
    }

    pub fn map_tx_input(
        &self,
        input: &trv::MultiEraInput,
        tx: &trv::MultiEraTx,
        // lexicographical order of the input we're mapping
        order: u32,
        resolved: &Option<UtxoMap>,
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
        input: &trv::MultiEraInput,
        resolved: &Option<UtxoMap>,
        tx: &trv::MultiEraTx,
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
        input: &trv::MultiEraInput,
        resolved: &Option<UtxoMap>,
        tx: &trv::MultiEraTx,
    ) -> u5c::TxInput {
        u5c::TxInput {
            tx_hash: input.hash().to_vec().into(),
            output_index: input.index() as u32,
            as_output: self.decode_resolved_utxo(resolved, input, tx),
            redeemer: None,
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
                Some(babbage::DatumOption::Data(x)) => x.raw_cbor().to_vec().into(),
                _ => vec![].into(),
            },
        }
    }

    pub fn map_any_script(&self, x: &conway::MintedScriptRef) -> u5c::Script {
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

    pub fn map_tx_output(
        &self,
        x: &trv::MultiEraOutput,
        tx: Option<&trv::MultiEraTx>,
    ) -> u5c::TxOutput {
        u5c::TxOutput {
            address: x.address().map(|a| a.to_vec()).unwrap_or_default().into(),
            coin: x.value().coin(),
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
        }
    }

    pub fn map_stake_credential(&self, x: &babbage::StakeCredential) -> u5c::StakeCredential {
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

    pub fn map_relay(&self, x: &alonzo::Relay) -> u5c::Relay {
        match x {
            babbage::Relay::SingleHostAddr(port, v4, v6) => u5c::Relay {
                // ip_v4: v4.map(|x| x.to_vec().into()).into().unwrap_or_default(),
                ip_v4: Option::from(v4.clone().map(|x| x.to_vec().into())).unwrap_or_default(),
                ip_v6: Option::from(v6.clone().map(|x| x.to_vec().into())).unwrap_or_default(),
                dns_name: String::default(),
                port: Option::from(port.clone()).unwrap_or_default(),
            },
            babbage::Relay::SingleHostName(port, name) => u5c::Relay {
                ip_v4: Default::default(),
                ip_v6: Default::default(),
                dns_name: name.clone(),
                port: Option::from(port.clone()).unwrap_or_default(),
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
        tx: &trv::MultiEraTx,
        order: u32,
    ) -> u5c::Withdrawal {
        u5c::Withdrawal {
            reward_account: Vec::from(x.0).into(),
            coin: x.1,
            redeemer: tx
                .find_withdrawal_redeemer(order)
                .map(|x| self.map_redeemer(&x)),
        }
    }

    pub fn map_asset(&self, x: &trv::MultiEraAsset) -> u5c::Asset {
        u5c::Asset {
            name: x.name().to_vec().into(),
            output_coin: x.output_coin().unwrap_or_default(),
            mint_coin: x.mint_coin().unwrap_or_default(),
        }
    }

    pub fn map_policy_assets(&self, x: &trv::MultiEraPolicyAssets) -> u5c::Multiasset {
        u5c::Multiasset {
            policy_id: x.policy().to_vec().into(),
            assets: x.assets().iter().map(|x| self.map_asset(x)).collect(),
            redeemer: None,
        }
    }

    pub fn map_vkey_witness(&self, x: &alonzo::VKeyWitness) -> u5c::VKeyWitness {
        u5c::VKeyWitness {
            vkey: x.vkey.to_vec().into(),
            signature: x.signature.to_vec().into(),
        }
    }

    pub fn map_native_script(x: &alonzo::NativeScript) -> u5c::NativeScript {
        let inner = match x {
            babbage::NativeScript::ScriptPubkey(x) => {
                u5c::native_script::NativeScript::ScriptPubkey(x.to_vec().into())
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

    fn collect_all_scripts(&self, tx: &trv::MultiEraTx) -> Vec<u5c::Script> {
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

    pub fn map_plutus_constr(&self, x: &alonzo::Constr<alonzo::PlutusData>) -> u5c::Constr {
        u5c::Constr {
            tag: x.tag as u32,
            any_constructor: x.any_constructor.unwrap_or_default(),
            fields: x.fields.iter().map(|x| self.map_plutus_datum(x)).collect(),
        }
    }

    pub fn map_plutus_map(
        &self,
        x: &KeyValuePairs<alonzo::PlutusData, alonzo::PlutusData>,
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

    pub fn map_plutus_array(&self, x: &[alonzo::PlutusData]) -> u5c::PlutusDataArray {
        u5c::PlutusDataArray {
            items: x.iter().map(|x| self.map_plutus_datum(x)).collect(),
        }
    }

    pub fn map_plutus_bigint(&self, x: &alonzo::BigInt) -> u5c::BigInt {
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

    pub fn map_plutus_datum(&self, x: &alonzo::PlutusData) -> u5c::PlutusData {
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
        x: &Option<conway::GovActionId>,
    ) -> Option<u5c::GovernanceActionId> {
        x.as_ref().map(|inner|
              u5c::GovernanceActionId {
                  transaction_id: inner.transaction_id.to_vec().into(),
                  governance_action_index: inner.action_index,
              }
        )
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
                                coin: *v,
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
                u5c::governance_action::GovernanceAction::InfoAction(6) // The 6 is just a placeholder, we don't need to use it
            }
        };

        u5c::GovernanceAction {
            governance_action: Some(inner),
        }
    }

    pub fn map_gov_proposal(&self, x: &trv::MultiEraProposal) -> u5c::GovernanceActionProposal {
        u5c::GovernanceActionProposal {
            deposit: x.deposit(),
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

    pub fn map_metadatum(x: &alonzo::Metadatum) -> u5c::Metadatum {
        let inner = match x {
            babbage::Metadatum::Int(x) => u5c::metadatum::Metadatum::Int(i128::from(x.0) as i64),
            babbage::Metadatum::Bytes(x) => {
                u5c::metadatum::Metadatum::Bytes(Vec::<u8>::from(x.clone()).into())
            }
            babbage::Metadatum::Text(x) => u5c::metadatum::Metadatum::Text(x.clone()),
            babbage::Metadatum::Array(x) => u5c::metadatum::Metadatum::Array(u5c::MetadatumArray {
                items: x.iter().map(|x| Self::map_metadatum(x)).collect(),
            }),
            babbage::Metadatum::Map(x) => u5c::metadatum::Metadatum::Map(u5c::MetadatumMap {
                pairs: x
                    .iter()
                    .map(|(k, v)| u5c::MetadatumPair {
                        key: Self::map_metadatum(k).into(),
                        value: Self::map_metadatum(v).into(),
                    })
                    .collect(),
            }),
        };

        u5c::Metadatum {
            metadatum: inner.into(),
        }
    }

    pub fn map_metadata(&self, label: u64, datum: &alonzo::Metadatum) -> u5c::Metadata {
        u5c::Metadata {
            label,
            value: Self::map_metadatum(datum).into(),
        }
    }

    fn collect_all_aux_scripts(&self, tx: &trv::MultiEraTx) -> Vec<u5c::Script> {
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

        // TODO: check why we don't have plutus v2 aux script, is that a possibility?

        ns.chain(p1).collect()
    }

    fn find_related_inputs(&self, tx: &trv::MultiEraTx) -> Vec<TxoRef> {
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
                total_collateral: tx.total_collateral().unwrap_or_default(),
            }
            .into(),
            fee: tx.fee().unwrap_or_default(),
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

    pub fn map_block(&self, block: &trv::MultiEraBlock) -> u5c::Block {
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
        }
    }

    pub fn map_block_cbor(&self, raw: &[u8]) -> u5c::Block {
        let block = trv::MultiEraBlock::decode(raw).unwrap();
        self.map_block(&block)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[derive(Clone)]
    struct NoLedger;

    impl LedgerContext for NoLedger {
        fn get_utxos(&self, _refs: &[TxoRef]) -> Option<UtxoMap> {
            None
        }
    }

    #[test]
    fn snapshot() {
        let test_blocks = [include_str!("../../test_data/u5c1.block")];
        let test_snapshots = [include_str!("../../test_data/u5c1.json")];

        let mapper = Mapper::new(NoLedger);

        for (block_str, json_str) in test_blocks.iter().zip(test_snapshots) {
            let cbor = hex::decode(block_str).unwrap();
            let block = pallas_traverse::MultiEraBlock::decode(&cbor).unwrap();
            let current = serde_json::json!(mapper.map_block(&block));

            // un-comment the following to generate a new snapshot

            // std::fs::write(
            //     "new_snapshot.json",
            //     serde_json::to_string_pretty(&current).unwrap(),
            // )
            // .unwrap();

            let expected: serde_json::Value = serde_json::from_str(json_str).unwrap();

            assert_eq!(expected, current)
        }
    }
}
