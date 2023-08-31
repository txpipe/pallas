use std::time::Instant;

use pallas_primitives::babbage::{AddrKeyhash, NativeScript as ExternalNativeScript};

use crate::NetworkParams;

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum NativeScriptError {
    #[error("Invalid timestamp")]
    InvalidTimestamp,
}

pub trait BuildNativeScript
where
    Self: Clone,
{
    fn build(self) -> ExternalNativeScript;
}

#[derive(Clone, Default)]
pub struct NativeScriptBuilder<K: Clone> {
    inner: K,
}

impl<K: BuildNativeScript> NativeScriptBuilder<K> {
    pub fn build(self) -> ExternalNativeScript {
        self.inner.build()
    }
}

#[derive(Clone)]
pub struct PubKey(AddrKeyhash);

impl BuildNativeScript for PubKey {
    fn build(self) -> ExternalNativeScript {
        ExternalNativeScript::ScriptPubkey(self.0)
    }
}

#[derive(Clone, Default)]
pub struct All(Vec<ExternalNativeScript>);

impl NativeScriptBuilder<All> {
    pub fn add<T: BuildNativeScript>(mut self, script: NativeScriptBuilder<T>) -> Self {
        self.inner.0.push(script.build());
        self
    }
}

impl BuildNativeScript for All {
    fn build(self) -> ExternalNativeScript {
        ExternalNativeScript::ScriptAll(self.0)
    }
}

#[derive(Clone, Default)]
pub struct Any(Vec<ExternalNativeScript>);

impl NativeScriptBuilder<Any> {
    pub fn add<T: BuildNativeScript>(mut self, script: NativeScriptBuilder<T>) -> Self {
        self.inner.0.push(script.build());
        self
    }
}

impl BuildNativeScript for Any {
    fn build(self) -> ExternalNativeScript {
        ExternalNativeScript::ScriptAny(self.0)
    }
}

#[derive(Clone, Default)]
pub struct AtLeastN(u32, Vec<ExternalNativeScript>);

impl NativeScriptBuilder<AtLeastN> {
    pub fn add<T: BuildNativeScript>(mut self, script: NativeScriptBuilder<T>) -> Self {
        self.inner.1.push(script.build());
        self
    }
}

impl BuildNativeScript for AtLeastN {
    fn build(self) -> ExternalNativeScript {
        ExternalNativeScript::ScriptNOfK(self.0, self.1)
    }
}

#[derive(Clone, Default)]
pub struct InvalidBefore(u64);

impl BuildNativeScript for InvalidBefore {
    fn build(self) -> ExternalNativeScript {
        ExternalNativeScript::InvalidBefore(self.0)
    }
}

#[derive(Clone, Default)]
pub struct InvalidAfter(u64);

impl BuildNativeScript for InvalidAfter {
    fn build(self) -> ExternalNativeScript {
        ExternalNativeScript::InvalidHereafter(self.0)
    }
}

pub struct NativeScript;

impl NativeScript {
    pub fn pubkey(addr: impl Into<AddrKeyhash>) -> NativeScriptBuilder<PubKey> {
        NativeScriptBuilder {
            inner: PubKey(addr.into()),
        }
    }

    pub fn all() -> NativeScriptBuilder<All> {
        NativeScriptBuilder {
            inner: All::default(),
        }
    }

    pub fn any() -> NativeScriptBuilder<Any> {
        NativeScriptBuilder {
            inner: Any::default(),
        }
    }

    pub fn at_least_n(n: impl Into<u32>) -> NativeScriptBuilder<AtLeastN> {
        NativeScriptBuilder {
            inner: AtLeastN(n.into(), vec![]),
        }
    }

    pub fn invalid_before(
        network: NetworkParams,
        timestamp: Instant,
    ) -> Result<NativeScriptBuilder<InvalidBefore>, NativeScriptError> {
        let slot = network
            .timestamp_to_slot(timestamp)
            .ok_or(NativeScriptError::InvalidTimestamp)?;

        Ok(Self::invalid_before_slot(slot))
    }

    pub fn invalid_before_slot(slot: impl Into<u64>) -> NativeScriptBuilder<InvalidBefore> {
        NativeScriptBuilder {
            inner: InvalidBefore(slot.into()),
        }
    }

    pub fn invalid_after(
        network: NetworkParams,
        timestamp: Instant,
    ) -> Result<NativeScriptBuilder<InvalidAfter>, NativeScriptError> {
        let slot = network
            .timestamp_to_slot(timestamp)
            .ok_or(NativeScriptError::InvalidTimestamp)?;

        Ok(Self::invalid_after_slot(slot))
    }

    pub fn invalid_after_slot(slot: impl Into<u64>) -> NativeScriptBuilder<InvalidAfter> {
        NativeScriptBuilder {
            inner: InvalidAfter(slot.into()),
        }
    }
}
