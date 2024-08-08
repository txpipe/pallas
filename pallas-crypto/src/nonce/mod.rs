use thiserror::Error;

pub mod rolling_nonce;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Nonce error: {0}")]
    Nonce(String),
}

/// A trait for generating nonces.
pub trait NonceGenerator: Sized {
    fn finalize(&mut self) -> Result<Self, Error>;
}
