//! Base types used for validating transactions in each era.

use std::{borrow::Cow, collections::HashMap};

pub use pallas_traverse::{MultiEraInput, MultiEraOutput};

pub type UTxOs<'b> = HashMap<MultiEraInput<'b>, MultiEraOutput<'b>>;

// TODO: add a field for each protocol parameter in the Byron era.
#[derive(Debug, Clone)]
pub struct ByronProtParams;

// TODO: add variants for the other eras.
#[derive(Debug)]
#[non_exhaustive]
pub enum MultiEraProtParams<'b> {
    Byron(Box<Cow<'b, ByronProtParams>>),
}

// TODO: replace this generic variant with validation-rule-specific ones.
#[derive(Debug)]
#[non_exhaustive]
pub enum ValidationError {
    InputMissingInUTxO,
    TxInsEmpty,
    TxOutsEmpty,
}

pub type ValidationResult = Result<(), ValidationError>;
