use pallas_configs::{alonzo, byron, conway, shelley};
use pallas_validate::utils::MultiEraProtocolParameters;
use utxorpc_spec::utxorpc::v1alpha::cardano as u5c;

use crate::{LedgerContext, Mapper};

impl<C: LedgerContext> Mapper<C> {
    /// Map genesis configuration to UTxO RPC Genesis type
    pub fn map_genesis(
        &self,
        byron: &byron::GenesisFile,
        shelley: &shelley::GenesisFile,
        alonzo: &alonzo::GenesisFile,
        conway: &conway::GenesisFile,
        current_params: Option<MultiEraProtocolParameters>,
    ) -> u5c::Genesis {
        u5c::Genesis {
            // Shelley data
            network_magic: shelley.network_magic.unwrap_or(0),
            network_id: shelley.network_id.clone().unwrap_or_default(),
            epoch_length: shelley.epoch_length.unwrap_or(0),
            slot_length: shelley.slot_length.unwrap_or(0),
            security_param: shelley.security_param.unwrap_or(0),
            system_start: shelley.system_start.clone().unwrap_or_default(),
            max_lovelace_supply: shelley.max_lovelace_supply.unwrap_or(0),
            max_kes_evolutions: shelley.max_kes_evolutions.unwrap_or(0),
            slots_per_kes_period: shelley.slots_per_kes_period.unwrap_or(0),
            update_quorum: shelley.update_quorum.unwrap_or(0),
            initial_funds: shelley.initial_funds.clone().unwrap_or_default(),

            // Alonzo data
            lovelace_per_utxo_word: alonzo.lovelace_per_utxo_word,
            max_value_size: alonzo.max_value_size,
            collateral_percentage: alonzo.collateral_percentage,
            max_collateral_inputs: alonzo.max_collateral_inputs,

            // Conway data
            committee_min_size: conway.committee_min_size as u64,
            committee_max_term_length: conway.committee_max_term_length as u64,
            gov_action_lifetime: conway.gov_action_lifetime as u64,
            gov_action_deposit: conway.gov_action_deposit,
            drep_deposit: conway.d_rep_deposit,
            drep_activity: conway.d_rep_activity as u64,

            // Byron data
            start_time: byron.start_time,
            boot_stakeholders: byron
                .boot_stakeholders
                .iter()
                .map(|(k, v)| (k.clone(), *v as u64))
                .collect(),
            avvm_distr: byron.avvm_distr.clone(),
            non_avvm_balances: byron.non_avvm_balances.clone(),

            // Map complex nested structures
            active_slots_coeff: shelley.active_slots_coeff.map(|coeff| u5c::RationalNumber {
                numerator: (coeff * 1000.0) as i32,
                denominator: 1000,
            }),

            gen_delegs: shelley
                .gen_delegs
                .as_ref()
                .map(|delegs| {
                    delegs
                        .iter()
                        .map(|(k, v)| {
                            (
                                k.clone(),
                                u5c::GenDelegs {
                                    delegate: v.delegate.clone().unwrap_or_default(),
                                    vrf: v.vrf.clone().unwrap_or_default(),
                                },
                            )
                        })
                        .collect()
                })
                .unwrap_or_default(),

            // Alonzo execution prices
            execution_prices: Some(u5c::ExPrices {
                steps: Some(u5c::RationalNumber {
                    numerator: alonzo.execution_prices.pr_steps.numerator as i32,
                    denominator: alonzo.execution_prices.pr_steps.denominator as u32,
                }),
                memory: Some(u5c::RationalNumber {
                    numerator: alonzo.execution_prices.pr_mem.numerator as i32,
                    denominator: alonzo.execution_prices.pr_mem.denominator as u32,
                }),
            }),

            // Alonzo execution units
            max_tx_ex_units: Some(u5c::ExUnits {
                memory: alonzo.max_tx_ex_units.ex_units_mem,
                steps: alonzo.max_tx_ex_units.ex_units_steps,
            }),
            max_block_ex_units: Some(u5c::ExUnits {
                memory: alonzo.max_block_ex_units.ex_units_mem,
                steps: alonzo.max_block_ex_units.ex_units_steps,
            }),

            // Conway governance structures
            committee: Some(u5c::Committee {
                members: conway.committee.members.clone(),
                threshold: Some(u5c::RationalNumber {
                    numerator: conway.committee.threshold.numerator as i32,
                    denominator: conway.committee.threshold.denominator as u32,
                }),
            }),
            constitution: Some(u5c::Constitution {
                anchor: Some(u5c::Anchor {
                    url: conway.constitution.anchor.url.clone(),
                    content_hash: conway.constitution.anchor.data_hash.clone().into(),
                }),
                hash: conway
                    .constitution
                    .script
                    .clone()
                    .unwrap_or_default()
                    .into(),
            }),
            min_fee_ref_script_cost_per_byte: Some(u5c::RationalNumber {
                numerator: conway.min_fee_ref_script_cost_per_byte as i32,
                denominator: 1,
            }),

            // Conway voting thresholds
            drep_voting_thresholds: Some(u5c::DRepVotingThresholds {
                motion_no_confidence: Some(u5c::RationalNumber {
                    numerator: (conway.d_rep_voting_thresholds.motion_no_confidence * 100.0) as i32,
                    denominator: 100,
                }),
                committee_normal: Some(u5c::RationalNumber {
                    numerator: (conway.d_rep_voting_thresholds.committee_normal * 100.0) as i32,
                    denominator: 100,
                }),
                committee_no_confidence: Some(u5c::RationalNumber {
                    numerator: (conway.d_rep_voting_thresholds.committee_no_confidence * 100.0)
                        as i32,
                    denominator: 100,
                }),
                update_to_constitution: Some(u5c::RationalNumber {
                    numerator: (conway.d_rep_voting_thresholds.update_to_constitution * 100.0)
                        as i32,
                    denominator: 100,
                }),
                hard_fork_initiation: Some(u5c::RationalNumber {
                    numerator: (conway.d_rep_voting_thresholds.hard_fork_initiation * 100.0) as i32,
                    denominator: 100,
                }),
                pp_network_group: Some(u5c::RationalNumber {
                    numerator: (conway.d_rep_voting_thresholds.pp_network_group * 100.0) as i32,
                    denominator: 100,
                }),
                pp_economic_group: Some(u5c::RationalNumber {
                    numerator: (conway.d_rep_voting_thresholds.pp_economic_group * 100.0) as i32,
                    denominator: 100,
                }),
                pp_technical_group: Some(u5c::RationalNumber {
                    numerator: (conway.d_rep_voting_thresholds.pp_technical_group * 100.0) as i32,
                    denominator: 100,
                }),
                pp_gov_group: Some(u5c::RationalNumber {
                    numerator: (conway.d_rep_voting_thresholds.pp_gov_group * 100.0) as i32,
                    denominator: 100,
                }),
                treasury_withdrawal: Some(u5c::RationalNumber {
                    numerator: (conway.d_rep_voting_thresholds.treasury_withdrawal * 100.0) as i32,
                    denominator: 100,
                }),
            }),
            pool_voting_thresholds: Some(u5c::PoolVotingThresholds {
                motion_no_confidence: Some(u5c::RationalNumber {
                    numerator: (conway.pool_voting_thresholds.motion_no_confidence * 100.0) as i32,
                    denominator: 100,
                }),
                committee_normal: Some(u5c::RationalNumber {
                    numerator: (conway.pool_voting_thresholds.committee_normal * 100.0) as i32,
                    denominator: 100,
                }),
                committee_no_confidence: Some(u5c::RationalNumber {
                    numerator: (conway.pool_voting_thresholds.committee_no_confidence * 100.0)
                        as i32,
                    denominator: 100,
                }),
                hard_fork_initiation: Some(u5c::RationalNumber {
                    numerator: (conway.pool_voting_thresholds.hard_fork_initiation * 100.0) as i32,
                    denominator: 100,
                }),
                pp_security_group: Some(u5c::RationalNumber {
                    numerator: (conway.pool_voting_thresholds.pp_security_group * 100.0) as i32,
                    denominator: 100,
                }),
            }),

            // Byron complex structures
            heavy_delegation: byron
                .heavy_delegation
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        u5c::HeavyDelegation {
                            omega: v.omega,
                            issuer_pk: v.issuer_pk.clone(),
                            delegate_pk: v.delegate_pk.clone(),
                            cert: v.cert.clone(),
                        },
                    )
                })
                .collect(),

            // More Byron and other fields
            block_version_data: Some(u5c::BlockVersionData {
                script_version: byron.block_version_data.script_version as u32,
                max_block_size: byron.block_version_data.max_block_size.to_string(),
                max_tx_size: byron.block_version_data.max_tx_size.to_string(),
                max_header_size: byron.block_version_data.max_header_size.to_string(),
                max_proposal_size: byron.block_version_data.max_proposal_size.to_string(),
                mpc_thd: byron.block_version_data.mpc_thd.to_string(),
                heavy_del_thd: byron.block_version_data.heavy_del_thd.to_string(),
                slot_duration: byron.block_version_data.slot_duration.to_string(),
                update_vote_thd: byron.block_version_data.update_vote_thd.to_string(),
                update_proposal_thd: byron.block_version_data.update_proposal_thd.to_string(),
                update_implicit: byron.block_version_data.update_implicit.to_string(),
                softfork_rule: Some(u5c::SoftforkRule {
                    init_thd: byron.block_version_data.softfork_rule.init_thd.to_string(),
                    min_thd: byron.block_version_data.softfork_rule.min_thd.to_string(),
                    thd_decrement: byron
                        .block_version_data
                        .softfork_rule
                        .thd_decrement
                        .to_string(),
                }),
                tx_fee_policy: Some(u5c::TxFeePolicy {
                    summand: byron.block_version_data.tx_fee_policy.summand.to_string(),
                    multiplier: byron
                        .block_version_data
                        .tx_fee_policy
                        .multiplier
                        .to_string(),
                }),
                unlock_stake_epoch: byron.block_version_data.unlock_stake_epoch.to_string(),
            }),
            fts_seed: byron.fts_seed.clone().unwrap_or_default(),
            protocol_consts: Some(u5c::ProtocolConsts {
                k: byron.protocol_consts.k as u32,
                protocol_magic: byron.protocol_consts.protocol_magic as u32,
                vss_min_ttl: byron.protocol_consts.vss_min_ttl.unwrap_or(0),
                vss_max_ttl: byron.protocol_consts.vss_max_ttl.unwrap_or(0),
            }),
            vss_certs: byron
                .vss_certs
                .as_ref()
                .map(|certs| {
                    certs
                        .iter()
                        .map(|(k, v)| {
                            (
                                k.clone(),
                                u5c::VssCert {
                                    vss_key: v.vss_key.clone(),
                                    expiry_epoch: v.expiry_epoch,
                                    signature: v.signature.clone(),
                                    signing_key: v.signing_key.clone(),
                                },
                            )
                        })
                        .collect()
                })
                .unwrap_or_default(),
            protocol_params: current_params.map(|params| self.map_pparams(params)),
            cost_models: Some(u5c::CostModelMap {
                plutus_v1: alonzo
                    .cost_models
                    .get(&alonzo::Language::PlutusV1)
                    .map(|v| u5c::CostModel {
                        values: v.clone().into(),
                    }),
                plutus_v2: alonzo
                    .cost_models
                    .get(&alonzo::Language::PlutusV2)
                    .map(|v| u5c::CostModel {
                        values: v.clone().into(),
                    }),
                plutus_v3: Some(u5c::CostModel {
                    values: conway.plutus_v3_cost_model.clone(),
                }),
            }),
        }
    }

    /// Map era summaries using HFC data from GenesisValues
    /// Includes all eras, even when slot data is missing (will use None for unknown slots)
    pub fn map_era_summaries(
        &self,
        current_params: Option<MultiEraProtocolParameters>,
    ) -> u5c::EraSummaries {
        // Include all eras that exist, not just ones with available slot data
        let all_eras = self.genesis.available_eras();

        let summaries = all_eras
            .iter()
            .enumerate()
            .map(|(i, era)| {
                let start_slot = self.genesis.era_start_slot(*era);
                let end_slot = if i < all_eras.len() - 1 {
                    self.genesis.era_start_slot(all_eras[i + 1])
                } else {
                    None // Current era has no end
                };

                let is_current_era = i == all_eras.len() - 1;

                u5c::EraSummary {
                    name: era.to_string().to_lowercase(),
                    start: start_slot.map(|slot| {
                        let (epoch, _) = self.genesis.absolute_slot_to_relative(slot);
                        u5c::EraBoundary {
                            time: self.genesis.slot_to_wallclock(slot),
                            slot,
                            epoch,
                        }
                    }),
                    end: end_slot.map(|slot| {
                        let (epoch, _) = self.genesis.absolute_slot_to_relative(slot);
                        u5c::EraBoundary {
                            time: self.genesis.slot_to_wallclock(slot),
                            slot,
                            epoch,
                        }
                    }),
                    protocol_params: if is_current_era {
                        current_params
                            .as_ref()
                            .map(|params| self.map_pparams(params.clone()))
                    } else {
                        None
                    },
                }
            })
            .collect();

        u5c::EraSummaries { summaries }
    }
}
