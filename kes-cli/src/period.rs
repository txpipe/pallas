/// Period of signing key
use clap::Parser;
use pallas_crypto::kes::common::open_any;
use pallas_crypto::kes::summed_kes::Sum6Kes;
use pallas_crypto::kes::traits::KesSk;
use std::error::Error;
use std::io::Read;

#[derive(Debug, Parser)]
/// Arguments for period getter
pub struct Args {
    ///Signing key path used for determining a period
    #[arg(short, long, value_name = "FILE")]
    file: Option<String>,
}

/// Get period from 612 bytes signing key
pub fn run(args: Args) -> Result<(), Box<dyn Error>> {
    match args.file {
        None => {
            eprintln!("No stdin or file was provided to read a signing key");
        }
        Some(sk_source) => match open_any(&sk_source) {
            Err(err) => {
                eprintln!("Failed to open {sk_source}: {err}");
            }
            Ok(sk_handle) => {
                let mut buffer = [0; 1224];
                let mut handle = sk_handle.take(1224);
                handle.read_exact(&mut buffer)?;
                match hex::decode(buffer) {
                    Ok(bs) => {
                        let mut sk_bytes = [0u8; 612];
                        sk_bytes.copy_from_slice(&bs);
                        match Sum6Kes::from_bytes(&mut sk_bytes) {
                            Ok(sk) => {
                                let period = sk.get_period();
                                print!("{period}");
                            }
                            _ => {
                                eprintln!("Signing key expects 612 bytes");
                            }
                        };
                    }
                    Err(err) => {
                        eprintln!("Decode error of the signing key: {err}");
                    }
                }
            }
        },
    };

    Ok(())
}
