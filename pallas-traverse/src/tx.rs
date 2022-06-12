use pallas_primitives::{alonzo, byron};

use crate::MultiEraTx;

impl<'b> MultiEraTx<'b> {
    pub fn from_byron(tx: byron::MintedTxPayload<'b>) -> Self {
        Self::Byron(Box::new(tx))
    }

    pub fn from_alonzo_compatible(tx: alonzo::MintedTx<'b>) -> Self {
        Self::AlonzoCompatible(Box::new(tx))
    }
}
