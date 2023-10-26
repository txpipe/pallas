use std::borrow::Cow;

use pallas_primitives::{alonzo, babbage};

use crate::MultiEraUpdate;

impl<'b> MultiEraUpdate<'b> {
    pub fn from_alonzo_compatible(update: &'b alonzo::Update) -> Self {
        Self::AlonzoCompatible(Box::new(Cow::Borrowed(update)))
    }

    pub fn from_babbage(update: &'b babbage::Update) -> Self {
        Self::Babbage(Box::new(Cow::Borrowed(update)))
    }

    pub fn as_alonzo(&self) -> Option<&alonzo::Update> {
        match self {
            Self::AlonzoCompatible(x) => Some(x),
            _ => None,
        }
    }

    pub fn as_babbage(&self) -> Option<&babbage::Update> {
        match self {
            Self::Babbage(x) => Some(x),
            _ => None,
        }
    }
}
