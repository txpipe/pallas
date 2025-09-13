/// Signing key derivation
use clap::Parser;
use pallas_crypto::kes::common::open_any;
use pallas_crypto::kes::summed_kes::Sum6Kes;
use pallas_crypto::kes::traits::KesSk;
use std::error::Error;
use std::io::Read;

#[derive(Debug, Parser)]
/// Arguments for signing key derivation
pub struct Args {
    ///Seed path used for derivation of a signing key
    #[arg(short, long, value_name = "FILE")]
    file: Option<String>,
}

/// Derives 612 bytes signing key of Sum6Kes from 32 bytes seed
pub fn run(args: Args) -> Result<(), Box<dyn Error>> {
    match args.file {
        None => {
            eprintln!("No stdin or file was provided to read a secret seed");
        }
        Some(seed_source) => match open_any(&seed_source) {
            Err(err) => {
                eprintln!("Failed to open {seed_source}: {err}");
            }
            Ok(seed_handle) => {
                let mut buffer = [0; 64];
                let mut handle = seed_handle.take(64);
                handle.read_exact(&mut buffer)?;
                match hex::decode(buffer) {
                    Ok(bs) => {
                        let mut seed_bytes = [0u8; 32];
                        seed_bytes.copy_from_slice(&bs);
                        let mut key_bytes = [0u8; Sum6Kes::SIZE + 4];
                        let (sk, _pk) = Sum6Kes::keygen(&mut key_bytes, &mut seed_bytes);
                        print!("{}", hex::encode(sk.as_bytes()));
                    }
                    Err(err) => {
                        eprintln!("Decode error of the secret seed: {err}");
                    }
                }
            }
        },
    }

    Ok(())
}
