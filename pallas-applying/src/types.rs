//! Base types used for validating transactions in each era.

pub mod environment;
pub mod validation;

pub use environment::*;
pub use pallas_traverse::{MultiEraInput, MultiEraOutput};
use std::collections::HashMap;
pub use validation::*;

pub type UTxOs<'b> = HashMap<MultiEraInput<'b>, MultiEraOutput<'b>>;
