use pallas_primitives::{alonzo, conway};

#[cfg(feature = "unstable")]
use std::ops::Deref;

#[cfg(feature = "unstable")]
use pallas_primitives::dijkstra;

use crate::{Era, MultiEraCert};

impl MultiEraCert<'_> {
    pub fn as_alonzo(&self) -> Option<&alonzo::Certificate> {
        match self {
            MultiEraCert::AlonzoCompatible(x) => Some(x),
            _ => None,
        }
    }

    pub fn as_conway(&self) -> Option<&conway::Certificate> {
        match self {
            MultiEraCert::Conway(x) => Some(x),
            _ => None,
        }
    }

    #[cfg(feature = "unstable")]
    pub fn as_dijkstra(&self) -> Option<&dijkstra::Certificate> {
        match self {
            MultiEraCert::Dijkstra(x) => Some(x),
            _ => None,
        }
    }

    pub fn era(&self) -> Option<Era> {
        match self {
            MultiEraCert::NotApplicable => None,
            // The Alonzo type serves Shelley through Babbage, so this arm names the
            // first era of that group.
            MultiEraCert::AlonzoCompatible(_) => Some(Era::Alonzo),
            MultiEraCert::Conway(_) => Some(Era::Conway),
            #[cfg(feature = "unstable")]
            MultiEraCert::Dijkstra(_) => Some(Era::Dijkstra),
        }
    }

    #[cfg(feature = "unstable")]
    pub fn bls_key(&self) -> BlsKeySlot<'_> {
        match self {
            MultiEraCert::Dijkstra(x) => match x.deref().deref() {
                dijkstra::Certificate::PoolRegistration { bls_key, .. } => match bls_key {
                    None => BlsKeySlot::NoSlot,
                    Some(pallas_primitives::Nullable::Some(k)) => BlsKeySlot::Key(k),
                    Some(_) => BlsKeySlot::Null,
                },
                _ => BlsKeySlot::NotAPoolRegistration,
            },
            MultiEraCert::AlonzoCompatible(x) => match x.deref().deref() {
                alonzo::Certificate::PoolRegistration { .. } => BlsKeySlot::NoSlot,
                _ => BlsKeySlot::NotAPoolRegistration,
            },
            MultiEraCert::Conway(x) => match x.deref().deref() {
                conway::Certificate::PoolRegistration { .. } => BlsKeySlot::NoSlot,
                _ => BlsKeySlot::NotAPoolRegistration,
            },
            MultiEraCert::NotApplicable => BlsKeySlot::NotAPoolRegistration,
        }
    }
}

#[cfg(feature = "unstable")]
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum BlsKeySlot<'b> {
    NotAPoolRegistration,
    /// A pool registration with no BLS key slot, or one the transaction omitted.
    NoSlot,
    /// A pool registration that wrote the slot as nil.
    Null,
    Key(&'b dijkstra::BlsKey),
}

#[cfg(feature = "unstable")]
impl BlsKeySlot<'_> {
    pub fn key(&self) -> Option<&dijkstra::BlsKey> {
        match self {
            BlsKeySlot::Key(k) => Some(k),
            BlsKeySlot::NotAPoolRegistration | BlsKeySlot::NoSlot | BlsKeySlot::Null => None,
        }
    }
}

#[cfg(all(test, feature = "unstable"))]
mod tests {
    use super::*;
    use crate::MultiEraBlock;

    fn block(hex_str: &str) -> Vec<u8> {
        hex::decode(hex_str).expect("invalid hex")
    }

    #[test]
    fn every_dijkstra_fixture_certificate_is_readable() {
        let cases = [
            (
                include_str!("../../test_data/dijkstra2.block"),
                1usize,
                1usize,
            ),
            (include_str!("../../test_data/dijkstra3.block"), 0, 0),
            (include_str!("../../test_data/dijkstra4.block"), 2, 1),
            (include_str!("../../test_data/dijkstra5.block"), 2, 1),
            (include_str!("../../test_data/dijkstra6.block"), 5, 2),
            (include_str!("../../test_data/dijkstra7.block"), 1, 0),
            (include_str!("../../test_data/dijkstra10.block"), 2, 1),
            (include_str!("../../test_data/dijkstra13.block"), 2, 0),
        ];

        let mut total = 0usize;
        let mut with_key = 0usize;

        for (block_str, certs, registrations) in cases {
            let cbor = block(block_str);
            let decoded = MultiEraBlock::decode(&cbor).expect("invalid cbor");

            let txs = decoded.txs();
            let all: Vec<_> = txs.iter().flat_map(|tx| tx.certs()).collect();
            assert_eq!(all.len(), certs, "certificate count");

            let mut registrations_seen = 0usize;
            for cert in all.iter() {
                assert_eq!(cert.era(), Some(Era::Dijkstra));
                assert!(
                    cert.as_dijkstra().is_some(),
                    "a Dijkstra certificate must be readable as one"
                );
                assert!(cert.as_conway().is_none());
                assert!(cert.as_alonzo().is_none());

                if let Some(key) = cert.bls_key().key() {
                    assert_eq!(key.bls_pubkey.len(), 96, "bls_pubkey is 96 bytes");
                    assert_eq!(
                        key.bls_possession_proof.len(),
                        48,
                        "bls_possession_proof is 48 bytes"
                    );
                    registrations_seen += 1;
                    with_key += 1;
                }
            }

            assert_eq!(
                registrations_seen, registrations,
                "pool registrations carrying a populated bls_key"
            );
            total += all.len();
        }

        assert_eq!(
            total, 15,
            "certificates across the seven fixtures that have any"
        );
        assert_eq!(with_key, 6, "pool registrations carrying a BLS key");
    }

    #[test]
    fn a_vote_delegation_certificate_is_read_through_the_multi_era_cert() {
        let cbor = block(include_str!("../../test_data/dijkstra13.block"));
        let decoded = MultiEraBlock::decode(&cbor).expect("invalid cbor");

        let txs = decoded.txs();
        let all: Vec<_> = txs.iter().flat_map(|tx| tx.certs()).collect();
        assert_eq!(all.len(), 2, "a registration and a vote delegation");

        let delegation = all
            .iter()
            .find(|c| matches!(c.as_dijkstra(), Some(dijkstra::Certificate::VoteDeleg(..))))
            .expect("dijkstra13 carries a vote delegation certificate");

        assert_eq!(delegation.era(), Some(Era::Dijkstra));
        assert!(delegation.as_conway().is_none());
        assert!(delegation.as_alonzo().is_none());

        assert!(
            delegation.bls_key().key().is_none(),
            "a vote delegation carries no pool parameters"
        );

        let Some(dijkstra::Certificate::VoteDeleg(credential, drep)) = delegation.as_dijkstra()
        else {
            unreachable!()
        };
        assert!(matches!(
            credential,
            pallas_primitives::StakeCredential::AddrKeyhash(_)
        ));
        assert!(
            matches!(drep, dijkstra::DRep::Abstain),
            "this delegation is to the predefined abstain drep, found {drep:?}"
        );

        assert!(
            all.iter()
                .any(|c| matches!(c.as_dijkstra(), Some(dijkstra::Certificate::Reg(..)))),
            "the other certificate is a registration"
        );
    }

    #[test]
    fn an_earlier_era_certificate_is_not_read_as_dijkstra() {
        let cbor = block(include_str!("../../test_data/babbage10.block"));
        let decoded = MultiEraBlock::decode(&cbor).expect("invalid cbor");

        let txs = decoded.txs();
        let all: Vec<_> = txs.iter().flat_map(|tx| tx.certs()).collect();
        assert!(!all.is_empty(), "this fixture carries body key 4");

        for cert in all.iter() {
            assert_eq!(cert.era(), Some(Era::Alonzo));
            assert!(cert.as_alonzo().is_some());
            assert!(cert.as_dijkstra().is_none());
            assert!(cert.as_conway().is_none());
            assert!(
                cert.bls_key().key().is_none(),
                "no era before Dijkstra has a BLS key slot"
            );
        }
    }
}
