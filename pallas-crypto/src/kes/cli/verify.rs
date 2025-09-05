#![cfg(feature = "kes-cli")]

/// Message verifying
use crate::kes::common::open_three;
use crate::kes::summed_kes::Sum6KesSig;
use crate::kes::traits::KesSig;
use crate::kes::PublicKey;
use clap::Parser;
use std::error::Error;
use std::io::Read;

#[derive(Debug, Parser)]
/// Arguments for message verification
pub struct Args {
    ///Public key filepath used for verification
    #[arg(short, long, value_name = "FILE")]
    file: Option<String>,

    ///Signature filepath to be verified against the stdin msg and using the public key
    #[arg(short, long, value_name = "FILE")]
    signature: Option<String>,

    /// Period for which verification is realized, ie., the period that was current when signature was made.
    #[arg(short, long, value_name = "INT")]
    period: Option<u32>,
}

/// Get period from 612 bytes signing key
pub fn run(args: Args) -> Result<(), Box<dyn Error>> {
    match (args.file, args.signature) {
        (Some(pk_source), Some(sig_source)) => match open_three(&pk_source, &sig_source) {
            Err(err) => {
                eprintln!("{pk_source},{sig_source}: {err}");
            }
            Ok((mut msg_handle, pk_handle, sig_handle)) => {
                let mut pk_buffer = [0; 64];
                let mut pk_handle_ok = pk_handle.take(64);
                pk_handle_ok.read_exact(&mut pk_buffer)?;
                let mut sig_buffer = [0; 896];
                let mut sig_handle_ok = sig_handle.take(896);
                sig_handle_ok.read_exact(&mut sig_buffer)?;
                match hex::decode(pk_buffer) {
                    Ok(pk_bytes) => {
                        let mut pk_array = [0u8; 32];
                        pk_array.copy_from_slice(&pk_bytes);
                        let pk = PublicKey::from_bytes(&pk_array)?;
                        let msg = msg_handle.fill_buf()?;
                        let mut sig_array = [0u8; 448];
                        match hex::decode(sig_buffer) {
                            Ok(sig_bytes) => {
                                sig_array.copy_from_slice(&sig_bytes);
                                let sig = Sum6KesSig::from_bytes(&sig_array)?;
                                match args.period {
                                    None => {
                                        eprintln!("A period value is missing");
                                    }
                                    Some(p) => match sig.verify(p, &pk, msg) {
                                        Ok(()) => {
                                            println!("OK");
                                        }
                                        _ => {
                                            println!("Fail");
                                        }
                                    },
                                }
                            }
                            Err(err) => {
                                eprintln!("Decode error of the signature: {err}");
                            }
                        }
                    }
                    Err(err) => {
                        eprintln!("Decode error of the secret key: {err}");
                    }
                }
            }
        },
        _ => {
            eprintln!("Both public key and signature must be provided via filepaths");
        }
    };

    Ok(())
}
