use pallas_primitives::alonzo;

use crate::MultiEraMeta;

impl Default for MultiEraMeta<'_> {
    fn default() -> Self {
        MultiEraMeta::Empty
    }
}

impl<'b> MultiEraMeta<'b> {
    pub fn as_alonzo(&self) -> Option<&alonzo::Metadata> {
        match self {
            Self::AlonzoCompatible(x) => Some(x),
            _ => None,
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            MultiEraMeta::AlonzoCompatible(x) => x.is_empty(),
            _ => true,
        }
    }

    pub fn find(&self, label: alonzo::MetadatumLabel) -> Option<&alonzo::Metadatum> {
        self.as_alonzo()?
            .iter()
            .find_map(|(key, value)| if key.eq(&label) { Some(value) } else { None })
    }

    pub fn collect<'a, T>(&'a self) -> T
    where
        T: FromIterator<(&'a alonzo::MetadatumLabel, &'a alonzo::Metadatum)>,
    {
        match self {
            MultiEraMeta::NotApplicable => T::from_iter(std::iter::empty()),
            MultiEraMeta::Empty => T::from_iter(std::iter::empty()),
            MultiEraMeta::AlonzoCompatible(x) => {
                let iter = x.iter().map(|(k, v)| (k, v));
                T::from_iter(iter)
            }
        }
    }
}
