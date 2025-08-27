#![cfg(feature = "kes_cli")]

/// Signing key generation
use crate::kes::common::generate_crypto_secure_seed;
use crate::kes::summed_kes::Sum6Kes;
use crate::kes::traits::KesSk;
use std::error::Error;

/// Generates 612 bytes signing key of Sum6Kes using cryptographic secure generator
pub fn run() -> Result<(), Box<dyn Error>> {
    let mut key_bytes = [0u8; Sum6Kes::SIZE + 4];
    let mut seed_bytes = [0u8; 32];
    generate_crypto_secure_seed(&mut seed_bytes);
    let (sk, _pk) = Sum6Kes::keygen(&mut key_bytes, &mut seed_bytes);
    let mut sk_bytes = [0u8; Sum6Kes::SIZE + 4];
    sk_bytes.copy_from_slice(sk.as_bytes());
    print!("{}", hex::encode(sk_bytes));

    Ok(())
}
