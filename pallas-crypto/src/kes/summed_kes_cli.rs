#![cfg(feature = "kes-cli")]

//! CLI implementation using Sum6Kes implementation of KES

use clap::{Parser, Subcommand};
use pallas_crypto::kes::cli;
use std::error::Error;

/// CLI commands available
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Generates 32 bytes secret seed
    GenerateSeed,

    /// Generates 612 bytes signing key of Sum6Kes
    GenerateSk,

    /// Derives 612 bytes signing key of Sum6Kes from 32 bytes seed
    DeriveSk(cli::derive_sk::Args),

    /// Derives 32 bytes public key from 612 bytes signing key
    DerivePk(cli::derive_pk::Args),

    /// Get period from 612 bytes signing key
    Period(cli::period::Args),

    /// Sign msg from stdin using 612 bytes signing key read from file
    Sign(cli::sign::Args),

    /// Verify, using public key read from file, that msg read from stdin was signed by the corresponding signing key and resulted in the signature included as argument
    Verify(cli::verify::Args),

    /// Increment period for a 612 bytes signing key which result in the updated signing key
    Update(cli::update::Args),
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
        Command::GenerateSeed => cli::generate_seed::run(),
        Command::GenerateSk => cli::generate_sk::run(),
        Command::DeriveSk(args) => cli::derive_sk::run(args),
        Command::DerivePk(args) => cli::derive_pk::run(args),
        Command::Period(args) => cli::period::run(args),
        Command::Sign(args) => cli::sign::run(args),
        Command::Verify(args) => cli::verify::run(args),
        Command::Update(args) => cli::update::run(args),
    }
}
