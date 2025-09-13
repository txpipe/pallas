//! CLI implementation using Sum6Kes implementation of KES

use clap::{Parser, Subcommand};
use std::error::Error;

/// Public key derivation
pub mod derive_pk;

/// Signing key derivation
pub mod derive_sk;

/// Seed generation
pub mod generate_seed;

/// Signing key generation
pub mod generate_sk;

/// Period of signing key
pub mod period;

/// Message signing
pub mod sign;

/// Signing key updating
pub mod update;

/// Message verifying
pub mod verify;

/// CLI tests
mod tests;

/// CLI commands available
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Generates 32 bytes secret seed
    GenerateSeed,

    /// Generates 612 bytes signing key of Sum6Kes
    GenerateSk,

    /// Derives 612 bytes signing key of Sum6Kes from 32 bytes seed
    DeriveSk(derive_sk::Args),

    /// Derives 32 bytes public key from 612 bytes signing key
    DerivePk(derive_pk::Args),

    /// Get period from 612 bytes signing key
    Period(period::Args),

    /// Sign msg from stdin using 612 bytes signing key read from file
    Sign(sign::Args),

    /// Verify, using public key read from file, that msg read from stdin was
    /// signed by the corresponding signing key and resulted in the signature
    /// included as argument
    Verify(verify::Args),

    /// Increment period for a 612 bytes signing key which result in the updated
    /// signing key
    Update(update::Args),
}

#[derive(Debug, Parser)]
#[clap(name = "kes-cli")]
#[clap(bin_name = "kes")]
#[clap(author = "HAL Team <hal@cardanofoundation.org>")]
#[clap(version=env!("CARGO_PKG_VERSION"))]
#[clap(about = "Cardano compliant Rust KES library using Sum6")]
#[clap(about, long_about = None)]
/// Cli command data type
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

/// main function of kes cli binary
pub fn main() -> Result<(), Box<dyn Error>> {
    let args = Cli::parse();

    match args.command {
        Command::GenerateSeed => generate_seed::run(),
        Command::GenerateSk => generate_sk::run(),
        Command::DeriveSk(args) => derive_sk::run(args),
        Command::DerivePk(args) => derive_pk::run(args),
        Command::Period(args) => period::run(args),
        Command::Sign(args) => sign::run(args),
        Command::Verify(args) => verify::run(args),
        Command::Update(args) => update::run(args),
    }
}
