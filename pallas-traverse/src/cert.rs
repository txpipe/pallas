use pallas_primitives::{alonzo, conway};

use crate::MultiEraCert;

impl<'b> MultiEraCert<'b> {
    pub fn as_alonzo(&self) -> Option<&alonzo::Certificate> {
        match self {
            MultiEraCert::AlonzoCompatible(x) => Some(x),
            _ => None,
        }
    }

    pub fn as_conway(&self) -> Option<&conway::Certificate> {
        match self {
            MultiEraCert::Conway(x) => Some(x),
            _ => None,
        }
    }
}
