use std::collections::{HashMap, HashSet};

use pallas_primitives::Hash;
use pallas_traverse::{Era, MultiEraInput, MultiEraOutput, MultiEraUpdate};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod phase_one;
pub mod phase_two;

pub type TxHash = Hash<32>;
pub type TxoIdx = u32;
pub type BlockSlot = u64;
pub type BlockHash = Hash<32>;
pub type TxOrder = usize;

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct EraCbor(pub Era, pub Vec<u8>);

impl From<(Era, Vec<u8>)> for EraCbor {
    fn from(value: (Era, Vec<u8>)) -> Self {
        Self(value.0, value.1)
    }
}

impl From<EraCbor> for (Era, Vec<u8>) {
    fn from(value: EraCbor) -> Self {
        (value.0, value.1)
    }
}

impl From<MultiEraOutput<'_>> for EraCbor {
    fn from(value: MultiEraOutput<'_>) -> Self {
        EraCbor(value.era(), value.encode())
    }
}

impl<'a> TryFrom<&'a EraCbor> for MultiEraOutput<'a> {
    type Error = pallas_codec::minicbor::decode::Error;

    fn try_from(value: &'a EraCbor) -> Result<Self, Self::Error> {
        MultiEraOutput::decode(value.0, &value.1)
    }
}

impl TryFrom<EraCbor> for MultiEraUpdate<'_> {
    type Error = pallas_codec::minicbor::decode::Error;

    fn try_from(value: EraCbor) -> Result<Self, Self::Error> {
        MultiEraUpdate::decode_for_era(value.0, &value.1)
    }
}

pub type UtxoBody<'a> = MultiEraOutput<'a>;

#[derive(Debug, Eq, PartialEq, Hash, Clone, Serialize, Deserialize)]
pub struct TxoRef(pub TxHash, pub TxoIdx);

impl From<(TxHash, TxoIdx)> for TxoRef {
    fn from(value: (TxHash, TxoIdx)) -> Self {
        Self(value.0, value.1)
    }
}

impl From<TxoRef> for (TxHash, TxoIdx) {
    fn from(value: TxoRef) -> Self {
        (value.0, value.1)
    }
}

impl From<&MultiEraInput<'_>> for TxoRef {
    fn from(value: &MultiEraInput<'_>) -> Self {
        TxoRef(*value.hash(), value.index() as u32)
    }
}

#[derive(Debug, Eq, PartialEq, Hash)]
pub struct ChainPoint(pub BlockSlot, pub BlockHash);

pub type UtxoMap = HashMap<TxoRef, EraCbor>;

pub type UtxoSet = HashSet<TxoRef>;

#[derive(Debug, Error)]
pub enum BrokenInvariant {
    #[error("missing utxo {0:?}")]
    MissingUtxo(TxoRef),
}