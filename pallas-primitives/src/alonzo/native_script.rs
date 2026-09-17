//! The Shelley native script, unchanged through Conway. Everything about
//! its lifecycle lives in [`crate::native_script`].

use super::NativeScript;
use crate::native_script::impl_native_script;

impl_native_script!(NativeScript);

#[cfg(test)]
mod tests;
