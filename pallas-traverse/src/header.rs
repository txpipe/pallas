use std::borrow::Cow;
use std::ops::Deref;

use pallas_codec::minicbor;
use pallas_crypto::hash::Hash;
use pallas_primitives::{alonzo, babbage, byron};

#[cfg(feature = "unstable")]
use pallas_primitives::dijkstra;

use crate::{Era, Error, MultiEraHeader, OriginalHash, wellknown::GenesisValues};

impl<'b> MultiEraHeader<'b> {
    /// Decode a header given the era tag of the chainsync header envelope.
    ///
    /// Not the block wrapper tag. The wrapper counts Byron twice, so from
    /// Shelley onward the envelope tag is one less than the wrapper tag.
    pub fn decode(tag: u8, subtag: Option<u8>, cbor: &'b [u8]) -> Result<Self, Error> {
        match tag {
            0 => match subtag {
                Some(0) => {
                    let header = minicbor::decode(cbor).map_err(Error::invalid_cbor)?;
                    Ok(MultiEraHeader::EpochBoundary(Cow::Owned(header)))
                }
                _ => {
                    let header = minicbor::decode(cbor).map_err(Error::invalid_cbor)?;
                    Ok(MultiEraHeader::Byron(Cow::Owned(header)))
                }
            },
            1..=4 => {
                let header = minicbor::decode(cbor).map_err(Error::invalid_cbor)?;
                Ok(MultiEraHeader::ShelleyCompatible(Cow::Owned(header)))
            }
            #[cfg(feature = "unstable")]
            5 | 6 => {
                let header = minicbor::decode(cbor).map_err(Error::invalid_cbor)?;
                Ok(MultiEraHeader::BabbageCompatible(Cow::Owned(header)))
            }
            #[cfg(feature = "unstable")]
            7 => {
                let header = minicbor::decode(cbor).map_err(Error::invalid_cbor)?;
                Ok(MultiEraHeader::Dijkstra(Cow::Owned(header)))
            }
            #[cfg(feature = "unstable")]
            unknown => Err(Error::UnknownEra(unknown.into())),
            #[cfg(not(feature = "unstable"))]
            _ => {
                let header = minicbor::decode(cbor).map_err(Error::invalid_cbor)?;
                Ok(MultiEraHeader::BabbageCompatible(Cow::Owned(header)))
            }
        }
    }

    /// The era whose header rule carries this value. The two shared shapes
    /// name the first era of their group rather than the era of the header.
    pub fn era(&self) -> Era {
        match self {
            MultiEraHeader::EpochBoundary(_) => Era::Byron,
            MultiEraHeader::Byron(_) => Era::Byron,
            MultiEraHeader::ShelleyCompatible(_) => Era::Shelley,
            MultiEraHeader::BabbageCompatible(_) => Era::Babbage,
            #[cfg(feature = "unstable")]
            MultiEraHeader::Dijkstra(_) => Era::Dijkstra,
        }
    }

    pub fn cbor(&self) -> &[u8] {
        match self {
            MultiEraHeader::EpochBoundary(x) => x.raw_cbor(),
            MultiEraHeader::ShelleyCompatible(x) => x.raw_cbor(),
            MultiEraHeader::BabbageCompatible(x) => x.raw_cbor(),
            MultiEraHeader::Byron(x) => x.raw_cbor(),
            #[cfg(feature = "unstable")]
            MultiEraHeader::Dijkstra(x) => x.raw_cbor(),
        }
    }

    pub fn number(&self) -> u64 {
        match self {
            MultiEraHeader::EpochBoundary(x) => x
                .consensus_data
                .difficulty
                .first()
                .cloned()
                .unwrap_or_default(),
            MultiEraHeader::ShelleyCompatible(x) => x.header_body.block_number,
            MultiEraHeader::BabbageCompatible(x) => x.header_body.block_number,
            MultiEraHeader::Byron(x) => x.consensus_data.2.first().cloned().unwrap_or_default(),
            #[cfg(feature = "unstable")]
            MultiEraHeader::Dijkstra(x) => x.header_body.block_number,
        }
    }

    pub fn slot(&self) -> u64 {
        match self {
            MultiEraHeader::ShelleyCompatible(x) => x.header_body.slot,
            MultiEraHeader::BabbageCompatible(x) => x.header_body.slot,
            MultiEraHeader::EpochBoundary(x) => {
                let genesis = GenesisValues::default();
                genesis.relative_slot_to_absolute(x.consensus_data.epoch_id, 0)
            }
            MultiEraHeader::Byron(x) => {
                let genesis = GenesisValues::default();
                genesis.relative_slot_to_absolute(x.consensus_data.0.epoch, x.consensus_data.0.slot)
            }
            #[cfg(feature = "unstable")]
            MultiEraHeader::Dijkstra(x) => x.header_body.slot,
        }
    }

    pub fn hash(&self) -> Hash<32> {
        match self {
            MultiEraHeader::EpochBoundary(x) => x.original_hash(),
            MultiEraHeader::ShelleyCompatible(x) => x.original_hash(),
            MultiEraHeader::BabbageCompatible(x) => x.original_hash(),
            MultiEraHeader::Byron(x) => x.original_hash(),
            #[cfg(feature = "unstable")]
            MultiEraHeader::Dijkstra(x) => x.original_hash(),
        }
    }

    pub fn previous_hash(&self) -> Option<Hash<32>> {
        match self {
            MultiEraHeader::ShelleyCompatible(x) => x.header_body.prev_hash,
            MultiEraHeader::BabbageCompatible(x) => x.header_body.prev_hash,
            MultiEraHeader::EpochBoundary(x) => Some(x.prev_block),
            MultiEraHeader::Byron(x) => Some(x.prev_block),
            #[cfg(feature = "unstable")]
            MultiEraHeader::Dijkstra(x) => x.header_body.prev_hash,
        }
    }

    pub fn vrf_vkey(&self) -> Option<&[u8]> {
        match self {
            MultiEraHeader::ShelleyCompatible(x) => Some(x.header_body.vrf_vkey.as_ref()),
            MultiEraHeader::BabbageCompatible(x) => Some(x.header_body.vrf_vkey.as_ref()),
            MultiEraHeader::EpochBoundary(_) => None,
            MultiEraHeader::Byron(_) => None,
            #[cfg(feature = "unstable")]
            MultiEraHeader::Dijkstra(x) => Some(x.header_body.vrf_vkey.as_ref()),
        }
    }

    pub fn issuer_vkey(&self) -> Option<&[u8]> {
        match self {
            MultiEraHeader::ShelleyCompatible(x) => Some(x.header_body.issuer_vkey.as_ref()),
            MultiEraHeader::BabbageCompatible(x) => Some(x.header_body.issuer_vkey.as_ref()),
            MultiEraHeader::EpochBoundary(_) => None,
            MultiEraHeader::Byron(_) => None,
            #[cfg(feature = "unstable")]
            MultiEraHeader::Dijkstra(x) => Some(x.header_body.issuer_vkey.as_ref()),
        }
    }

    pub fn leader_vrf_output(&self) -> Result<Vec<u8>, Error> {
        match self {
            MultiEraHeader::EpochBoundary(_) => Err(Error::InvalidEra(Era::Byron)),
            MultiEraHeader::ShelleyCompatible(x) => Ok(x.header_body.leader_vrf.0.to_vec()),
            MultiEraHeader::BabbageCompatible(x) => Ok(x.header_body.leader_vrf_output()),
            MultiEraHeader::Byron(_) => Err(Error::InvalidEra(Era::Byron)),
            #[cfg(feature = "unstable")]
            MultiEraHeader::Dijkstra(x) => Ok(x.header_body.leader_vrf_output()),
        }
    }

    pub fn nonce_vrf_output(&self) -> Result<Vec<u8>, Error> {
        match self {
            MultiEraHeader::EpochBoundary(_) => Err(Error::InvalidEra(Era::Byron)),
            MultiEraHeader::ShelleyCompatible(x) => Ok(x.header_body.nonce_vrf.0.to_vec()),
            MultiEraHeader::BabbageCompatible(x) => Ok(x.header_body.nonce_vrf_output()),
            MultiEraHeader::Byron(_) => Err(Error::InvalidEra(Era::Byron)),
            #[cfg(feature = "unstable")]
            MultiEraHeader::Dijkstra(x) => Ok(x.header_body.nonce_vrf_output()),
        }
    }

    /// Whether this body carries a Leios certificate. `None` before Dijkstra.
    #[cfg(feature = "unstable")]
    pub fn block_body_contains_leios_cert(&self) -> Option<bool> {
        match self {
            MultiEraHeader::Dijkstra(x) => Some(x.header_body.block_body_contains_leios_cert),
            _ => None,
        }
    }

    /// The endorser block this header announces. `None` both for an era with
    /// no announcement field and for a Dijkstra header that left it nil.
    #[cfg(feature = "unstable")]
    pub fn eb_announcement(&self) -> Option<&dijkstra::EbAnnouncement> {
        match self {
            MultiEraHeader::Dijkstra(x) => match &x.header_body.eb_announcement {
                pallas_primitives::Nullable::Some(a) => Some(a),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn as_eb(&self) -> Option<&byron::EbbHead> {
        match self {
            MultiEraHeader::EpochBoundary(x) => Some(x.deref().deref()),
            _ => None,
        }
    }

    pub fn as_byron(&self) -> Option<&byron::BlockHead> {
        match self {
            MultiEraHeader::Byron(x) => Some(x.deref().deref()),
            _ => None,
        }
    }

    pub fn as_alonzo(&self) -> Option<&alonzo::Header> {
        match self {
            MultiEraHeader::ShelleyCompatible(x) => Some(x.deref().deref()),
            _ => None,
        }
    }

    pub fn as_babbage(&self) -> Option<&babbage::Header> {
        match self {
            MultiEraHeader::BabbageCompatible(x) => Some(x.deref()),
            _ => None,
        }
    }

    #[cfg(feature = "unstable")]
    pub fn as_dijkstra(&self) -> Option<&dijkstra::Header> {
        match self {
            MultiEraHeader::Dijkstra(x) => Some(x.deref()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MultiEraBlock;

    fn header_of(block_str: &str) -> Vec<u8> {
        let cbor = hex::decode(block_str).unwrap();
        let block = MultiEraBlock::decode(&cbor).unwrap();
        block.header().cbor().to_vec()
    }

    #[cfg(feature = "unstable")]
    fn dijkstra_header_bytes() -> Vec<u8> {
        header_of(include_str!("../../test_data/dijkstra1.block"))
    }

    #[cfg(feature = "unstable")]
    #[test]
    fn dijkstra_header_decodes_as_dijkstra() {
        let raw = dijkstra_header_bytes();
        let header = MultiEraHeader::decode(7, None, &raw).unwrap();

        assert!(matches!(header, MultiEraHeader::Dijkstra(_)));
        assert_eq!(header.era(), Era::Dijkstra);
        assert!(header.as_dijkstra().is_some());
        assert_eq!(header.block_body_contains_leios_cert(), Some(false));
        assert!(header.eb_announcement().is_none());
    }

    #[cfg(feature = "unstable")]
    #[test]
    fn a_header_that_announces_an_endorser_block_says_so() {
        let raw = header_of(include_str!("../../test_data/dijkstra8.block"));
        let header = MultiEraHeader::decode(7, None, &raw).unwrap();

        assert_eq!(header.era(), Era::Dijkstra);
        assert_eq!(header.block_body_contains_leios_cert(), Some(true));

        let announcement = header
            .eb_announcement()
            .expect("dijkstra8 announces an endorser block");
        assert_eq!(announcement.eb_size, 39_495);
        assert_eq!(
            hex::encode(announcement.eb_hash),
            "de5f4b812d0e6dc3129510c6663de4ea99bbd6bf019ec2d541853242926fd446"
        );
    }

    #[cfg(feature = "unstable")]
    #[test]
    fn the_certificate_flag_and_the_announcement_are_read_separately() {
        let raw = header_of(include_str!("../../test_data/dijkstra9.block"));
        let header = MultiEraHeader::decode(7, None, &raw).unwrap();
        assert_eq!(header.block_body_contains_leios_cert(), Some(true));
        assert!(
            header.eb_announcement().is_none(),
            "dijkstra9 certifies without announcing"
        );

        let raw = header_of(include_str!("../../test_data/dijkstra11.block"));
        let header = MultiEraHeader::decode(7, None, &raw).unwrap();
        assert_eq!(header.block_body_contains_leios_cert(), Some(false));
        assert!(
            header.eb_announcement().is_none(),
            "dijkstra11 neither certifies nor announces"
        );
    }

    #[cfg(feature = "unstable")]
    #[test]
    fn an_earlier_era_header_has_no_leios_fields_at_all() {
        let raw = header_of(include_str!("../../test_data/conway1.block"));
        let header = MultiEraHeader::decode(6, None, &raw).unwrap();

        assert_eq!(header.era(), Era::Babbage);
        assert_eq!(
            header.block_body_contains_leios_cert(),
            None,
            "a ten field header has no certificate flag to report"
        );
        assert!(header.eb_announcement().is_none());

        let dijkstra = header_of(include_str!("../../test_data/dijkstra9.block"));
        let dijkstra = MultiEraHeader::decode(7, None, &dijkstra).unwrap();
        assert!(dijkstra.eb_announcement().is_none());
        assert_ne!(
            dijkstra.block_body_contains_leios_cert(),
            header.block_body_contains_leios_cert(),
            "the flag is what separates the two, and it does"
        );
    }

    #[cfg(feature = "unstable")]
    #[test]
    fn envelope_tag_seven_reads_the_slot_the_fixture_record_names() {
        let raw = header_of(include_str!("../../test_data/dijkstra2.block"));

        let header = MultiEraHeader::decode(7, None, &raw).unwrap();

        assert_eq!(header.era(), Era::Dijkstra);
        assert_eq!(header.slot(), 86855);
    }

    #[cfg(feature = "unstable")]
    #[test]
    fn the_conway_tag_reads_a_dijkstra_header_as_babbage_and_drops_two_fields() {
        let raw = dijkstra_header_bytes();

        let as_conway = MultiEraHeader::decode(6, None, &raw).unwrap();
        assert!(matches!(as_conway, MultiEraHeader::BabbageCompatible(_)));
        assert_eq!(as_conway.era(), Era::Babbage);

        let as_dijkstra = MultiEraHeader::decode(7, None, &raw).unwrap();
        assert_eq!(as_conway.number(), as_dijkstra.number());
        assert_eq!(as_conway.slot(), as_dijkstra.slot());
        assert_eq!(as_conway.hash(), as_dijkstra.hash());

        assert_eq!(
            as_conway.block_body_contains_leios_cert(),
            None,
            "a header decoded through the Conway tag cannot report Leios fields"
        );

        let short = minicbor::to_vec(as_conway.as_babbage().unwrap()).unwrap();
        assert!(
            short.len() < raw.len(),
            "the ten field encoder writes back fewer bytes than it read"
        );
    }

    #[cfg(feature = "unstable")]
    #[test]
    fn unknown_envelope_tag_is_refused() {
        let raw = header_of(include_str!("../../test_data/conway1.block"));

        for tag in [8u8, 9, 200] {
            let err = MultiEraHeader::decode(tag, None, &raw)
                .expect_err("an unknown envelope tag must be refused, not decoded as Babbage");
            assert!(
                matches!(err, Error::UnknownEra(t) if t == u16::from(tag)),
                "envelope tag {tag} gave {err:?}"
            );
        }
    }

    #[cfg(not(feature = "unstable"))]
    #[test]
    fn an_unknown_envelope_tag_decodes_as_babbage_without_the_feature() {
        let raw = header_of(include_str!("../../test_data/conway1.block"));

        for tag in [7u8, 8, 9, 200] {
            let header = match MultiEraHeader::decode(tag, None, &raw) {
                Ok(header) => header,
                Err(err) => panic!("envelope tag {tag} gave {err:?}"),
            };
            assert!(
                matches!(header, MultiEraHeader::BabbageCompatible(_)),
                "envelope tag {tag} decoded as something other than Babbage"
            );
            assert_eq!(header.era(), Era::Babbage, "envelope tag {tag}");
        }
    }

    #[test]
    fn conway_header_still_decodes() {
        let raw = header_of(include_str!("../../test_data/conway1.block"));

        let header = MultiEraHeader::decode(6, None, &raw).unwrap();
        assert!(matches!(header, MultiEraHeader::BabbageCompatible(_)));
        assert_eq!(header.era(), Era::Babbage);
        #[cfg(feature = "unstable")]
        assert_eq!(header.block_body_contains_leios_cert(), None);
    }

    #[test]
    fn every_header_variant_names_its_era() {
        #[allow(unused_mut)]
        let mut cases = vec![
            (
                0u8,
                header_of(include_str!("../../test_data/byron1.block")),
                Era::Byron,
            ),
            (
                1,
                header_of(include_str!("../../test_data/shelley1.block")),
                Era::Shelley,
            ),
            (
                6,
                header_of(include_str!("../../test_data/conway1.block")),
                Era::Babbage,
            ),
        ];
        #[cfg(feature = "unstable")]
        cases.push((7, dijkstra_header_bytes(), Era::Dijkstra));

        for (tag, raw, era) in cases {
            assert_eq!(
                MultiEraHeader::decode(tag, None, &raw).unwrap().era(),
                era,
                "envelope tag {tag}"
            );
        }
    }
}
