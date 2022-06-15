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
