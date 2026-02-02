#[cfg(test)]
mod test {

    use crate::kes::common::PublicKey;
    use crate::kes::errors::Error;
    use crate::kes::summed_kes::*;
    use crate::kes::traits::{KesSig, KesSk};

    use proptest::prelude::*;

    fn secret_public_key_bytes() -> impl Strategy<Value = ([u8; Sum6Kes::SIZE + 4], PublicKey)> {
        proptest::string::bytes_regex("[[:ascii:]]{32}")
            .unwrap()
            .prop_map(|vec| {
                let mut key_bytes = [0u8; Sum6Kes::SIZE + 4];
                let mut seed_bytes = [0u8; 32];
                seed_bytes.copy_from_slice(&vec);
                let (sk, pk) = Sum6Kes::keygen(&mut key_bytes, &mut seed_bytes);
                let mut sk_bytes = [0u8; Sum6Kes::SIZE + 4];
                sk_bytes.copy_from_slice(sk.as_bytes());
                (sk_bytes, pk)
            })
    }

    fn payload() -> impl Strategy<Value = Vec<u8>> {
        proptest::string::bytes_regex("[[:ascii:]]{0,254}").unwrap()
    }

    proptest! {
        #[test]
        fn public_key_derivation_is_correct((mut sk_bytes,pk) in secret_public_key_bytes()) {
            let sk = Sum6Kes::from_bytes(&mut sk_bytes);
            prop_assert!(sk?.to_pk() == pk);
        }

        #[test]
        fn keys_are_always_different_if_seeds_are_different(((sk_bytes1,pk1), (sk_bytes2,pk2)) in (secret_public_key_bytes(), secret_public_key_bytes()) ) {
            prop_assert!(sk_bytes1 != sk_bytes2);
            prop_assert!(pk1 != pk2);
        }

        #[test]
        fn period_is_initially_always_zero((mut sk_bytes, _pk) in secret_public_key_bytes()) {
            let sk = Sum6Kes::from_bytes(&mut sk_bytes);
            prop_assert!(sk?.get_period() == 0);
        }

        #[test]
        fn two_msgs_have_different_signature_with_one_skey(((mut sk_bytes,_pk),msg1,msg2) in (secret_public_key_bytes(), payload(),payload())) {
            prop_assume!(msg1 != msg2);

            let mut sk_bytes1 = [0u8; Sum6Kes::SIZE + 4];
            sk_bytes1.copy_from_slice(&sk_bytes);
            let sk = Sum6Kes::from_bytes(&mut sk_bytes);
            let sk_copied = Sum6Kes::from_bytes(&mut sk_bytes1);
            prop_assert!(sk?.sign(&msg1) != sk_copied?.sign(&msg2));
        }

        #[test]
        fn same_msg_have_different_signature_with_two_skey(((mut sk_bytes1,_pk1),(mut sk_bytes2,_pk2),msg) in (secret_public_key_bytes(), secret_public_key_bytes(),payload())) {
            let sk1 = Sum6Kes::from_bytes(&mut sk_bytes1);
            let sk2 = Sum6Kes::from_bytes(&mut sk_bytes2);
            prop_assert!(sk1?.sign(&msg) != sk2?.sign(&msg));
        }

        #[test]
        fn simple_verification_works(((mut sk_bytes,pk),msg) in (secret_public_key_bytes(), payload())) {
            let sk = Sum6Kes::from_bytes(&mut sk_bytes);
            let sig = sk?.sign(&msg);
            prop_assert!(sig.verify(0, &pk, &msg).is_ok());
        }

        #[test]
        fn simple_verification_fails_for_other_msg(((mut sk_bytes,pk),msg1,msg2) in (secret_public_key_bytes(), payload(), payload())) {
            prop_assume!(msg1 != msg2);

            let sk = Sum6Kes::from_bytes(&mut sk_bytes);
            let sig = sk?.sign(&msg1);
            let err_str = String::from("signature error: Verification equation was not satisfied");
            prop_assert!(sig.verify(0, &pk, &msg2) == Err(Error::Ed25519Signature(err_str)));
        }

        #[test]
        fn simple_verification_fails_for_other_pk(((mut sk_bytes,_pk),(mut _sk_bytes,pk),msg) in (secret_public_key_bytes(), secret_public_key_bytes(), payload())) {
            let sk = Sum6Kes::from_bytes(&mut sk_bytes);
            let sig = sk?.sign(&msg);
            prop_assert!(sig.verify(0, &pk, &msg) == Err(Error::InvalidHashComparison));
        }

        #[test]
        fn one_update_behaves_correctly(((mut sk_bytes,pk),msg) in (secret_public_key_bytes(), payload())) {
            let mut sk = Sum6Kes::from_bytes(&mut sk_bytes).unwrap();
            let sig1 = sk.sign(&msg);
            prop_assert!(sig1.verify(0, &pk, &msg).is_ok());

            sk.update().unwrap();

            //can always verify with the same pk signatures after update
            let sig2 = sk.sign(&msg);
            prop_assert!(sig2.verify(1, &pk, &msg).is_ok());
            prop_assert!(sk.get_period() == 1);

            //signatures from different periods of the same message are always different
            prop_assert!(sig1 != sig2);

            //cannot verify signature 2 with pk if period=0
            let err_str = String::from("signature error: Verification equation was not satisfied");
            prop_assert!(sig2.verify(0, &pk, &msg) == Err(Error::Ed25519Signature(err_str)));
        }

        #[test]
        fn n_update_behaves_correctly(((mut sk_bytes,pk),msg, n) in (secret_public_key_bytes(), payload(), 2u32..10)) {
            let mut sk = Sum6Kes::from_bytes(&mut sk_bytes).unwrap();
            let sig1 = sk.sign(&msg);
            prop_assert!(sig1.verify(0, &pk, &msg).is_ok());

            for _ in 0..n {
                sk.update().unwrap();
            }

            //can always verify with the same pk signatures after n updates
            let sig2 = sk.sign(&msg);
            prop_assert!(sig2.verify(n, &pk, &msg).is_ok());
            prop_assert!(sk.get_period() == n);

            //signatures from different periods of the same message are always different
            prop_assert!(sig1 != sig2);

            //cannot verify signature 2 with pk if period=0...n-1
            for i in 0..n {
                if n-i == 1 && n % 2 == 1 {
                    let err_str = String::from("signature error: Verification equation was not satisfied");
                    prop_assert!(sig2.verify(i, &pk, &msg) == Err(Error::Ed25519Signature(err_str)));
                } else {
                    prop_assert!(sig2.verify(i, &pk, &msg) == Err(Error::InvalidHashComparison));
                }
            }
        }
    }
}
