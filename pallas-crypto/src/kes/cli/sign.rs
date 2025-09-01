#![cfg(feature = "kes-cli")]

/// Message signing
use crate::kes::common::open_both;
use crate::kes::summed_kes::Sum6Kes;
use crate::kes::traits::KesSk;
use clap::Parser;
use std::error::Error;
use std::io::Read;

#[derive(Debug, Parser)]
/// Arguments for signing
pub struct Args {
    ///Signing key path used for determining a period
    #[arg(short, long, value_name = "FILE")]
    file: Option<String>,
}

/// Get period from 612 bytes signing key
pub fn run(args: Args) -> Result<(), Box<dyn Error>> {
    match args.file {
        None => {
            eprintln!("A secret key must be provided in a file");
        }
        Some(sk_source) => match open_both(&sk_source) {
            Err(err) => {
                eprintln!("{sk_source}: {err}");
            }
            Ok((mut msg_handle, sk_handle)) => {
                let mut buffer = [0; 1224];
                let mut handle = sk_handle.take(1224);
                handle.read_exact(&mut buffer)?;
                match hex::decode(buffer) {
                    Ok(bs) => {
                        let mut sk_bytes = [0u8; 612];
                        sk_bytes.copy_from_slice(&bs);
                        match Sum6Kes::from_bytes(&mut sk_bytes) {
                            Ok(sk) => {
                                let msg = msg_handle.fill_buf()?;
                                let signature = sk.sign(msg);
                                print!("{}", hex::encode(signature.to_bytes()));
                            }
                            _ => {
                                eprintln!("Signing key expects 612 bytes");
                            }
                        };
                    }
                    Err(err) => {
                        eprintln!("Decode error of the secret key: {err}");
                    }
                }
            }
        },
    };

    Ok(())
}
