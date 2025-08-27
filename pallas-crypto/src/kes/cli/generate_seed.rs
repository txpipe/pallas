#![cfg(feature = "kes_cli")]

/// Seed generation
use crate::kes::common::generate_crypto_secure_seed;
use std::error::Error;

/// Generates 32 bytes secret seed using cryptographic secure generator
pub fn run() -> Result<(), Box<dyn Error>> {
    let mut seed_bytes = [0u8; 32];
    generate_crypto_secure_seed(&mut seed_bytes);
    print!("{}", hex::encode(seed_bytes));

    Ok(())
}
