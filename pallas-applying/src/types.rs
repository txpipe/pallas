//! Base types used for validating transactions in each era.

use std::{borrow::Cow, collections::HashMap};

pub use pallas_traverse::{MultiEraInput, MultiEraOutput};


pub type UTxOs<'b> = HashMap<MultiEraInput<'b>, MultiEraOutput<'b>>;

#[derive(Debug, Clone)]	
pub struct ByronProtParams;

#[derive(Debug)]
#[non_exhaustive]
pub enum MultiEraProtParams<'b> {
	Byron(Box<Cow<'b, ByronProtParams>>)
}

#[derive(Debug)]
#[non_exhaustive]
pub enum ValidationError {
    ValidationError
}

pub type ValidationResult = Result<(), ValidationError>;
