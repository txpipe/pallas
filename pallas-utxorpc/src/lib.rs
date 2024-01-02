use std::ops::Deref;

use pallas_codec::utils::KeyValuePairs;
use pallas_primitives::{alonzo, babbage};
use pallas_traverse as trv;

use trv::OriginalHash;

use utxorpc::proto::cardano::v1 as u5c;

pub fn map_purpose(x: &alonzo::RedeemerTag) -> u5c::RedeemerPurpose {
    match x {
        babbage::RedeemerTag::Spend => u5c::RedeemerPurpose::Spend,
        babbage::RedeemerTag::Mint => u5c::RedeemerPurpose::Mint,
        babbage::RedeemerTag::Cert => u5c::RedeemerPurpose::Cert,
        babbage::RedeemerTag::Reward => u5c::RedeemerPurpose::Reward,
    }
}

pub fn map_redeemer(x: &alonzo::Redeemer) -> u5c::Redeemer {
    u5c::Redeemer {
        purpose: map_purpose(&x.tag).into(),
        datum: map_plutus_datum(&x.data).into(),
    }
}

pub fn map_tx_input(i: &trv::MultiEraInput, tx: &trv::MultiEraTx) -> u5c::TxInput {
    let redeemer = tx
        .redeemers()
        .iter()
        .find(|r| (r.index as u64) == i.index());

    u5c::TxInput {
        tx_hash: i.hash().to_vec().into(),
        output_index: i.index() as u32,
        redeemer: redeemer.map(map_redeemer),
        // TODO: map output data from some context
        as_output: None,
    }
}

pub fn map_tx_output(x: &trv::MultiEraOutput) -> u5c::TxOutput {
    u5c::TxOutput {
        address: x.address().map(|a| a.to_vec()).unwrap_or_default().into(),
        coin: x.lovelace_amount(),
        // TODO: this is wrong, we're crating a new item for each asset even if they share
        // the same policy id. We need to adjust Pallas' interface to make this mapping more
        // ergonomic.
        assets: x.non_ada_assets().iter().map(map_policy_assets).collect(),
        datum: match x.datum() {
            Some(babbage::PseudoDatumOption::Data(x)) => map_plutus_datum(&x.0).into(),
            _ => None,
        },
        datum_hash: match x.datum() {
            Some(babbage::PseudoDatumOption::Data(x)) => x.original_hash().to_vec().into(),
            Some(babbage::PseudoDatumOption::Hash(x)) => x.to_vec().into(),
            _ => vec![].into(),
        },
        script: match x.script_ref() {
            Some(babbage::PseudoScript::NativeScript(x)) => u5c::Script {
                script: u5c::script::Script::Native(map_native_script(&x)).into(),
            }
            .into(),
            Some(babbage::PseudoScript::PlutusV1Script(x)) => u5c::Script {
                script: u5c::script::Script::PlutusV1(x.0.to_vec().into()).into(),
            }
            .into(),
            Some(babbage::PseudoScript::PlutusV2Script(x)) => u5c::Script {
                script: u5c::script::Script::PlutusV2(x.0.to_vec().into()).into(),
            }
            .into(),
            _ => None,
        },
    }
}

pub fn map_stake_credential(x: &babbage::StakeCredential) -> u5c::StakeCredential {
    let inner = match x {
        babbage::StakeCredential::AddrKeyhash(x) => {
            u5c::stake_credential::StakeCredential::AddrKeyHash(x.to_vec().into())
        }
        babbage::StakeCredential::Scripthash(x) => {
            u5c::stake_credential::StakeCredential::ScriptHash(x.to_vec().into())
        }
    };

    u5c::StakeCredential {
        stake_credential: inner.into(),
    }
}

pub fn map_relay(x: &alonzo::Relay) -> u5c::Relay {
    match x {
        babbage::Relay::SingleHostAddr(port, v4, v6) => u5c::Relay {
            ip_v4: v4.as_ref().map(|x| x.to_vec().into()).unwrap_or_default(),
            ip_v6: v6.as_ref().map(|x| x.to_vec().into()).unwrap_or_default(),
            dns_name: String::default(),
            port: port.unwrap_or_default(),
        },
        babbage::Relay::SingleHostName(port, name) => u5c::Relay {
            ip_v4: Default::default(),
            ip_v6: Default::default(),
            dns_name: name.clone(),
            port: port.unwrap_or_default(),
        },
        babbage::Relay::MultiHostName(name) => u5c::Relay {
            ip_v4: Default::default(),
            ip_v6: Default::default(),
            dns_name: name.clone(),
            port: Default::default(),
        },
    }
}

pub fn map_cert(x: &trv::MultiEraCert) -> u5c::Certificate {
    let inner = match x.as_alonzo().unwrap() {
        babbage::Certificate::StakeRegistration(a) => {
            u5c::certificate::Certificate::StakeRegistration(map_stake_credential(a))
        }
        babbage::Certificate::StakeDeregistration(a) => {
            u5c::certificate::Certificate::StakeDeregistration(map_stake_credential(a))
        }
        babbage::Certificate::StakeDelegation(a, b) => {
            u5c::certificate::Certificate::StakeDelegation(u5c::StakeDelegationCert {
                stake_credential: map_stake_credential(a).into(),
                pool_keyhash: b.to_vec().into(),
            })
        }
        babbage::Certificate::PoolRegistration {
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
            pledge: *pledge,
            cost: *cost,
            margin: u5c::RationalNumber {
                numerator: margin.numerator as i32,
                denominator: margin.denominator as u32,
            }
            .into(),
            reward_account: reward_account.to_vec().into(),
            pool_owners: pool_owners.iter().map(|x| x.to_vec().into()).collect(),
            relays: relays.iter().map(map_relay).collect(),
            pool_metadata: pool_metadata.as_ref().map(|x| u5c::PoolMetadata {
                url: x.url.clone(),
                hash: x.hash.to_vec().into(),
            }),
        }),
        babbage::Certificate::PoolRetirement(a, b) => {
            u5c::certificate::Certificate::PoolRetirement(u5c::PoolRetirementCert {
                pool_keyhash: a.to_vec().into(),
                epoch: *b,
            })
        }
        babbage::Certificate::GenesisKeyDelegation(a, b, c) => {
            u5c::certificate::Certificate::GenesisKeyDelegation(u5c::GenesisKeyDelegationCert {
                genesis_hash: a.to_vec().into(),
                genesis_delegate_hash: b.to_vec().into(),
                vrf_keyhash: c.to_vec().into(),
            })
        }
        babbage::Certificate::MoveInstantaneousRewardsCert(a) => {
            u5c::certificate::Certificate::MirCert(u5c::MirCert {
                from: match &a.source {
                    babbage::InstantaneousRewardSource::Reserves => u5c::MirSource::Reserves.into(),
                    babbage::InstantaneousRewardSource::Treasury => u5c::MirSource::Treasury.into(),
                },
                to: match &a.target {
                    babbage::InstantaneousRewardTarget::StakeCredentials(x) => x
                        .iter()
                        .map(|(k, v)| u5c::MirTarget {
                            stake_credential: map_stake_credential(k).into(),
                            delta_coin: *v,
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
    }
}

pub fn map_withdrawals(x: &(&[u8], u64)) -> u5c::Withdrawal {
    u5c::Withdrawal {
        reward_account: Vec::from(x.0).into(),
        coin: x.1,
    }
}

pub fn map_asset(x: &trv::MultiEraAsset) -> u5c::Asset {
    u5c::Asset {
        name: x.name().to_vec().into(),
        output_coin: x.output_coin().unwrap_or_default(),
        mint_coin: x.mint_coin().unwrap_or_default(),
    }
}

pub fn map_policy_assets(x: &trv::MultiEraPolicyAssets) -> u5c::Multiasset {
    u5c::Multiasset {
        policy_id: x.policy().to_vec().into(),
        assets: x.assets().iter().map(map_asset).collect(),
    }
}

pub fn map_vkey_witness(x: &alonzo::VKeyWitness) -> u5c::VKeyWitness {
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
                items: x.iter().map(map_native_script).collect(),
            })
        }
        babbage::NativeScript::ScriptAny(x) => {
            u5c::native_script::NativeScript::ScriptAll(u5c::NativeScriptList {
                items: x.iter().map(map_native_script).collect(),
            })
        }
        babbage::NativeScript::ScriptNOfK(n, k) => {
            u5c::native_script::NativeScript::ScriptNOfK(u5c::ScriptNOfK {
                k: *n,
                scripts: k.iter().map(map_native_script).collect(),
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

fn collect_all_scripts(tx: &trv::MultiEraTx) -> Vec<u5c::Script> {
    let ns = tx
        .native_scripts()
        .iter()
        .map(|x| map_native_script(x.deref()))
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

pub fn map_plutus_constr(x: &alonzo::Constr<alonzo::PlutusData>) -> u5c::Constr {
    u5c::Constr {
        tag: x.tag as u32,
        any_constructor: x.any_constructor.unwrap_or_default(),
        fields: x.fields.iter().map(map_plutus_datum).collect(),
    }
}

pub fn map_plutus_map(
    x: &KeyValuePairs<alonzo::PlutusData, alonzo::PlutusData>,
) -> u5c::PlutusDataMap {
    u5c::PlutusDataMap {
        pairs: x
            .iter()
            .map(|(k, v)| u5c::PlutusDataPair {
                key: map_plutus_datum(k).into(),
                value: map_plutus_datum(v).into(),
            })
            .collect(),
    }
}

pub fn map_plutus_array(x: &[alonzo::PlutusData]) -> u5c::PlutusDataArray {
    u5c::PlutusDataArray {
        items: x.iter().map(map_plutus_datum).collect(),
    }
}

pub fn map_plutus_bigint(x: &alonzo::BigInt) -> u5c::BigInt {
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

pub fn map_plutus_datum(x: &alonzo::PlutusData) -> u5c::PlutusData {
    let inner = match x {
        babbage::PlutusData::Constr(x) => {
            u5c::plutus_data::PlutusData::Constr(map_plutus_constr(x))
        }
        babbage::PlutusData::Map(x) => u5c::plutus_data::PlutusData::Map(map_plutus_map(x)),
        babbage::PlutusData::Array(x) => u5c::plutus_data::PlutusData::Array(map_plutus_array(x)),
        babbage::PlutusData::BigInt(x) => {
            u5c::plutus_data::PlutusData::BigInt(map_plutus_bigint(x))
        }
        babbage::PlutusData::BoundedBytes(x) => {
            u5c::plutus_data::PlutusData::BoundedBytes(x.to_vec().into())
        }
    };

    u5c::PlutusData {
        plutus_data: inner.into(),
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
            items: x.iter().map(map_metadatum).collect(),
        }),
        babbage::Metadatum::Map(x) => u5c::metadatum::Metadatum::Map(u5c::MetadatumMap {
            pairs: x
                .iter()
                .map(|(k, v)| u5c::MetadatumPair {
                    key: map_metadatum(k).into(),
                    value: map_metadatum(v).into(),
                })
                .collect(),
        }),
    };

    u5c::Metadatum {
        metadatum: inner.into(),
    }
}

pub fn map_metadata(label: u64, datum: &alonzo::Metadatum) -> u5c::Metadata {
    u5c::Metadata {
        label,
        value: map_metadatum(datum).into(),
    }
}

fn collect_all_aux_scripts(tx: &trv::MultiEraTx) -> Vec<u5c::Script> {
    let ns = tx
        .aux_native_scripts()
        .iter()
        .map(map_native_script)
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

pub fn map_tx(tx: &trv::MultiEraTx) -> u5c::Tx {
    u5c::Tx {
        inputs: tx.inputs().iter().map(|i| map_tx_input(i, tx)).collect(),
        outputs: tx.outputs().iter().map(map_tx_output).collect(),
        certificates: tx.certs().iter().map(map_cert).collect(),
        withdrawals: tx
            .withdrawals()
            .collect::<Vec<_>>()
            .iter()
            .map(map_withdrawals)
            .collect(),
        mint: tx.mints().iter().map(map_policy_assets).collect(),
        reference_inputs: tx
            .reference_inputs()
            .iter()
            .map(|x| map_tx_input(x, tx))
            .collect(),
        witnesses: u5c::WitnessSet {
            vkeywitness: tx.vkey_witnesses().iter().map(map_vkey_witness).collect(),
            script: collect_all_scripts(tx),
            plutus_datums: tx
                .plutus_data()
                .iter()
                .map(|x| map_plutus_datum(x.deref()))
                .collect(),
        }
        .into(),
        collateral: u5c::Collateral {
            collateral: tx
                .collateral()
                .iter()
                .map(|x| map_tx_input(x, tx))
                .collect(),
            collateral_return: tx.collateral_return().map(|x| map_tx_output(&x)),
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
                .map(|(l, d)| map_metadata(l, d))
                .collect(),
            scripts: collect_all_aux_scripts(tx),
        }
        .into(),
    }
}

pub fn map_block(block: &trv::MultiEraBlock) -> u5c::Block {
    u5c::Block {
        header: u5c::BlockHeader {
            slot: block.slot(),
            hash: block.hash().to_vec().into(),
        }
        .into(),
        body: u5c::BlockBody {
            tx: block.txs().iter().map(map_tx).collect(),
        }
        .into(),
    }
}

pub fn map_block_cbor(raw: &[u8]) -> u5c::Block {
    let block = trv::MultiEraBlock::decode(raw).unwrap();
    map_block(&block)
}
