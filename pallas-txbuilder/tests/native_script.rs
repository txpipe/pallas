use std::time::Instant;

use assert_matches::assert_matches;
use pallas_primitives::babbage::NativeScript as ExternalNativeScript;
use pallas_txbuilder::prelude::*;
use rand::{distributions::Standard, prelude::Distribution, rngs::OsRng, Rng, RngCore};

fn gen<T>() -> T
where
    Standard: Distribution<T>,
{
    OsRng.gen()
}

fn gen_hash<const N: usize>() -> [u8; N] {
    let mut buf: [u8; N] = [0; N];
    OsRng.fill_bytes(&mut buf);

    buf
}

#[test]
fn test_build_pubkey_script() {
    let pubkey = gen_hash();
    let output = NativeScript::pubkey(pubkey);

    assert_eq!(
        output.build(),
        ExternalNativeScript::ScriptPubkey(pubkey.into())
    );
}

#[test]
fn test_build_all_script() {
    let scripts = vec![
        NativeScript::pubkey(gen_hash()),
        NativeScript::pubkey(gen_hash()),
        NativeScript::pubkey(gen_hash()),
    ];

    let mut output = NativeScript::all();

    for script in scripts.clone().into_iter() {
        output = output.add(script);
    }

    assert_eq!(
        output.build(),
        ExternalNativeScript::ScriptAll(scripts.into_iter().map(|x| x.build()).collect())
    );
}

#[test]
fn test_build_any_script() {
    let scripts = vec![
        NativeScript::pubkey(gen_hash()),
        NativeScript::pubkey(gen_hash()),
        NativeScript::pubkey(gen_hash()),
    ];

    let mut output = NativeScript::any();

    for script in scripts.clone().into_iter() {
        output = output.add(script);
    }

    assert_eq!(
        output.build(),
        ExternalNativeScript::ScriptAny(scripts.into_iter().map(|x| x.build()).collect())
    );
}

#[test]
fn test_build_n_of_k_script() {
    let scripts = vec![
        NativeScript::pubkey(gen_hash()),
        NativeScript::pubkey(gen_hash()),
        NativeScript::pubkey(gen_hash()),
    ];
    let n = scripts.len() - 1;

    let mut output = NativeScript::at_least_n(n as u32);

    for script in scripts.clone().into_iter() {
        output = output.add(script);
    }

    assert_eq!(
        output.build(),
        ExternalNativeScript::ScriptNOfK(
            n as u32,
            scripts.into_iter().map(|x| x.build()).collect()
        )
    );
}

#[test]
fn test_build_invalid_before_slot_script() {
    let slot = gen();
    let output = NativeScript::invalid_before_slot(slot);

    assert_eq!(output.build(), ExternalNativeScript::InvalidBefore(slot));
}

#[test]
fn test_build_invalid_before_timestamp_script() -> Result<(), NativeScriptError> {
    let timestamp = Instant::now();
    let output = NativeScript::invalid_before(NetworkParams::mainnet(), timestamp)?;

    assert_matches!(output.build(), ExternalNativeScript::InvalidBefore(_));

    Ok(())
}

#[test]
fn test_build_invalid_after_slot_script() {
    let slot = gen();
    let output = NativeScript::invalid_after_slot(slot);

    assert_eq!(output.build(), ExternalNativeScript::InvalidHereafter(slot));
}

#[test]
fn test_build_invalid_after_timestamp_script() -> Result<(), NativeScriptError> {
    let timestamp = Instant::now();
    let output = NativeScript::invalid_after(NetworkParams::mainnet(), timestamp)?;

    assert_matches!(output.build(), ExternalNativeScript::InvalidHereafter(_));

    Ok(())
}
