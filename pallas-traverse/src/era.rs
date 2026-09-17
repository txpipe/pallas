use std::fmt::{Display, Formatter};

use crate::{Era, Feature};

impl Era {
    #[allow(clippy::match_like_matches_macro)]
    pub fn has_feature(&self, feature: Feature) -> bool {
        match feature {
            Feature::Staking => self.ge(&Era::Shelley),
            Feature::MultiAssets => self.ge(&Era::Mary),
            Feature::TimeLocks => self.ge(&Era::Allegra),
            Feature::SmartContracts => self.ge(&Era::Alonzo),
            Feature::CIP31 => self.ge(&Era::Babbage),
            Feature::CIP32 => self.ge(&Era::Babbage),
            Feature::CIP33 => self.ge(&Era::Babbage),
            Feature::CIP1694 => self.ge(&Era::Conway),
        }
    }
}

// for consistency, we use the same tag convention used by the node's cbor
// encoding
impl TryFrom<u16> for Era {
    type Error = crate::Error;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Era::Byron),
            1 => Ok(Era::Byron),
            2 => Ok(Era::Shelley),
            3 => Ok(Era::Allegra),
            4 => Ok(Era::Mary),
            5 => Ok(Era::Alonzo),
            6 => Ok(Era::Babbage),
            7 => Ok(Era::Conway),
            #[cfg(feature = "unstable")]
            8 => Ok(Era::Dijkstra),
            x => Err(crate::Error::UnknownEra(x)),
        }
    }
}

impl From<Era> for u16 {
    fn from(other: Era) -> Self {
        match other {
            Era::Byron => 1,
            Era::Shelley => 2,
            Era::Allegra => 3,
            Era::Mary => 4,
            Era::Alonzo => 5,
            Era::Babbage => 6,
            Era::Conway => 7,
            #[cfg(feature = "unstable")]
            Era::Dijkstra => 8,
        }
    }
}

impl Display for Era {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Era::Byron => write!(f, "Byron"),
            Era::Shelley => write!(f, "Shelley"),
            Era::Allegra => write!(f, "Allegra"),
            Era::Mary => write!(f, "Mary"),
            Era::Alonzo => write!(f, "Alonzo"),
            Era::Babbage => write!(f, "Babbage"),
            Era::Conway => write!(f, "Conway"),
            #[cfg(feature = "unstable")]
            Era::Dijkstra => write!(f, "Dijkstra"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_era_round_trips_through_its_tag() {
        #[allow(unused_mut)]
        let mut eras = vec![
            Era::Byron,
            Era::Shelley,
            Era::Allegra,
            Era::Mary,
            Era::Alonzo,
            Era::Babbage,
            Era::Conway,
        ];
        #[cfg(feature = "unstable")]
        eras.push(Era::Dijkstra);

        for era in eras {
            let tag: u16 = era.into();
            let back = Era::try_from(tag)
                .unwrap_or_else(|e| panic!("{era} encodes to tag {tag} but does not decode: {e}"));
            assert_eq!(back, era, "tag {tag} did not round trip");
        }
    }

    #[test]
    fn unknown_era_tags_are_still_refused() {
        for tag in [9u16, 10, 4242] {
            assert!(Era::try_from(tag).is_err(), "tag {tag} must be refused");
        }
    }

    #[cfg(not(feature = "unstable"))]
    #[test]
    fn tag_eight_is_unknown_without_the_feature() {
        assert!(
            Era::try_from(8).is_err(),
            "tag 8 must be refused when the Dijkstra era is not compiled in"
        );
    }

    #[cfg(feature = "unstable")]
    #[test]
    fn dijkstra_sorts_after_conway_and_inherits_its_features() {
        assert!(Era::Dijkstra > Era::Conway);
        assert!(Era::Dijkstra.has_feature(crate::Feature::CIP1694));
        assert!(Era::Dijkstra.has_feature(crate::Feature::SmartContracts));
        assert!(Era::Dijkstra.has_feature(crate::Feature::MultiAssets));
        assert!(Era::Dijkstra.has_feature(crate::Feature::CIP31));
        assert!(!Era::Babbage.has_feature(crate::Feature::CIP1694));
        assert!(!Era::Byron.has_feature(crate::Feature::Staking));
    }
}
