#![cfg(feature = "ed25519-dalek")]

use pallas_crypto::key::ed25519::{PublicKey, Signature};

const VECTORS: &str = include_str!("data/ed25519_libsodium_verdicts.txt");

#[test]
fn verify_strict_agrees_with_libsodium() {
    let mut checked = 0;
    let mut accepted = 0;
    let mut disagreements = Vec::new();

    for line in VECTORS.lines().filter(|l| !l.starts_with('#')) {
        let fields: Vec<&str> = line.split_whitespace().collect();
        let [label, message, key, signature, verdict] = fields[..] else {
            panic!("malformed vector line: {line}");
        };

        let message = match message {
            "-" => Vec::new(),
            hex => hex::decode(hex).unwrap(),
        };
        let key: PublicKey = key.parse().unwrap();
        let signature: Signature = signature.parse().unwrap();
        let expected = match verdict {
            "accept" => true,
            "reject" => false,
            other => panic!("unknown verdict {other} in {label}"),
        };

        if key.verify_strict(&message, &signature) != expected {
            disagreements.push(label);
        }
        checked += 1;
        accepted += usize::from(expected);
    }

    // 914 C2SP CCTV vectors and 12 speccheck cases, of which libsodium accepts 44.
    assert_eq!(checked, 926, "unexpected number of vectors");
    assert_eq!(accepted, 44, "unexpected number of accepted vectors");
    assert!(disagreements.is_empty(), "disagreements: {disagreements:?}");
}
