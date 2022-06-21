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
        }
    }
}
