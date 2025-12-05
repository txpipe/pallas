use pallas_primitives::{alonzo, babbage, conway};
use pallas_traverse as trv;
use utxorpc_spec::utxorpc::v1alpha::cardano as u5c;

use crate::{i64_to_bigint, u64_to_bigint, LedgerContext, Mapper};

impl<C: LedgerContext> Mapper<C> {
    pub fn map_alonzo_compatible_cert(
        &self,
        x: &pallas_primitives::alonzo::Certificate,
        tx: &trv::MultiEraTx,
        order: u32,
    ) -> u5c::Certificate {
        let inner = match x {
            alonzo::Certificate::StakeRegistration(a) => {
                u5c::certificate::Certificate::StakeRegistration(self.map_stake_credential(a))
            }
            alonzo::Certificate::StakeDeregistration(a) => {
                u5c::certificate::Certificate::StakeDeregistration(self.map_stake_credential(a))
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
            } => u5c::certificate::Certificate::PoolRegistration(u5c::PoolRegistrationCert {
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
            }),
            alonzo::Certificate::PoolRetirement(a, b) => {
                u5c::certificate::Certificate::PoolRetirement(u5c::PoolRetirementCert {
                    pool_keyhash: a.to_vec().into(),
                    epoch: *b,
                })
            }
            alonzo::Certificate::GenesisKeyDelegation(a, b, c) => {
                u5c::certificate::Certificate::GenesisKeyDelegation(u5c::GenesisKeyDelegationCert {
                    genesis_hash: a.to_vec().into(),
                    genesis_delegate_hash: b.to_vec().into(),
                    vrf_keyhash: c.to_vec().into(),
                })
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

    pub fn map_drep(&self, x: &conway::DRep) -> u5c::DRep {
        u5c::DRep {
            drep: match x {
                conway::DRep::Key(x) => u5c::d_rep::Drep::AddrKeyHash(x.to_vec().into()).into(),
                conway::DRep::Script(x) => u5c::d_rep::Drep::ScriptHash(x.to_vec().into()).into(),
                conway::DRep::Abstain => u5c::d_rep::Drep::Abstain(true).into(),
                conway::DRep::NoConfidence => u5c::d_rep::Drep::NoConfidence(true).into(),
            },
        }
    }

    pub fn map_conway_cert(
        &self,
        x: &conway::Certificate,
        tx: &trv::MultiEraTx,
        order: u32,
    ) -> u5c::Certificate {
        let inner = match x {
            conway::Certificate::StakeRegistration(a) => {
                u5c::certificate::Certificate::StakeRegistration(self.map_stake_credential(a))
            }
            conway::Certificate::StakeDeregistration(a) => {
                u5c::certificate::Certificate::StakeDeregistration(self.map_stake_credential(a))
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
            } => u5c::certificate::Certificate::PoolRegistration(u5c::PoolRegistrationCert {
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
            }),
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
                u5c::certificate::Certificate::StakeVoteRegDelegCert(u5c::StakeVoteRegDelegCert {
                    stake_credential: self.map_stake_credential(stake_cred).into(),
                    pool_keyhash: pool_id.to_vec().into(),
                    drep: self.map_drep(drep).into(),
                    coin: u64_to_bigint(*coin),
                })
            }
            conway::Certificate::AuthCommitteeHot(cold_cred, hot_cred) => {
                u5c::certificate::Certificate::AuthCommitteeHotCert(u5c::AuthCommitteeHotCert {
                    committee_cold_credential: self.map_stake_credential(cold_cred).into(),
                    committee_hot_credential: self.map_stake_credential(hot_cred).into(),
                })
            }
            conway::Certificate::ResignCommitteeCold(cold_cred, anchor) => {
                u5c::certificate::Certificate::ResignCommitteeColdCert(
                    u5c::ResignCommitteeColdCert {
                        committee_cold_credential: self.map_stake_credential(cold_cred).into(),
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
        x: &trv::MultiEraCert,
        tx: &trv::MultiEraTx,
        order: u32,
    ) -> Option<u5c::Certificate> {
        match x {
            pallas_traverse::MultiEraCert::AlonzoCompatible(x) => {
                self.map_alonzo_compatible_cert(x, tx, order).into()
            }
            pallas_traverse::MultiEraCert::Conway(x) => self.map_conway_cert(x, tx, order).into(),
            _ => None,
        }
    }
}
