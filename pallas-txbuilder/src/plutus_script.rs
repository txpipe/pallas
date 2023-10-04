use std::fmt::Debug;

use hex::FromHexError;
use pallas_codec::utils::Bytes;
use pallas_primitives::babbage::{PlutusV1Script, PlutusV2Script};

pub(crate) type V1Script = PlutusScriptBuilder<V1>;
pub(crate) type V2Script = PlutusScriptBuilder<V2>;

#[derive(Debug, Clone, thiserror::Error, PartialEq)]
pub enum PlutusScriptError {
    #[error("Invalid hex value: {0}")]
    InvalidHexValue(#[from] FromHexError),
}

pub trait BuildPlutusScript {
    type Output;

    fn build(self) -> Self::Output;
}

#[derive(Debug, Clone, Default)]
pub struct PlutusScriptBuilder<V> {
    inner: V,
}

impl<V: BuildPlutusScript> PlutusScriptBuilder<V> {
    pub fn build(self) -> V::Output {
        self.inner.build()
    }
}

#[derive(Debug, Clone, Default)]
pub struct V1(Bytes);

impl BuildPlutusScript for V1 {
    type Output = PlutusV1Script;

    fn build(self) -> Self::Output {
        PlutusV1Script(self.0.into())
    }
}

impl PlutusScriptBuilder<V1> {
    pub fn from_hex(mut self, data: impl AsRef<str>) -> Result<Self, PlutusScriptError> {
        self.inner.0 = hex::decode(data.as_ref())?.into();
        Ok(self)
    }

    pub fn from_bytes(mut self, data: impl Into<Bytes>) -> Self {
        self.inner.0 = data.into();
        self
    }
}

#[derive(Debug, Clone, Default)]
pub struct V2(Bytes);

impl BuildPlutusScript for V2 {
    type Output = PlutusV2Script;

    fn build(self) -> Self::Output {
        PlutusV2Script(self.0.into())
    }
}

impl PlutusScriptBuilder<V2> {
    pub fn from_hex(mut self, data: impl AsRef<str>) -> Result<Self, PlutusScriptError> {
        self.inner.0 = hex::decode(data.as_ref())?.into();
        Ok(self)
    }

    pub fn from_bytes(mut self, data: impl Into<Bytes>) -> Self {
        self.inner.0 = data.into();
        self
    }
}

pub struct PlutusScript;

impl PlutusScript {
    pub fn v1() -> PlutusScriptBuilder<V1> {
        PlutusScriptBuilder::default()
    }

    pub fn v2() -> PlutusScriptBuilder<V2> {
        PlutusScriptBuilder::default()
    }
}
