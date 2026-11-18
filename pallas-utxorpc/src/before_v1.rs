/// Emits the parts of `Mapper<C>` that only the schemas before v1 keep.
macro_rules! impl_cardano_mapper_before_v1 {
    () => {
        /// The message this schema maps a protocol parameter update to.
        type ParamUpdate = u5c::PParams;

        /// The value this schema's update message holds for a scalar key, its proto3 zero when the update leaves the key unset.
        fn update_scalar<T: Default>(value: Option<T>) -> T {
            value.unwrap_or_default()
        }

        /// The schema's ratio for a ledger ratio, each field cut to its low 32 bits.
        fn rational_number_to_u5c(value: pallas_primitives::RationalNumber) -> u5c::RationalNumber {
            u5c::RationalNumber {
                numerator: value.numerator as i32,
                denominator: value.denominator as u32,
            }
        }

        /// The schema's integer for a Plutus integer, cut to its low 64 bits.
        fn map_plutus_int(n: i128) -> u5c::big_int::BigInt {
            u5c::big_int::BigInt::Int(n as i64)
        }

        /// Returns the mapped block unchanged, as this schema has no Leios field.
        fn with_leios_fields(
            mapped: u5c::Block,
            _block: &pallas_traverse::MultiEraBlock,
        ) -> u5c::Block {
            mapped
        }

        /// Returns the update unchanged, as this schema has no field for a key after 33.
        fn with_dijkstra_update(
            update: ParamUpdate,
            _: &pallas_traverse::MultiEraParamUpdate,
        ) -> ParamUpdate {
            update
        }

        /// An empty parameter set, as this schema has no field after the DRep inactivity period.
        #[cfg(feature = "unstable")]
        fn dijkstra_pparams(_: &pallas_validate::utils::DijkstraProtParams) -> u5c::PParams {
            u5c::PParams::default()
        }

        /// Returns the unspecified purpose for a guarding redeemer, for which this schema has no member.
        #[cfg(feature = "unstable")]
        fn guarding_purpose() -> u5c::RedeemerPurpose {
            u5c::RedeemerPurpose::Unspecified
        }

        /// Returns no clause for a guard clause, for which this schema has no member.
        #[cfg(feature = "unstable")]
        fn native_script_guard(
            _: &pallas_primitives::StakeCredential,
        ) -> Option<u5c::native_script::NativeScript> {
            None
        }

        /// Returns the certificate unchanged, as this schema has no field for a pool BLS key.
        #[cfg(feature = "unstable")]
        fn with_pool_bls_key(
            cert: Option<u5c::certificate::Certificate>,
            _: Option<&pallas_primitives::dijkstra::BlsKey>,
        ) -> Option<u5c::certificate::Certificate> {
            cert
        }

        /// The fields a sub transaction shares, which this schema defines on the transaction itself.
        #[cfg(all(test, feature = "unstable"))]
        fn common(tx: &u5c::Tx) -> &u5c::Tx {
            tx
        }

        impl<C: $crate::LedgerContext> Mapper<C> {
            #[deprecated(
                since = "1.5.0",
                note = "use map_multi_era_purpose. This method cannot represent the Dijkstra Guarding tag"
            )]
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

            #[deprecated(
                since = "1.5.0",
                note = "use Mapper::map_cert or Mapper::map_cert_kind"
            )]
            pub fn map_alonzo_compatible_cert(
                &self,
                x: &pallas_primitives::alonzo::Certificate,
                tx: &pallas_traverse::MultiEraTx,
                order: u32,
            ) -> u5c::Certificate {
                let cert = pallas_traverse::MultiEraCert::AlonzoCompatible(Box::new(
                    std::borrow::Cow::Borrowed(x),
                ));

                self.map_cert(&cert, tx, order)
                    .expect("an Alonzo compatible certificate is always applicable")
            }

            #[deprecated(
                since = "1.5.0",
                note = "use Mapper::map_cert or Mapper::map_cert_kind"
            )]
            pub fn map_conway_cert(
                &self,
                x: &pallas_primitives::conway::Certificate,
                tx: &pallas_traverse::MultiEraTx,
                order: u32,
            ) -> u5c::Certificate {
                let cert = pallas_traverse::MultiEraCert::Conway(Box::new(
                    std::borrow::Cow::Borrowed(x),
                ));

                self.map_cert(&cert, tx, order)
                    .expect("a Conway certificate is always applicable")
            }
        }

        #[cfg(test)]
        mod before_v1_tests {
            use super::cert_tests::{
                credential, key_credential, tx_with_redeemers, NoLedger, CREDENTIAL, EPOCH, POOL,
            };
            use super::*;

            use pallas_primitives::{alonzo, conway};
            use pallas_traverse::MultiEraTx;
            use pretty_assertions::assert_eq;

            #[test]
            #[allow(deprecated)]
            fn the_era_specific_mappers_map_through_the_certificate_view() {
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

                let conway_certificate = conway::Certificate::StakeRegistration(credential());

                assert_eq!(
                    mapper.map_conway_cert(&conway_certificate, &tx, 0),
                    u5c::Certificate {
                        certificate: Some(
                            u5c::certificate::Certificate::StakeRegistration(key_credential(
                                CREDENTIAL
                            ))
                        ),
                        redeemer: None,
                    }
                );
                assert_eq!(
                    mapper
                        .map_conway_cert(&conway_certificate, &tx, 1)
                        .redeemer
                        .map(|r| r.index),
                    Some(1),
                    "position one is indexed by a certificate redeemer"
                );

                let alonzo_certificate = alonzo::Certificate::PoolRetirement(POOL.into(), EPOCH);

                assert_eq!(
                    mapper.map_alonzo_compatible_cert(&alonzo_certificate, &tx, 0),
                    u5c::Certificate {
                        certificate: Some(u5c::certificate::Certificate::PoolRetirement(
                            u5c::PoolRetirementCert {
                                pool_keyhash: POOL.to_vec().into(),
                                epoch: EPOCH,
                            }
                        )),
                        redeemer: None,
                    }
                );
                assert_eq!(
                    mapper
                        .map_alonzo_compatible_cert(&alonzo_certificate, &tx, 1)
                        .redeemer
                        .map(|r| r.index),
                    Some(1),
                    "position one is indexed by a certificate redeemer"
                );
            }

            #[test]
            fn a_rational_wider_than_32_bits_keeps_the_low_32_bits_of_each_field() {
                let x = rational_number_to_u5c(pallas_primitives::RationalNumber {
                    numerator: (1 << 63) + 1,
                    denominator: 10_000_000_000_000_000_000,
                });

                assert_eq!((x.numerator, x.denominator), (1, 2_313_682_944));
            }

            #[test]
            fn a_plutus_integer_beyond_i64_keeps_its_low_64_bits() {
                use u5c::big_int::BigInt::Int;

                let mapper = Mapper::new(NoLedger);
                let map = |n: i128| {
                    let x = alonzo::BigInt::Int(pallas_primitives::Int::try_from(n).unwrap());
                    mapper.map_plutus_bigint(&x).big_int.unwrap()
                };

                assert_eq!(map(1 << 63), Int(i64::MIN));
                assert_eq!(map(-(1 << 63) - 1), Int(i64::MAX));
            }

            #[test]
            #[cfg(feature = "unstable")]
            fn withdrawals_map_in_the_byte_order_of_their_reward_accounts() {
                use crate::testing::{
                    conway_tx_with_key_and_script_withdrawals, KEY_REWARD_ACCOUNT,
                    SCRIPT_REWARD_ACCOUNT,
                };

                let mapped =
                    Mapper::new(NoLedger).map_tx(&conway_tx_with_key_and_script_withdrawals());
                let withdrawals: Vec<_> = mapped
                    .withdrawals
                    .iter()
                    .map(|x| (x.reward_account.to_vec(), x.redeemer.is_some()))
                    .collect();

                assert_eq!(
                    withdrawals,
                    vec![
                        (KEY_REWARD_ACCOUNT.to_vec(), true),
                        (SCRIPT_REWARD_ACCOUNT.to_vec(), false),
                    ]
                );
            }

            #[cfg(feature = "unstable")]
            fn native(x: u5c::NativeScript) -> u5c::Script {
                u5c::Script {
                    script: Some(u5c::script::Script::Native(x)),
                }
            }

            #[cfg(feature = "unstable")]
            fn unset() -> u5c::NativeScript {
                u5c::NativeScript { native_script: None }
            }

            #[test]
            #[cfg(feature = "unstable")]
            fn a_dijkstra_guard_clause_maps_to_unset_beside_its_aux_plutus_v4_script() {
                let cbor = super::script_tests::fixture("dijkstra-scripts.tx");
                let tx = MultiEraTx::decode_for_era(pallas_traverse::Era::Dijkstra, &cbor)
                    .expect("the fixture decodes");
                let mapper = Mapper::new(NoLedger);

                assert_eq!(mapper.collect_all_scripts(&tx), vec![native(unset())]);
                assert_eq!(
                    mapper.collect_all_aux_scripts(&tx),
                    vec![
                        native(unset()),
                        u5c::Script {
                            script: u5c::script::Script::PlutusV4(vec![0xd8, 0x79, 0x80].into())
                                .into(),
                        },
                    ]
                );
            }

            #[test]
            #[cfg(feature = "unstable")]
            fn a_nested_dijkstra_guard_clause_keeps_its_position() {
                use pallas_primitives::dijkstra;

                let script = dijkstra::NativeScript::ScriptAll(vec![
                    dijkstra::NativeScript::ScriptRequireGuard(
                        pallas_primitives::StakeCredential::AddrKeyhash([7; 28].into()),
                    ),
                    dijkstra::NativeScript::ScriptPubkey([1; 28].into()),
                ]);

                assert_eq!(
                    Mapper::<NoLedger>::map_any_native_script(
                        &pallas_traverse::MultiEraNativeScript::from_decoded_dijkstra(&script)
                    ),
                    u5c::NativeScript {
                        native_script: Some(u5c::native_script::NativeScript::ScriptAll(
                            u5c::NativeScriptList {
                                items: vec![
                                    unset(),
                                    Mapper::<NoLedger>::map_native_script(
                                        &alonzo::NativeScript::ScriptPubkey([1; 28].into())
                                    ),
                                ],
                            }
                        )),
                    }
                );
            }

            #[test]
            #[cfg(feature = "unstable")]
            fn a_dijkstra_guarding_redeemer_maps_to_unspecified_beside_a_withdrawal_redeemer() {
                let cbor = super::script_tests::fixture("dijkstra-guarding-redeemer.tx");
                let tx = MultiEraTx::decode_for_era(pallas_traverse::Era::Dijkstra, &cbor)
                    .expect("the fixture decodes");
                let mapper = Mapper::new(NoLedger);

                let mapped: Vec<_> = tx.redeemers().iter().map(|r| mapper.map_redeemer(r)).collect();

                assert_eq!(
                    mapped
                        .iter()
                        .map(|r| (r.purpose, r.index, r.ex_units.clone()))
                        .collect::<Vec<_>>(),
                    vec![
                        (
                            u5c::RedeemerPurpose::Reward as i32,
                            2498742861,
                            Some(u5c::ExUnits {
                                memory: 7394972775039390592,
                                steps: 4455026327047478000,
                            }),
                        ),
                        (
                            u5c::RedeemerPurpose::Unspecified as i32,
                            3723616606,
                            Some(u5c::ExUnits {
                                memory: 7982482519345238718,
                                steps: 8267307830042052150,
                            }),
                        ),
                    ],
                    "both redeemers the ledger reads map in place, the guarding one with no purpose"
                );
                assert_eq!(
                    mapped[1].original_cbor.get(..3),
                    Some(&[0x82, 0x82, 0x06][..]),
                    "the guarding redeemer's own bytes hold its tag"
                );
            }
        }
    };
}

pub(crate) use impl_cardano_mapper_before_v1;
