use std::borrow::Cow;

use pallas_codec::minicbor;
use pallas_primitives::{alonzo, conway};

#[cfg(feature = "unstable")]
use pallas_primitives::dijkstra;

use crate::{MultiEraRedeemer, MultiEraRedeemerTag};

impl<'b> MultiEraRedeemer<'b> {
    pub fn multi_era_tag(&self) -> MultiEraRedeemerTag {
        match &self {
            Self::AlonzoCompatible(x) => x.tag.into(),
            Self::Conway(x, _) => x.tag.into(),
            #[cfg(feature = "unstable")]
            Self::Dijkstra(x, _) => x.tag.into(),
        }
    }

    #[deprecated(
        since = "1.5.0",
        note = "use multi_era_tag. This method cannot represent the Dijkstra Guarding tag"
    )]
    pub fn tag(&self) -> conway::RedeemerTag {
        match &self {
            Self::AlonzoCompatible(x) => match x.tag {
                alonzo::RedeemerTag::Cert => conway::RedeemerTag::Cert,
                alonzo::RedeemerTag::Spend => conway::RedeemerTag::Spend,
                alonzo::RedeemerTag::Mint => conway::RedeemerTag::Mint,
                alonzo::RedeemerTag::Reward => conway::RedeemerTag::Reward,
            },
            Self::Conway(x, _) => x.tag,
            #[cfg(feature = "unstable")]
            Self::Dijkstra(x, _) => match x.tag {
                dijkstra::RedeemerTag::Cert => conway::RedeemerTag::Cert,
                dijkstra::RedeemerTag::Spend => conway::RedeemerTag::Spend,
                dijkstra::RedeemerTag::Mint => conway::RedeemerTag::Mint,
                dijkstra::RedeemerTag::Reward => conway::RedeemerTag::Reward,
                dijkstra::RedeemerTag::Vote => conway::RedeemerTag::Vote,
                dijkstra::RedeemerTag::Propose => conway::RedeemerTag::Propose,
                dijkstra::RedeemerTag::Guarding => {
                    panic!("the Guarding tag has no Conway name, read it with multi_era_tag")
                }
            },
        }
    }

    pub fn data(&self) -> &alonzo::PlutusData {
        match &self {
            Self::AlonzoCompatible(x) => &x.data,
            Self::Conway(_, x) => &x.data,
            #[cfg(feature = "unstable")]
            Self::Dijkstra(_, x) => &x.data,
        }
    }

    pub fn ex_units(&self) -> alonzo::ExUnits {
        match &self {
            Self::AlonzoCompatible(x) => x.ex_units,
            Self::Conway(_, x) => x.ex_units,
            #[cfg(feature = "unstable")]
            Self::Dijkstra(_, x) => x.ex_units,
        }
    }

    pub fn index(&self) -> u32 {
        match self {
            Self::AlonzoCompatible(x) => x.index,
            Self::Conway(x, _) => x.index,
            #[cfg(feature = "unstable")]
            Self::Dijkstra(x, _) => x.index,
        }
    }

    pub fn as_alonzo(&self) -> Option<&alonzo::Redeemer> {
        match self {
            Self::AlonzoCompatible(x) => Some(x),
            Self::Conway(..) => None,
            #[cfg(feature = "unstable")]
            Self::Dijkstra(..) => None,
        }
    }

    pub fn as_conway(&self) -> Option<(&conway::RedeemersKey, &conway::RedeemersValue)> {
        match self {
            Self::AlonzoCompatible(_) => None,
            Self::Conway(x, y) => Some((x, y)),
            #[cfg(feature = "unstable")]
            Self::Dijkstra(..) => None,
        }
    }

    #[cfg(feature = "unstable")]
    pub fn as_dijkstra(&self) -> Option<(&dijkstra::RedeemersKey, &dijkstra::RedeemersValue)> {
        match self {
            Self::AlonzoCompatible(_) => None,
            Self::Conway(..) => None,
            Self::Dijkstra(x, y) => Some((x, y)),
        }
    }

    #[cfg(feature = "unstable")]
    pub fn from_dijkstra(
        redeemers_key: &'b dijkstra::RedeemersKey,
        redeemers_val: &'b dijkstra::RedeemersValue,
    ) -> Self {
        Self::Dijkstra(
            Box::new(Cow::Borrowed(redeemers_key)),
            Box::new(Cow::Borrowed(redeemers_val)),
        )
    }

    pub fn into_conway_deprecated(&self) -> Option<conway::Redeemer> {
        match self {
            Self::AlonzoCompatible(_) => None,
            Self::Conway(x, y) => Some(conway::Redeemer {
                tag: x.tag,
                index: x.index,
                data: y.data.clone(),
                ex_units: y.ex_units,
            }),
            #[cfg(feature = "unstable")]
            Self::Dijkstra(..) => None,
        }
    }

    pub fn from_alonzo_compatible(redeemer: &'b alonzo::Redeemer) -> Self {
        Self::AlonzoCompatible(Box::new(Cow::Borrowed(redeemer)))
    }

    pub fn from_conway(
        redeemers_key: &'b conway::RedeemersKey,
        redeemers_val: &'b conway::RedeemersValue,
    ) -> Self {
        Self::Conway(
            Box::new(Cow::Borrowed(redeemers_key)),
            Box::new(Cow::Borrowed(redeemers_val)),
        )
    }

    pub fn from_conway_deprecated(redeemer: &'b conway::Redeemer) -> Self {
        Self::Conway(
            Box::new(Cow::Owned(conway::RedeemersKey {
                tag: redeemer.tag,
                index: redeemer.index,
            })),
            Box::new(Cow::Owned(conway::RedeemersValue {
                data: redeemer.data.clone(),
                ex_units: redeemer.ex_units,
            })),
        )
    }

    pub fn encode(&self) -> Vec<u8> {
        match self {
            MultiEraRedeemer::AlonzoCompatible(x) => minicbor::to_vec(x).unwrap(),
            MultiEraRedeemer::Conway(k, v) => minicbor::to_vec((k, v)).unwrap(),
            #[cfg(feature = "unstable")]
            MultiEraRedeemer::Dijkstra(k, v) => minicbor::to_vec((k, v)).unwrap(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{Era, MultiEraRedeemerTag, MultiEraTx, testing};

    fn conway_redeemer_tag(tag: u8) -> MultiEraRedeemerTag {
        let cbor = testing::conway_tx(
            &testing::minimal_body(),
            &testing::witness_set_with_redeemer(tag, 0),
            None,
            true,
        );
        let tx = MultiEraTx::decode_for_era(Era::Conway, &cbor).expect("must decode");
        let redeemers = tx.redeemers();
        assert_eq!(redeemers.len(), 1, "the witness set carries one redeemer");
        redeemers[0].multi_era_tag()
    }

    #[test]
    fn a_conway_purpose_keeps_its_name_in_the_shared_tag_space() {
        assert_eq!(conway_redeemer_tag(0), MultiEraRedeemerTag::Spend);
        assert_eq!(conway_redeemer_tag(1), MultiEraRedeemerTag::Mint);
        assert_eq!(conway_redeemer_tag(2), MultiEraRedeemerTag::Cert);
        assert_eq!(conway_redeemer_tag(3), MultiEraRedeemerTag::Reward);
        assert_eq!(conway_redeemer_tag(4), MultiEraRedeemerTag::Vote);
        assert_eq!(conway_redeemer_tag(5), MultiEraRedeemerTag::Propose);
    }

    #[test]
    fn a_purpose_the_redeemer_does_not_carry_is_not_reported() {
        assert_ne!(
            conway_redeemer_tag(4),
            MultiEraRedeemerTag::Spend,
            "a vote redeemer must not be reported under the first tag in the space"
        );
    }
}

#[cfg(all(test, feature = "unstable"))]
mod dijkstra_tests {
    use super::*;
    use crate::{Era, MultiEraRedeemerTag, MultiEraTx, testing};

    fn redeemer_tx(tag: u8) -> Vec<u8> {
        testing::dijkstra_block_tx(
            &testing::minimal_body(),
            &testing::witness_set_with_redeemer(tag, 0),
            None,
            true,
        )
    }

    fn with_only_redeemer<T>(tag: u8, f: impl FnOnce(&MultiEraRedeemer<'_>) -> T) -> T {
        let cbor = redeemer_tx(tag);
        let tx = MultiEraTx::decode_for_era(Era::Dijkstra, &cbor).expect("must decode");
        let redeemers = tx.redeemers();
        assert_eq!(redeemers.len(), 1, "the witness set carries one redeemer");
        f(&redeemers[0])
    }

    #[test]
    fn a_guarding_redeemer_is_named_in_the_shared_tag_space() {
        with_only_redeemer(6, |redeemer| {
            assert_eq!(redeemer.multi_era_tag(), MultiEraRedeemerTag::Guarding);
            assert_eq!(redeemer.index(), 0);
        });
    }

    #[test]
    fn a_dijkstra_spend_redeemer_is_named_under_the_spend_tag() {
        with_only_redeemer(0, |redeemer| {
            assert_eq!(redeemer.multi_era_tag(), MultiEraRedeemerTag::Spend);
        });
    }

    #[test]
    fn a_dijkstra_purpose_the_redeemer_does_not_carry_is_not_reported() {
        with_only_redeemer(6, |redeemer| {
            assert_ne!(
                redeemer.multi_era_tag(),
                MultiEraRedeemerTag::Spend,
                "a guarding redeemer must not be reported under the first tag in the space"
            );
        });
    }

    #[test]
    #[allow(deprecated)]
    fn the_legacy_reader_answers_a_dijkstra_tag_conway_shares() {
        with_only_redeemer(0, |redeemer| {
            assert_eq!(redeemer.tag(), conway::RedeemerTag::Spend);
        });
        with_only_redeemer(4, |redeemer| {
            assert_eq!(redeemer.tag(), conway::RedeemerTag::Vote);
        });
    }

    #[test]
    #[should_panic(expected = "the Guarding tag has no Conway name")]
    #[allow(deprecated)]
    fn the_legacy_reader_refuses_the_guarding_tag() {
        with_only_redeemer(6, |redeemer| redeemer.tag());
    }
}
