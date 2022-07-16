use pallas_primitives::alonzo;

use crate::MultiEraCert;

impl<'b> MultiEraCert<'b> {
    pub fn as_alonzo(&self) -> Option<&alonzo::Certificate> {
        match self {
            MultiEraCert::AlonzoCompatible(x) => Some(x),
            _ => None,
        }
    }
}
