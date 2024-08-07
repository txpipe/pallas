use pallas_primitives::alonzo;

use crate::MultiEraWithdrawals;

impl<'b> MultiEraWithdrawals<'b> {
    pub fn as_alonzo(&self) -> Option<&alonzo::Withdrawals> {
        match self {
            Self::AlonzoCompatible(x) => Some(x),
            _ => None,
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            MultiEraWithdrawals::AlonzoCompatible(x) => x.is_empty(),
            _ => true,
        }
    }

    pub fn collect<'a, T>(&'a self) -> T
    where
        T: FromIterator<(&'a [u8], u64)>,
    {
        match self {
            MultiEraWithdrawals::NotApplicable => std::iter::empty().collect(),
            MultiEraWithdrawals::Empty => std::iter::empty().collect(),
            MultiEraWithdrawals::AlonzoCompatible(x) => {
                x.iter().map(|(k, v)| (k.as_slice(), *v)).collect()
            }
            MultiEraWithdrawals::Conway(x) => x.iter().map(|(k, v)| (k.as_slice(), *v)).collect(),
        }
    }
}
