use pallas_codec::minicbor;
use pallas_crypto::hash::Hash;
use pallas_primitives::{alonzo, byron, ToHash};
use std::borrow::Cow;

use crate::{MultiEraCert, MultiEraOutput, MultiEraTx};

impl<'b> MultiEraTx<'b> {
    pub fn from_byron(tx: &'b byron::MintedTxPayload<'b>) -> Self {
        Self::Byron(Cow::Borrowed(tx))
    }

    pub fn from_alonzo_compatible(tx: &'b alonzo::MintedTx<'b>) -> Self {
        Self::AlonzoCompatible(Cow::Borrowed(tx))
    }

    pub fn encode(&self) -> Result<Vec<u8>, minicbor::encode::Error<std::io::Error>> {
        match self {
            MultiEraTx::AlonzoCompatible(x) => minicbor::to_vec(x),
            MultiEraTx::Byron(x) => minicbor::to_vec(x),
        }
    }

    pub fn hash(&self) -> Hash<32> {
        match self {
            MultiEraTx::AlonzoCompatible(x) => x.transaction_body.to_hash(),
            MultiEraTx::Byron(x) => x.transaction.to_hash(),
        }
    }

    pub fn outputs(&self) -> Vec<MultiEraOutput> {
        match self {
            MultiEraTx::AlonzoCompatible(x) => x
                .transaction_body
                .outputs
                .iter()
                .map(MultiEraOutput::from_alonzo_compatible)
                .collect(),
            MultiEraTx::Byron(x) => x
                .transaction
                .outputs
                .iter()
                .map(MultiEraOutput::from_byron)
                .collect(),
        }
    }

    pub fn certs(&self) -> Vec<MultiEraCert> {
        match self {
            MultiEraTx::AlonzoCompatible(x) => x
                .transaction_body
                .certificates
                .iter()
                .flat_map(|c| c.iter())
                .map(|c| MultiEraCert::AlonzoCompatible(Cow::Borrowed(c)))
                .collect(),
            MultiEraTx::Byron(_) => vec![],
        }
    }
}
