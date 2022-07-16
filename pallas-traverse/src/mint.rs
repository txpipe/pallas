use pallas_primitives::alonzo;

use crate::MultiEraMint;

impl Default for MultiEraMint<'_> {
    fn default() -> Self {
        MultiEraMint::Empty
    }
}

impl<'b> MultiEraMint<'b> {
    pub fn len(&self) -> usize {
        match self {
            MultiEraMint::AlonzoCompatible(x) => x.len(),
            _ => 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            MultiEraMint::AlonzoCompatible(x) => x.is_empty(),
            _ => true,
        }
    }

    pub fn as_alonzo(&self) -> Option<&alonzo::Mint> {
        match self {
            Self::AlonzoCompatible(x) => Some(x),
            _ => None,
        }
    }
}
