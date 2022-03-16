/// Shared re-export of minicbor lib across all Pallas
pub use minicbor;

/// Round-trip friendly common helper structs
pub mod utils;

pub trait Fragment: Sized + for<'b> minicbor::Decode<'b> + minicbor::Encode {}

impl<T> Fragment for T where T: for<'b> minicbor::Decode<'b> + minicbor::Encode + Sized {}
