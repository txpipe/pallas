use thiserror::Error;

use crate::hash::Hash;

pub mod epoch_nonce;
pub mod rolling_nonce;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Nonce error: {0}")]
    Nonce(String),
}

/// A trait for generating nonces.
pub trait NonceGenerator: Sized {
    fn finalize(&mut self) -> Result<Hash<32>, Error>;
}
