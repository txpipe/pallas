/// Emits the parts of `Mapper<C>` that only the schemas before v1 keep.
macro_rules! impl_cardano_mapper_before_v1 {
    () => {
        /// The message this schema maps a protocol parameter update to.
        type ParamUpdate = u5c::PParams;

        /// The value this schema's update message holds for a scalar key, its proto3 zero when the update leaves the key unset.
        fn update_scalar<T: Default>(value: Option<T>) -> T {
            value.unwrap_or_default()
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

        /// Panics on a guarding redeemer, for which this schema has no purpose.
        #[cfg(feature = "unstable")]
        fn guarding_purpose() -> u5c::RedeemerPurpose {
            unimplemented!("map_redeemer is not yet implemented for Dijkstra")
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
            fn a_dijkstra_guard_clause_maps_to_an_unset_native_script() {
                let cbor = super::script_tests::fixture("dijkstra-scripts.tx");
                let tx = MultiEraTx::decode_for_era(pallas_traverse::Era::Dijkstra, &cbor)
                    .expect("the fixture decodes");
                let mapper = Mapper::new(NoLedger);

                assert_eq!(mapper.collect_all_scripts(&tx), vec![native(unset())]);
                assert_eq!(mapper.collect_all_aux_scripts(&tx), vec![native(unset())]);
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
        }
    };
}

pub(crate) use impl_cardano_mapper_before_v1;
