use std::ops::Deref;

use prost_types::FieldMask;

use pallas_traverse as trv;

use crate::LedgerContext;

pub use utxorpc_spec::utxorpc::v1 as spec;

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

crate::shared::impl_cardano_mapper_shared!(utxorpc_spec::utxorpc::v1::cardano);
crate::since_v1beta::impl_cardano_mapper_since_v1beta!();

/// The message this schema maps a protocol parameter update to.
type ParamUpdate = u5c::PParamsUpdate;

/// The value this schema's update message holds for a scalar key, absent when the update leaves the key unset.
fn update_scalar<T>(value: Option<T>) -> Option<T> {
    value
}

/// The u5c purpose of a guarding redeemer.
fn guarding_purpose() -> u5c::RedeemerPurpose {
    u5c::RedeemerPurpose::Guarding
}

/// The u5c member of a guard clause.
fn native_script_guard(
    x: &pallas_primitives::StakeCredential,
) -> Option<u5c::native_script::NativeScript> {
    Some(u5c::native_script::NativeScript::ScriptRequireGuard(
        map_credential(x),
    ))
}

/// Adds the pool BLS key to a pool registration certificate.
fn with_pool_bls_key(
    cert: Option<u5c::certificate::Certificate>,
    key: Option<&pallas_primitives::dijkstra::BlsKey>,
) -> Option<u5c::certificate::Certificate> {
    match cert {
        Some(u5c::certificate::Certificate::PoolRegistration(x)) => Some(
            u5c::certificate::Certificate::PoolRegistration(u5c::PoolRegistrationCert {
                bls_key: key.map(|k| u5c::PoolBlsKey {
                    bls_pubkey: k.bls_pubkey.to_vec().into(),
                    bls_possession_proof: k.bls_possession_proof.to_vec().into(),
                }),
                ..x
            }),
        ),
        other => other,
    }
}

/// The deposit of one coin into one reward account.
fn map_direct_deposit(
    (account, coin): (&pallas_primitives::RewardAccount, &pallas_primitives::Coin),
) -> u5c::DirectDeposit {
    u5c::DirectDeposit {
        reward_account: account.to_vec().into(),
        coin: u64_to_bigint(*coin),
    }
}

/// The interval one reward account balance must fall within.
fn map_account_balance_interval(
    (account, interval): (
        &pallas_primitives::RewardAccount,
        &pallas_primitives::dijkstra::AccountBalanceInterval,
    ),
) -> u5c::AccountBalanceInterval {
    use pallas_primitives::dijkstra::AccountBalanceInterval;
    use u5c::account_balance_interval::Interval;

    let range = |lower: Option<u64>, upper: Option<u64>| {
        Some(Interval::Range(u5c::AccountBalanceRange {
            inclusive_lower_bound: lower.and_then(u64_to_bigint),
            exclusive_upper_bound: upper.and_then(u64_to_bigint),
        }))
    };

    u5c::AccountBalanceInterval {
        reward_account: account.to_vec().into(),
        interval: match *interval {
            AccountBalanceInterval::LowerBound(l) => range(Some(l), None),
            AccountBalanceInterval::Bounded(l, u) => range(Some(l), Some(u)),
            AccountBalanceInterval::UpperBound(u) => range(None, Some(u)),
            AccountBalanceInterval::Exact(c) => u64_to_bigint(c).map(Interval::Exact),
        },
    }
}

/// A key hash as a key credential.
fn map_key_hash(x: &pallas_primitives::AddrKeyhash) -> u5c::StakeCredential {
    map_credential(&pallas_primitives::StakeCredential::AddrKeyhash(*x))
}

/// The credentials at body key 14, key hashes before Dijkstra and guards from it on.
fn map_signers(x: &trv::MultiEraSigners) -> Vec<u5c::StakeCredential> {
    if let Some(x) = x.as_alonzo() {
        return x.iter().map(map_key_hash).collect();
    }

    if let Some(x) = x.as_dijkstra() {
        return map_guards(x);
    }

    vec![]
}

/// The credentials that guard a transaction, a bare key hash as a key credential.
fn map_guards(x: &pallas_primitives::dijkstra::Guards) -> Vec<u5c::StakeCredential> {
    use pallas_primitives::dijkstra::Guards;

    match x {
        Guards::AddrKeyhashes(x) => x.iter().map(map_key_hash).collect(),
        Guards::Credentials(x) => x.iter().map(map_credential).collect(),
    }
}

/// Sets the Leios and Peras fields of a mapped block from the block's header and body.
fn with_leios_fields(mut mapped: u5c::Block, block: &trv::MultiEraBlock) -> u5c::Block {
    let header = block.header();

    if let Some(mapped) = mapped.header.as_mut() {
        mapped.body_contains_leios_certificate =
            header.block_body_contains_leios_cert().unwrap_or_default();
        mapped.endorser_block_announcement =
            header
                .eb_announcement()
                .map(|x| u5c::EndorserBlockAnnouncement {
                    references_hash: x.eb_hash.to_vec().into(),
                    references_size: x.eb_size,
                });
    }

    if let Some(mapped) = mapped.body.as_mut() {
        mapped.leios_certificate = block.leios_certificate().map(|x| u5c::LeiosCertificate {
            signers: x.signers.to_vec().into(),
            signature: x.signature.to_vec().into(),
        });
        mapped.peras_certificate = block
            .peras_certificate()
            .map(|x| x.to_vec().into())
            .unwrap_or_default();
    }

    mapped
}

/// The fields a sub transaction shares, which this schema defines as `Tx.common`.
#[cfg(test)]
fn common(tx: &u5c::Tx) -> &u5c::TxCommon {
    tx.common.as_ref().expect("a mapped transaction has `common`")
}

impl<C: LedgerContext> Mapper<C> {
    pub fn map_tx(&self, tx: &trv::MultiEraTx) -> u5c::Tx {
        let resolved = self.resolve(tx);

        u5c::Tx {
            common: Some(self.map_tx_common(tx, &resolved)),
            collateral: self.map_collateral(tx, &resolved),
            fee: tx.fee().and_then(u64_to_bigint),
            successful: tx.is_valid(),
            sub_transactions: tx
                .sub_transactions()
                .iter()
                .map(|x| self.map_tx_common(x, &self.resolve(x)))
                .collect(),
            starting_account_balance_intervals: tx
                .starting_account_balance_intervals()
                .into_iter()
                .flatten()
                .map(map_account_balance_interval)
                .collect(),
        }
    }

    /// Looks up the outputs a transaction's inputs, collateral and reference inputs name.
    fn resolve(&self, tx: &trv::MultiEraTx) -> Option<crate::UtxoMap> {
        self.ledger.as_ref().and_then(|ctx| {
            let to_resolve = self.find_related_inputs(tx);
            ctx.get_utxos(to_resolve.as_slice())
        })
    }

    /// Maps the fields a transaction shares with a sub transaction.
    fn map_tx_common(
        &self,
        tx: &trv::MultiEraTx,
        resolved: &Option<crate::UtxoMap>,
    ) -> u5c::TxCommon {
        let start = tx.validity_start();
        let ttl = tx.ttl();

        u5c::TxCommon {
            hash: tx.hash().to_vec().into(),
            inputs: tx
                .inputs_sorted_set()
                .iter()
                .enumerate()
                .map(|(order, i)| self.map_tx_input(i, tx, order as u32, resolved))
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
                .map(|x| self.map_tx_reference_input(x, resolved, tx))
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
            validity: (start.is_some() || ttl.is_some()).then_some(u5c::TxValidity { start, ttl }),
            auxiliary: tx.has_aux_data().then(|| u5c::AuxData {
                metadata: tx
                    .metadata()
                    .collect::<Vec<_>>()
                    .into_iter()
                    .map(|(l, d)| self.map_metadata(l, d))
                    .collect(),
                scripts: self.collect_all_aux_scripts(tx),
            }),
            auxiliary_data_hash: tx
                .auxiliary_data_hash()
                .map(|x| x.to_vec().into())
                .unwrap_or_default(),
            script_data_hash: tx
                .script_data_hash()
                .map(|x| x.to_vec().into())
                .unwrap_or_default(),
            network_id: tx.network_id().map(|x| u8::from(x) as u32),
            treasury_value: tx.treasury_value().and_then(u64_to_bigint),
            treasury_donation: tx.donation().and_then(u64_to_bigint),
            guards: map_signers(&tx.required_signers()),
            direct_deposits: tx
                .direct_deposits()
                .into_iter()
                .flatten()
                .map(map_direct_deposit)
                .collect(),
            account_balance_intervals: tx
                .account_balance_intervals()
                .into_iter()
                .flatten()
                .map(map_account_balance_interval)
                .collect(),
            required_top_level_guards: tx
                .required_top_level_guards()
                .into_iter()
                .flatten()
                .map(|x| self.map_required_top_level_guard(x))
                .collect(),
        }
    }

    /// One credential required of the top level transaction, with no datum for a nil one.
    fn map_required_top_level_guard(
        &self,
        (credential, datum): (
            &pallas_primitives::StakeCredential,
            &pallas_primitives::Nullable<pallas_primitives::PlutusData>,
        ),
    ) -> u5c::RequiredTopLevelGuard {
        use pallas_primitives::Nullable;

        u5c::RequiredTopLevelGuard {
            credential: Some(map_credential(credential)),
            datum: match datum {
                Nullable::Some(x) => Some(self.map_plutus_datum(x)),
                Nullable::Null | Nullable::Undefined => None,
            },
        }
    }

    /// Maps the collateral fields of a transaction, absent when its body has none of keys 13, 16 and 17.
    fn map_collateral(
        &self,
        tx: &trv::MultiEraTx,
        resolved: &Option<crate::UtxoMap>,
    ) -> Option<u5c::Collateral> {
        let collateral: Vec<_> = tx
            .collateral()
            .iter()
            .map(|x| self.map_tx_collateral(x, resolved, tx))
            .collect();
        let collateral_return = tx
            .collateral_return()
            .map(|x| self.map_tx_output(&x, Some(tx)));
        let total_collateral = tx.total_collateral().and_then(u64_to_bigint);

        if collateral.is_empty() && collateral_return.is_none() && total_collateral.is_none() {
            return None;
        }

        Some(u5c::Collateral {
            collateral,
            collateral_return,
            total_collateral,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn snapshot() {
        let cases = [
            (
                include_str!("../../../test_data/u5c1.block"),
                include_str!("../../../test_data/u5c_v1.json"),
                "u5c_v1.json",
            ),
            (
                include_str!("../../../test_data/dijkstra6.block"),
                include_str!("../../../test_data/u5c_v1_dijkstra.json"),
                "u5c_v1_dijkstra.json",
            ),
        ];

        let mapper = Mapper::new(NoLedger);

        for (block_str, json_str, file) in cases {
            let cbor = hex::decode(block_str).unwrap();
            let block = trv::MultiEraBlock::decode(&cbor).unwrap();
            let current = serde_json::json!(mapper.map_block(&block));

            // Set REGENERATE_SNAPSHOTS=1 to overwrite the snapshot file in place.
            if std::env::var("REGENERATE_SNAPSHOTS").is_ok() {
                let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../test_data")
                    .join(file);
                std::fs::write(&path, serde_json::to_string_pretty(&current).unwrap()).unwrap();
                eprintln!("regenerated {}", path.display());
                continue;
            }

            let expected: serde_json::Value = serde_json::from_str(json_str).unwrap();

            assert_eq!(expected, current, "{file}")
        }
    }

    /// The optional fields of a mapped transaction, in the order the assertions name them.
    type OptionalFields = (
        Option<u5c::TxValidity>,
        bool,
        bool,
        Vec<u8>,
        Vec<u8>,
        Option<u32>,
        Option<u5c::BigInt>,
        Option<u5c::BigInt>,
    );

    /// Reads the optional fields of a mapped transaction.
    fn optional_fields(tx: &u5c::Tx) -> OptionalFields {
        let common = common(tx);
        (
            common.validity.clone(),
            common.auxiliary.is_some(),
            tx.collateral.is_some(),
            common.auxiliary_data_hash.to_vec(),
            common.script_data_hash.to_vec(),
            common.network_id,
            common.treasury_value.clone(),
            common.treasury_donation.clone(),
        )
    }

    #[test]
    fn a_transaction_without_optional_body_keys_maps_none_of_them() {
        let tx = conway_tx_with_certificate_redeemers();
        let mapped = Mapper::new(NoLedger).map_tx(&tx);

        assert_eq!(
            optional_fields(&mapped),
            (None, false, false, vec![], vec![], None, None, None),
            "a body without keys 3, 7, 8, 11, 13, 15, 21 and 22 and with nil auxiliary data maps none of them"
        );
    }

    #[test]
    fn a_transaction_maps_every_optional_body_key_it_carries() {
        let tx = conway_tx_with_optional_body_keys();
        let mapped = Mapper::new(NoLedger).map_tx(&tx);

        assert_eq!(
            optional_fields(&mapped),
            (
                Some(u5c::TxValidity {
                    start: Some(OPTIONAL_VALIDITY_START),
                    ttl: Some(OPTIONAL_TTL),
                }),
                true,
                true,
                OPTIONAL_AUX_DATA_HASH.to_vec(),
                OPTIONAL_SCRIPT_DATA_HASH.to_vec(),
                Some(OPTIONAL_NETWORK_ID as u32),
                u64_to_bigint(OPTIONAL_TREASURY_VALUE),
                u64_to_bigint(OPTIONAL_DONATION),
            ),
            "each key the body writes maps to its field, the network id of 0 and the empty auxiliary data included"
        );
        assert_eq!(
            mapped.collateral.and_then(|x| x.total_collateral),
            u64_to_bigint(OPTIONAL_TOTAL_COLLATERAL),
        );
    }

    #[test]
    fn an_update_maps_a_key_it_sets_to_zero_and_leaves_an_unset_key_absent() {
        // {3: 0, 4: 1100}, a zero max transaction size and a block header size.
        let update = conway_update(&[0xa2, 0x03, 0x00, 0x04, 0x19, 0x04, 0x4c]);
        let mapped = Mapper::new(NoLedger).map_conway_pparams_update(&update);

        assert_eq!(
            (
                mapped.max_tx_size,
                mapped.max_block_header_size,
                mapped.max_block_body_size,
                mapped.max_value_size,
            ),
            (Some(0), Some(1_100), None, None),
            "a key the update sets is present even at zero, and a key it leaves unset is absent"
        );
    }

    #[test]
    fn a_block_before_dijkstra_maps_no_leios_or_peras_field() {
        let cbor = hex::decode(include_str!("../../../test_data/conway1.block")).unwrap();
        let block = trv::MultiEraBlock::decode(&cbor).unwrap();
        let mapped = Mapper::new(NoLedger).map_block(&block);

        let header = mapped.header.unwrap();
        let body = mapped.body.unwrap();
        assert_eq!(
            (
                header.body_contains_leios_certificate,
                header.endorser_block_announcement,
                body.leios_certificate,
                body.peras_certificate.to_vec(),
            ),
            (false, None, None, vec![])
        );
    }

    #[test]
    fn a_dijkstra_block_without_certificates_maps_no_leios_or_peras_field() {
        let block = dijkstra_block(include_str!("../../../test_data/dijkstra11.block"));
        let mapped = Mapper::new(NoLedger).map_block(&block);

        let header = mapped.header.unwrap();
        let body = mapped.body.unwrap();
        assert_eq!(
            (
                header.body_contains_leios_certificate,
                header.endorser_block_announcement,
                body.leios_certificate,
                body.peras_certificate.to_vec(),
            ),
            (false, None, None, vec![])
        );
    }

    #[test]
    fn a_dijkstra_block_maps_its_leios_certificate_and_announcement() {
        let block = dijkstra_block(include_str!("../../../test_data/dijkstra8.block"));
        let mapped = Mapper::new(NoLedger).map_block(&block);

        let header = mapped.header.unwrap();
        assert!(header.body_contains_leios_certificate);
        assert_eq!(
            header.endorser_block_announcement,
            Some(u5c::EndorserBlockAnnouncement {
                references_hash: hex::decode(
                    "de5f4b812d0e6dc3129510c6663de4ea99bbd6bf019ec2d541853242926fd446"
                )
                .unwrap()
                .into(),
                references_size: 39_495,
            })
        );

        let body = mapped.body.unwrap();
        let certificate = body
            .leios_certificate
            .expect("dijkstra8 carries a Leios certificate");
        assert_eq!(hex::encode(&certificate.signers), "f218a17300000e08200200");
        assert_eq!(certificate.signature.len(), 48);
        assert_eq!(body.peras_certificate.to_vec(), Vec::<u8>::new());
    }

    #[test]
    fn a_dijkstra_block_certifying_without_announcing_maps_no_announcement() {
        let block = dijkstra_block(include_str!("../../../test_data/dijkstra9.block"));
        let mapped = Mapper::new(NoLedger).map_block(&block);

        let header = mapped.header.unwrap();
        assert!(header.body_contains_leios_certificate);
        assert_eq!(header.endorser_block_announcement, None);
        assert!(mapped.body.unwrap().leios_certificate.is_some());
    }

    #[test]
    fn a_dijkstra_block_maps_its_peras_certificate_bytes() {
        use pallas_codec::minicbor;
        use pallas_primitives::dijkstra;

        let cbor = hex::decode(include_str!("../../../test_data/dijkstra3.block")).unwrap();
        let (_, mut block): (u16, dijkstra::Block) = minicbor::decode(&cbor).unwrap();
        block.block_body.peras_certificate =
            pallas_primitives::Nullable::Some(vec![0x22; 16].into());
        let cbor = minicbor::to_vec((8u16, &block)).unwrap();

        let block = trv::MultiEraBlock::decode(&cbor).unwrap();
        let mapped = Mapper::new(NoLedger).map_block(&block);

        assert_eq!(
            mapped.body.unwrap().peras_certificate.to_vec(),
            vec![0x22; 16]
        );
    }

    /// Reads the transaction id and output index of each input given.
    fn input_refs(inputs: &[u5c::TxInput]) -> Vec<(String, u32)> {
        inputs
            .iter()
            .map(|x| (hex::encode(&x.tx_hash), x.output_index))
            .collect()
    }

    #[test]
    fn a_transaction_without_sub_transactions_maps_none() {
        let conway =
            Mapper::new(NoLedger).map_tx(&conway_tx(include_str!("../../../test_data/conway9.tx")));
        let block = dijkstra_block(include_str!("../../../test_data/dijkstra3.block"));
        let dijkstra = Mapper::new(NoLedger).map_tx(&block.txs()[0]);

        assert_eq!(
            (
                conway.sub_transactions.len(),
                dijkstra.sub_transactions.len(),
                common(&dijkstra).direct_deposits.len(),
                common(&dijkstra).account_balance_intervals.len(),
                dijkstra.starting_account_balance_intervals.len(),
            ),
            (0, 0, 0, 0, 0),
            "a Conway transaction and a Dijkstra one whose body has none of keys 23, 25, 26 or 27 map to empty lists"
        );
    }

    #[test]
    fn a_dijkstra_transaction_maps_its_own_body_and_its_sub_transaction() {
        let tx = dijkstra_tx(include_str!("../../../test_data/dijkstra-subtx.tx"));
        let mapped = Mapper::new(NoLedger).map_tx(&tx);
        let outer = common(&mapped);

        assert_eq!(
            hex::encode(&outer.hash),
            "74e2116ca6e0c809f156aa062a9d4ee8b156322618846f1bdea5d9ad7a206a12",
            "the common fields hold the id of the transaction itself"
        );
        assert_eq!(
            (
                input_refs(&outer.inputs),
                outer
                    .outputs
                    .iter()
                    .map(|x| x.coin.clone())
                    .collect::<Vec<_>>(),
                input_refs(&outer.reference_inputs),
            ),
            (
                vec![(
                    "737549420add1f19d144809aa02e2ae68a89b1761368e3294fbb30ee1467e3e4".to_string(),
                    12
                )],
                vec![u64_to_bigint(1_903_631)],
                vec![
                    (
                        "2ed1285cced47acb5f08502843d980d7231665af9d90ef33fe6751e0f9e0b171"
                            .to_string(),
                        0
                    ),
                    (
                        "c095234678d98a74cd1488bda1971fbd2e9c828acf57f7673c679ec601abf7a6"
                            .to_string(),
                        0
                    ),
                ],
            ),
            "the outer body's one input, one output and two reference inputs map to the common fields"
        );

        assert_eq!(
            mapped.sub_transactions.len(),
            1,
            "the body has one sub transaction"
        );
        let sub = &mapped.sub_transactions[0];

        assert_eq!(
            hex::encode(&sub.hash),
            "63ac433eefaed71cf0fa2c7134f82ea38f3c6c338b3d5129c686c47b44162691",
            "the sub transaction's id is the hash of its own body"
        );
        assert_eq!(
            (
                input_refs(&sub.inputs),
                sub.outputs
                    .iter()
                    .map(|x| x.coin.clone())
                    .collect::<Vec<_>>(),
                input_refs(&sub.reference_inputs),
            ),
            (
                vec![(
                    "2ed1285cced47acb5f08502843d980d7231665af9d90ef33fe6751e0f9e0b171".to_string(),
                    0
                )],
                vec![u64_to_bigint(3_000_000)],
                vec![(
                    "c095234678d98a74cd1488bda1971fbd2e9c828acf57f7673c679ec601abf7a6".to_string(),
                    0
                )],
            ),
            "the sub body's one input, one output and one reference input map to the sub transaction"
        );
        assert_eq!(
            (hex::encode(&sub.script_data_hash), sub.guards.clone()),
            (
                "860576d43755cdb611c2f5fdd10d38a91e11362cebe52b198e8beacd900d883e".to_string(),
                vec![u5c::StakeCredential {
                    stake_credential: Some(u5c::stake_credential::StakeCredential::AddrKeyHash(
                        hex::decode("81aa16380175bef2da792c64e694d92e76f0b6c56b333499ea2662df")
                            .unwrap()
                            .into()
                    )),
                }],
            ),
            "the sub body's script data hash and bare key hash guard map to the sub transaction"
        );
    }

    #[test]
    fn reference_inputs_map_the_outputs_the_ledger_resolves_for_them() {
        let tx = dijkstra_tx(include_str!("../../../test_data/dijkstra-subtx.tx"));
        let reference_inputs = tx.reference_inputs();
        let ledger = HeldOutputs(crate::UtxoMap::from([
            (input_ref(&reference_inputs[0]), conway_output(2_000_000)),
            (input_ref(&reference_inputs[1]), conway_output(3_000_000)),
        ]));
        let mapped = Mapper::new(ledger).map_tx(&tx);

        assert_eq!(
            (
                resolved_coins(&common(&mapped).reference_inputs),
                resolved_coins(&mapped.sub_transactions[0].reference_inputs),
            ),
            (
                vec![u64_to_bigint(2_000_000), u64_to_bigint(3_000_000)],
                vec![u64_to_bigint(3_000_000)],
            ),
            "each reference input maps the output the ledger holds for it, the sub transaction's included"
        );
    }

    #[test]
    fn a_transaction_maps_the_validity_flag_its_block_sets() {
        let valid = include_str!("../../../test_data/dijkstra-subtx.tx").trim();
        let invalid = valid
            .strip_suffix("f5")
            .map(|body| format!("{body}f4"))
            .expect("the fixture's last item is the flag, written as true");

        assert_eq!(
            (
                Mapper::new(NoLedger).map_tx(&dijkstra_tx(valid)).successful,
                Mapper::new(NoLedger)
                    .map_tx(&dijkstra_tx(&invalid))
                    .successful,
            ),
            (true, false),
            "the flag the block producer writes maps to successful, false included"
        );
    }

    /// Reads the coin of the output each input given resolves to.
    fn resolved_coins(inputs: &[u5c::TxInput]) -> Vec<Option<u5c::BigInt>> {
        inputs
            .iter()
            .map(|x| x.as_output.as_ref().and_then(|x| x.coin.clone()))
            .collect()
    }

    /// The output reference an input names.
    fn input_ref(x: &trv::MultiEraInput) -> crate::TxoRef {
        (*x.hash(), x.index() as u32)
    }

    #[test]
    fn a_sub_transaction_input_the_ledger_does_not_hold_maps_no_output() {
        let tx = dijkstra_tx_with_guards();
        let ledger = HeldOutputs(crate::UtxoMap::from([(
            input_ref(&tx.inputs()[0]),
            conway_output(1_000_000),
        )]));
        let mapped = Mapper::new(ledger).map_tx(&tx);

        assert_eq!(
            (
                resolved_coins(&common(&mapped).inputs),
                resolved_coins(&mapped.sub_transactions[0].inputs),
            ),
            (vec![u64_to_bigint(1_000_000)], vec![None]),
            "the ledger holds the output of the outer input and not that of the sub transaction's input"
        );
    }

    #[test]
    fn a_sub_transaction_input_maps_the_output_the_ledger_resolves_for_it() {
        let tx = dijkstra_tx_with_guards();
        let ledger = HeldOutputs(crate::UtxoMap::from([
            (input_ref(&tx.inputs()[0]), conway_output(1_000_000)),
            (
                input_ref(&tx.sub_transactions()[0].inputs()[0]),
                conway_output(2_000_000),
            ),
        ]));
        let mapped = Mapper::new(ledger).map_tx(&tx);

        assert_eq!(
            (
                resolved_coins(&common(&mapped).inputs),
                resolved_coins(&mapped.sub_transactions[0].inputs),
            ),
            (
                vec![u64_to_bigint(1_000_000)],
                vec![u64_to_bigint(2_000_000)]
            ),
            "each input maps the output the ledger holds for it, the sub transaction's input included"
        );
    }

    #[test]
    fn a_dijkstra_transaction_maps_its_collateral_input_and_fee() {
        let tx = dijkstra_tx(include_str!("../../../test_data/dijkstra-subtx.tx"));
        let mapped = Mapper::new(NoLedger).map_tx(&tx);

        assert_eq!(
            (
                input_refs(&mapped.collateral.as_ref().unwrap().collateral),
                mapped.fee,
            ),
            (
                vec![(
                    "7727ec1dfb0f84f2ffeb51935f35850cf84cfabfca1957e8c74cf8fde871dd34".to_string(),
                    12
                )],
                u64_to_bigint(700_000)
            ),
            "the outer body's collateral input and fee map to the transaction"
        );
    }

    #[test]
    fn deposits_and_intervals_map_with_their_accounts() {
        use u5c::account_balance_interval::Interval;

        let mapped = Mapper::new(NoLedger).map_tx(&dijkstra_tx_with_account_fields());

        let range = |lower: Option<u64>, upper: Option<u64>| {
            Some(Interval::Range(u5c::AccountBalanceRange {
                inclusive_lower_bound: lower.and_then(u64_to_bigint),
                exclusive_upper_bound: upper.and_then(u64_to_bigint),
            }))
        };
        let interval =
            |account: [u8; 29], interval: Option<Interval>| u5c::AccountBalanceInterval {
                reward_account: account.to_vec().into(),
                interval,
            };

        assert_eq!(
            common(&mapped).direct_deposits,
            vec![u5c::DirectDeposit {
                reward_account: DEPOSIT_ACCOUNT.to_vec().into(),
                coin: u64_to_bigint(DEPOSIT_COIN),
            }],
            "the body's one deposit maps with its account and its coin"
        );

        assert_eq!(
            common(&mapped).account_balance_intervals,
            vec![
                interval(LOWER_BOUND_ACCOUNT, range(Some(5), None)),
                interval(UPPER_BOUND_ACCOUNT, range(None, Some(6))),
                interval(BOUNDED_ACCOUNT, range(Some(7), Some(8))),
                interval(COIN_ACCOUNT, u64_to_bigint(9).map(Interval::Exact)),
            ],
            "each interval keeps the bounds it was written with, and a bare coin maps to an exact balance"
        );

        assert_eq!(
            mapped.starting_account_balance_intervals,
            vec![interval(STARTING_ACCOUNT, range(Some(10), None))],
            "the starting intervals map to their own field"
        );

        assert_eq!(
            mapped.sub_transactions.len(),
            1,
            "the body has one sub transaction"
        );
        let sub = &mapped.sub_transactions[0];
        assert_eq!(
            (
                sub.direct_deposits.clone(),
                sub.account_balance_intervals.clone(),
            ),
            (
                vec![u5c::DirectDeposit {
                    reward_account: SUB_DEPOSIT_ACCOUNT.to_vec().into(),
                    coin: u64_to_bigint(12),
                }],
                vec![interval(SUB_INTERVAL_ACCOUNT, range(None, Some(13)))],
            ),
            "the sub body's deposit and interval map to the sub transaction and not to the one holding it"
        );
    }

    #[test]
    fn a_body_without_guards_maps_none() {
        let mapped = Mapper::new(NoLedger).map_tx(&dijkstra_tx_with_account_fields());

        assert_eq!(
            (
                common(&mapped).guards.len(),
                mapped.sub_transactions[0].guards.len()
            ),
            (0, 0),
            "a Dijkstra body and a sub body without key 14 map to no guards"
        );
    }

    #[test]
    fn guards_map_with_their_key_or_script_tag() {
        use u5c::stake_credential::StakeCredential;

        let credential = |inner: StakeCredential| u5c::StakeCredential {
            stake_credential: Some(inner),
        };
        let mapped = Mapper::new(NoLedger).map_tx(&dijkstra_tx_with_guards());

        assert_eq!(
            common(&mapped).guards,
            vec![
                credential(StakeCredential::AddrKeyHash(GUARD_KEY_HASH.to_vec().into())),
                credential(StakeCredential::ScriptHash(
                    GUARD_SCRIPT_HASH.to_vec().into()
                )),
            ],
            "each credential of the body's guards maps with its key or script tag, in body order"
        );

        assert_eq!(
            mapped.sub_transactions[0].guards,
            vec![credential(StakeCredential::AddrKeyHash(
                SUB_GUARD_KEY_HASH.to_vec().into()
            ))],
            "a sub body's bare key hash maps to a key credential of the sub transaction"
        );
    }

    #[test]
    fn a_conway_body_without_required_signers_maps_no_guards() {
        let mapped = Mapper::new(NoLedger).map_tx(&conway_tx_with_certificate_redeemers());

        assert!(
            common(&mapped).guards.is_empty(),
            "a Conway body without key 14 maps to no guards"
        );
    }

    #[test]
    fn conway_required_signers_map_to_key_credentials() {
        use u5c::stake_credential::StakeCredential;

        let key = |hash: [u8; 28]| u5c::StakeCredential {
            stake_credential: Some(StakeCredential::AddrKeyHash(hash.to_vec().into())),
        };
        let mapped = Mapper::new(NoLedger).map_tx(&conway_tx_with_required_signers());

        assert_eq!(
            common(&mapped).guards,
            vec![key(FIRST_REQUIRED_SIGNER), key(SECOND_REQUIRED_SIGNER)],
            "each required signer maps to a key credential of the guards, in body order"
        );
    }

    #[test]
    fn a_body_without_required_top_level_guards_maps_none() {
        let mapped = Mapper::new(NoLedger).map_tx(&dijkstra_tx_with_guards());

        assert_eq!(
            (
                common(&mapped).required_top_level_guards.len(),
                mapped.sub_transactions[0].required_top_level_guards.len()
            ),
            (0, 0),
            "a body and a sub body without key 24 map to no required top level guards"
        );
    }

    #[test]
    fn required_top_level_guards_map_with_their_credential_and_datum() {
        use u5c::stake_credential::StakeCredential;

        let guard = |inner: StakeCredential, datum: Option<u5c::plutus_data::PlutusData>| {
            u5c::RequiredTopLevelGuard {
                credential: Some(u5c::StakeCredential {
                    stake_credential: Some(inner),
                }),
                datum: datum.map(|x| u5c::PlutusData {
                    plutus_data: Some(x),
                }),
            }
        };
        let mapped = Mapper::new(NoLedger).map_tx(&dijkstra_tx_with_required_top_level_guards());

        assert_eq!(
            common(&mapped).required_top_level_guards,
            vec![
                guard(
                    StakeCredential::ScriptHash(TOP_LEVEL_GUARD_SCRIPT_HASH.to_vec().into()),
                    None,
                ),
                guard(
                    StakeCredential::AddrKeyHash(TOP_LEVEL_GUARD_KEY_HASH.to_vec().into()),
                    Some(u5c::plutus_data::PlutusData::BigInt(u5c::BigInt {
                        big_int: Some(u5c::big_int::BigInt::Int(TOP_LEVEL_GUARD_DATUM)),
                    })),
                ),
            ],
            "each entry of the body's key 24 maps with its credential, a nil datum as none, in the ledger's credential order with script credentials first"
        );

        assert_eq!(
            mapped.sub_transactions[0].required_top_level_guards,
            vec![guard(
                StakeCredential::AddrKeyHash(SUB_TOP_LEVEL_GUARD_KEY_HASH.to_vec().into()),
                Some(u5c::plutus_data::PlutusData::Constr(u5c::Constr {
                    tag: 121,
                    any_constructor: 0,
                    fields: vec![],
                })),
            )],
            "a sub body's key 24 maps to the sub transaction"
        );
    }

    #[test]
    fn a_guarding_redeemer_maps_to_the_guarding_purpose() {
        assert_eq!(
            Mapper::new(NoLedger).map_multi_era_purpose(&trv::MultiEraRedeemerTag::Guarding),
            u5c::RedeemerPurpose::Guarding,
            "a guarding redeemer maps to the purpose of the same name"
        );
    }

    #[test]
    fn a_guard_clause_maps_to_the_credential_it_names() {
        use pallas_primitives::{StakeCredential, dijkstra};

        let script = dijkstra::NativeScript::ScriptAll(vec![
            dijkstra::NativeScript::ScriptRequireGuard(StakeCredential::ScriptHash(
                [0x7a; 28].into(),
            )),
            dijkstra::NativeScript::ScriptPubkey([0x44; 28].into()),
        ]);

        let mapped = Mapper::<NoLedger>::map_any_native_script(
            &trv::MultiEraNativeScript::from_decoded_dijkstra(&script),
        );

        let Some(u5c::native_script::NativeScript::ScriptAll(list)) = mapped.native_script else {
            panic!(
                "the root clause is script_all, got {:?}",
                mapped.native_script
            );
        };
        assert_eq!(
            list.items[0].native_script,
            Some(u5c::native_script::NativeScript::ScriptRequireGuard(
                u5c::StakeCredential {
                    stake_credential: Some(u5c::stake_credential::StakeCredential::ScriptHash(
                        [0x7a; 28].to_vec().into()
                    )),
                }
            )),
            "the guard maps to the credential it requires, of the kind it names"
        );
    }

    #[test]
    fn a_dijkstra_pool_registration_maps_the_bls_key_it_fills() {
        let block = dijkstra_block(include_str!("../../../test_data/dijkstra6.block"));
        let txs = block.txs();
        let expected: Vec<u5c::PoolBlsKey> = txs
            .iter()
            .flat_map(|tx| tx.certs())
            .filter_map(|cert| {
                cert.bls_key().map(|k| u5c::PoolBlsKey {
                    bls_pubkey: k.bls_pubkey.to_vec().into(),
                    bls_possession_proof: k.bls_possession_proof.to_vec().into(),
                })
            })
            .collect();
        assert_eq!(
            expected.len(),
            2,
            "this block's two pool registrations each fill the BLS key slot"
        );

        let mapped = Mapper::new(NoLedger).map_block(&block);
        let found: Vec<u5c::PoolBlsKey> = mapped
            .body
            .iter()
            .flat_map(|body| body.tx.iter())
            .flat_map(|tx| common(tx).certificates.iter())
            .filter_map(|cert| match &cert.certificate {
                Some(u5c::certificate::Certificate::PoolRegistration(x)) => x.bls_key.clone(),
                _ => None,
            })
            .collect();

        assert_eq!(
            found, expected,
            "each pool registration's key and proof map to its certificate"
        );
    }
}
