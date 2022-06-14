use pallas_primitives::alonzo;

use crate::MultiEraCert;

impl<'b> MultiEraCert<'b> {
    pub fn as_alonzo(&self) -> Option<&alonzo::Certificate> {
        match self {
            MultiEraCert::NotApplicable => None,
            MultiEraCert::AlonzoCompatible(x) => Some(x),
        }
    }
}
