#![cfg(feature = "vrf")]

use pallas_crypto::vrf::{keypair_from_seed, verify_draft03, VrfDraft03};

fn hex_decode(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

#[test]
fn draft03_ietf_vector_10() {
    // https://datatracker.ietf.org/doc/html/draft-irtf-cfrg-vrf-03#appendix-A.1
    let sk_seed = hex_decode("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60");
    let expected_pk =
        hex_decode("d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a");
    let expected_pi = hex_decode(concat!(
        "b6b4699f87d56126c9117a7da55bd0085246f4c56dbc95d20172612e9d38e8d7",
        "ca65e573a126ed88d4e30a46f80a666854d675cf3ba81de0de043c3774f06156",
        "0f55edc256a787afe701677c0f602900",
    ));
    let expected_beta = hex_decode(concat!(
        "5b49b554d05c0cd5a5325376b3387de59d924fd1e13ded44648ab33c21349a60",
        "3f25b84ec5ed887995b33da5e3bfcb87cd2f64521c4c62cf825cffabbe5d31cc",
    ));

    let seed: [u8; 32] = sk_seed.try_into().unwrap();
    let (sk, pk) = VrfDraft03::keypair_from_seed(&seed);

    assert_eq!(pk.as_slice(), expected_pk.as_slice());

    let proof = VrfDraft03::prove(&sk, &[]).expect("prove");
    assert_eq!(proof.as_slice(), expected_pi.as_slice());

    let beta = VrfDraft03::verify(&pk, &proof, &[]).expect("verify");
    assert_eq!(beta.as_slice(), expected_beta.as_slice());

    let beta2 = VrfDraft03::proof_to_hash(&proof).expect("proof_to_hash");
    assert_eq!(beta, beta2);
}

#[test]
fn draft03_ietf_vector_11() {
    let sk_seed = hex_decode("4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb");
    let expected_pk =
        hex_decode("3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c");
    let alpha = hex_decode("72");
    let expected_pi = hex_decode("ae5b66bdf04b4c010bfe32b2fc126ead2107b697634f6f7337b9bff8785ee111200095ece87dde4dbe87343f6df3b107d91798c8a7eb1245d3bb9c5aafb093358c13e6ae1111a55717e895fd15f99f07");
    let expected_beta = hex_decode(concat!(
        "94f4487e1b2fec954309ef1289ecb2e15043a2461ecc7b2ae7d4470607ef82eb",
        "1cfa97d84991fe4a7bfdfd715606bc27e2967a6c557cfb5875879b671740b7d8",
    ));

    let seed: [u8; 32] = sk_seed.try_into().unwrap();
    let (sk, pk) = VrfDraft03::keypair_from_seed(&seed);

    assert_eq!(pk.as_slice(), expected_pk.as_slice());

    let proof = VrfDraft03::prove(&sk, &alpha).expect("prove");
    assert_eq!(
        proof.as_slice().len(),
        pallas_crypto::vrf::DRAFT03_PROOF_SIZE
    );
    assert_eq!(proof.as_slice(), expected_pi.as_slice());

    let beta = VrfDraft03::verify(&pk, &proof, &alpha).expect("verify");
    assert_eq!(beta.as_slice().len(), pallas_crypto::vrf::OUTPUT_SIZE);
    assert_eq!(beta.as_slice(), expected_beta.as_slice());
}

#[test]
fn draft03_ietf_vector_12() {
    let sk_seed = hex_decode("c5aa8df43f9f837bedb7442f31dcb7b166d38535076f094b85ce3a2e0b4458f7");
    let expected_pk =
        hex_decode("fc51cd8e6218a1a38da47ed00230f0580816ed13ba3303ac5deb911548908025");
    let alpha = hex_decode("af82");
    let expected_pi = hex_decode(concat!(
        "dfa2cba34b611cc8c833a6ea83b8eb1bb5e2ef2dd1b0c481bc42ff36ae7847f6",
        "ab52b976cfd5def172fa412defde270c8b8bdfbaae1c7ece17d9833b1bcf3106",
        "4fff78ef493f820055b561ece45e1009",
    ));
    let expected_beta = hex_decode(concat!(
        "2031837f582cd17a9af9e0c7ef5a6540e3453ed894b62c293686ca3c1e319dde",
        "9d0aa489a4b59a9594fc2328bc3deff3c8a0929a369a72b1180a596e016b5ded",
    ));

    let seed: [u8; 32] = sk_seed.try_into().unwrap();
    let (sk, pk) = VrfDraft03::keypair_from_seed(&seed);

    assert_eq!(pk.as_slice(), expected_pk.as_slice());

    let proof = VrfDraft03::prove(&sk, &alpha).expect("prove");
    assert_eq!(proof.as_slice(), expected_pi.as_slice());

    let beta = VrfDraft03::verify(&pk, &proof, &alpha).expect("verify");
    assert_eq!(beta.as_slice(), expected_beta.as_slice());
}

#[test]
fn draft03_cardano_messages() {
    let seed = [0x42u8; 32];
    let kp = keypair_from_seed(&seed);
    let messages: &[&[u8]] = &[
        b"Block header hash",
        b"Epoch nonce derivation",
        b"Leader election slot 12345",
        &[0u8; 64],
        &[],
    ];

    for msg in messages {
        let (proof, out1) = pallas_crypto::vrf::prove_draft03(&kp.signing_key, msg).expect("prove");
        let out2 = verify_draft03(&kp.verification_key, &proof, msg).expect("verify");
        assert_eq!(out1.as_bytes(), out2.as_bytes());
    }
}
