use pallas_primitives::alonzo;

use crate::MultiEraCert;

impl<'b> MultiEraCert<'b> {
    pub fn as_alonzo(&'b self) -> Option<&'b alonzo::Certificate> {
        match self {
            MultiEraCert::NotApplicable => None,
            MultiEraCert::AlonzoCompatible(x) => Some(x),
        }
    }
}
