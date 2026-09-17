//! The Shelley native script plus this era's guard clause. Everything about
//! its lifecycle lives in [`crate::native_script`].

use super::NativeScript;
use crate::{StakeCredential, native_script::impl_native_script};

impl_native_script!(NativeScript { 6 => ScriptRequireGuard(StakeCredential) });

#[cfg(test)]
mod tests;
