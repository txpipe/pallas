#![cfg(feature = "kes")]

use pallas_crypto::kes::summed_kes::Sum6Kes;
use pallas_crypto::kes::traits::{KesSig, KesSk};

#[test]
fn sum6_total_periods_and_roundtrip() {
    let mut key_bytes = [0u8; Sum6Kes::SIZE + 4];
    let mut seed = [0x46u8; 32];
    let (mut sk, vk) = Sum6Kes::keygen(&mut key_bytes, &mut seed);

    // sign at period 0
    let msg0 = b"Sum6 period 0";
    let sig0 = sk.sign(msg0);
    sig0.verify(sk.get_period(), &vk, msg0).expect("verify p0");

    // evolve to final period and sign
    for _ in 0..63 {
        sk.update().expect("evolve");
    }
    assert_eq!(sk.get_period(), 63);

    let msg_last = b"Sum6 period 63";
    let sig_last = sk.sign(msg_last);
    sig_last.verify(63, &vk, msg_last).expect("verify p63");

    // adjacent periods should fail
    assert!(sig_last.verify(62, &vk, msg_last).is_err());
}

#[test]
fn sum6_verification_key_stability() {
    let mut key_bytes = [0u8; Sum6Kes::SIZE + 4];
    let mut seed = [0x99u8; 32];
    let (mut sk, vk0) = Sum6Kes::keygen(&mut key_bytes, &mut seed);
    let vk0_bytes = vk0.as_bytes().to_vec();

    for period in 0..10 {
        let vk_bytes = sk.to_pk().as_bytes().to_vec();
        assert_eq!(
            vk0_bytes, vk_bytes,
            "vkey must stay stable at period {period}"
        );
        if period < 9 {
            sk.update().expect("evolve");
        }
    }
}

#[test]
fn sum6_deterministic_from_seed() {
    let seed = [0xCCu8; 32];

    let mut key_bytes1 = [0u8; Sum6Kes::SIZE + 4];
    let mut seed1 = seed.clone();
    let (sk1, vk1) = Sum6Kes::keygen(&mut key_bytes1, &mut seed1);

    let mut key_bytes2 = [0u8; Sum6Kes::SIZE + 4];
    let mut seed2 = seed.clone();
    let (sk2, vk2) = Sum6Kes::keygen(&mut key_bytes2, &mut seed2);

    let m = b"deterministic";
    let sig1 = sk1.sign(m);
    let sig2 = sk2.sign(m);

    assert_eq!(vk1.as_bytes(), vk2.as_bytes());
    assert_eq!(sig1.to_bytes(), sig2.to_bytes());
}
