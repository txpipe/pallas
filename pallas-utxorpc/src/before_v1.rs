/// Emits the parts of `Mapper<C>` that only the schemas before v1 keep.
macro_rules! impl_cardano_mapper_before_v1 {
    () => {
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
        }
    };
}

pub(crate) use impl_cardano_mapper_before_v1;
