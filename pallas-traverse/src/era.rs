use crate::{Era, Feature};

impl Era {
    #[allow(clippy::match_like_matches_macro)]
    pub fn has_feature(&self, feature: Feature) -> bool {
        match (self, feature) {
            (Era::Byron, _) => false,
            (Era::Shelley, Feature::SmartContracts) => false,
            (Era::Shelley, Feature::TimeLocks) => false,
            (Era::Shelley, Feature::MultiAssets) => false,
            (Era::Allegra, Feature::MultiAssets) => false,
            (Era::Allegra, Feature::SmartContracts) => false,
            (Era::Mary, Feature::SmartContracts) => false,
            _ => true,
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
            x => Err(crate::Error::UnkownEra(x)),
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
        }
    }
}
