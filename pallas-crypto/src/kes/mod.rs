#[cfg(test)]
mod tests {
    use kes_summed_ed25519::kes::Sum6KesSig;
    use kes_summed_ed25519::traits::KesSig;
    use kes_summed_ed25519::PublicKey;
    use pallas_traverse::MultiEraHeader;

    // utility function to calculate the KES period given a slot and some genesis values
    fn slot_to_kes_period(slot: u64) -> u64 {
        let slot_length = 1u64; // from shelley-genesis.json (1 second)
        let slots_per_kes_period = 129600u64; // from shelley-genesis.json (1.5 days in seconds)
        slot / (slots_per_kes_period * slot_length)
    }

    #[test]
    fn test_kes_key_block_verification() {
        let test_block = include_bytes!("tests/mainnet_blockheader_10817298.cbor");
        let conway_block_tag: u8 = 6;
        let multi_era_header = MultiEraHeader::decode(conway_block_tag, None, test_block).unwrap();
        let header_hash = multi_era_header.hash();
        println!("header_hash: {}", hex::encode(header_hash.as_ref()));
        assert_eq!(
            hex::encode(header_hash),
            "627ea281970fc48f033c2d50d0a3393af5015ec6aaa0af435d8f2877173156ce"
        );

        let babbage_header = multi_era_header.as_babbage().expect("Infallible");
        assert_eq!(babbage_header.header_body.slot, 134402628u64);
        assert_eq!(
            hex::encode(
                &babbage_header
                    .header_body
                    .operational_cert
                    .operational_cert_hot_vkey
                    .as_slice()
            ),
            "2e5823037de29647e495b97d9dd7bf739f7ebc11d3701c8d0720f55618e1b292"
        );

        // We needed MintedHeader to be able to extract the header_body_cbor
        let header_body_cbor: &[u8] = multi_era_header.header_body_cbor().expect("Infallible");
        println!("header_body_cbor: {}", hex::encode(header_body_cbor));
        // let header_body_cbor = hex::decode("8a1a00a50f121a0802d24458203deea82abe788d260b8987a522aadec86c9f098e88a57d7cfcdb24f474a7afb65820cad3c900ca6baee9e65bf61073d900bfbca458eeca6d0b9f9931f5b1017a8cd65820576d49e98adfab65623dc16f9fff2edd210e8dd1d4588bfaf8af250beda9d3c7825840d944b8c81000fc1182ec02194ca9eca510fd84995d22bfe1842190b39d468e5ecbd863969e0c717b0071a371f748d44c895fa9233094cefcd3107410baabb19a5850f2a29f985d37ca8eb671c2847fab9cc45c93738a430b4e43837e7f33028b190a7e55152b0e901548961a66d56eebe72d616f9e68fd13e9955ccd8611c201a5b422ac8ef56af74cb657b5b868ce9d850f1945d15820639d4986d17de3cac8079a3b25d671f339467aa3a9948e29992dafebf90f719f8458202e5823037de29647e495b97d9dd7bf739f7ebc11d3701c8d0720f55618e1b292171903e958401feeeabc7460b19370f4050e986b558b149fdc8724b4a4805af8fe45c8e7a7c6753894ad7a1b9c313da269ddc5922e150da3b378977f1dfea79fc52fd2c12f08820901").unwrap();

        // let mut kes_sk_bytes = hex::decode("68b77b6e61925be0499d1445fd9210cec5bdfd5dd92662802eb2720ff70bc68fd8964580ff18bd2b232eb716dfbbeef82e2844b466ddd5dacaad9f15d3c753b348354141e973d039b1147c48e71e5b7cadc6deb28c86e4ae4fc26e8bbe1695c3374d4eb1094a7a698722894301546466c750947778b18ac3270397efd2eced4d25ced55d2bd2c09e7c0fa7b849d41787ca11defc91609d930a9870881a56a587bff20b2c5c59f63ccb008be495917da3fcae536d05401b6771bb1f9356f031b3ddadbffbc426a9a23e34274b187f7e93892e990644f6273772a02d3e38bee7459ed6a9bb5760fe012e47a2e75880125e7fb072b2b7a626a5375e2039d8d748cb8ad4dd02697250d3155eee39308ecc2925405a8c15e1cbe556cc4315d43ee5101003639bcb33bd6e27da3885888d7cca20b05cadbaa53941ef5282cde8f377c3bd0bf732cfac6b5d4d5597a1f72d81bc0d8af634a4c760b309fe8959bbde666ff10310377b313860bd52d56fd7cb149633beb1eb2e0076111df61e570a042f7cebae74a8de298a6f114938946230db42651ea4eddf5df2d7d2f3016464073da8a9dc715817b43586a61874e576da7b47a2bb6c2e19d4cbd5b1b39a24427e89b812cce6d30e0506e207f1eaab313c45a236068ea319958474237a5ffe02736e1c51c02a05999816c9253a557f09375c83acf5d7250f3bbc638e10c58fb274e2002eed841ecef6a9cbc57c3157a7c3cf47e66b1741e8173b6676ac973bc9715027a3225087cabad45407b891416330485891dc9a3875488a26428d20d581b629a8f4f42e3aa00cbcaae6c8e2b8f3fe033b874d1de6a3f8c321c92b77643f00d28e").unwrap();
        // kes_sk_bytes.extend([0u8; 4]);
        // let mut kes_sk = Sum6Kes::from_bytes(&mut kes_sk_bytes).unwrap();
        // println!("kes_sk period: {}", kes_sk.get_period());
        // println!("kes_sk: {}", hex::encode(kes_sk.as_bytes()));
        // let kes_pk = kes_sk.to_pk();
        // println!("kes_pk: {}", hex::encode(kes_pk.as_bytes()));
        // assert_eq!(kes_sk.get_period(), 0);
        // assert_eq!(hex::encode(kes_sk.as_bytes()), "68b77b6e61925be0499d1445fd9210cec5bdfd5dd92662802eb2720ff70bc68fd8964580ff18bd2b232eb716dfbbeef82e2844b466ddd5dacaad9f15d3c753b348354141e973d039b1147c48e71e5b7cadc6deb28c86e4ae4fc26e8bbe1695c3374d4eb1094a7a698722894301546466c750947778b18ac3270397efd2eced4d25ced55d2bd2c09e7c0fa7b849d41787ca11defc91609d930a9870881a56a587bff20b2c5c59f63ccb008be495917da3fcae536d05401b6771bb1f9356f031b3ddadbffbc426a9a23e34274b187f7e93892e990644f6273772a02d3e38bee7459ed6a9bb5760fe012e47a2e75880125e7fb072b2b7a626a5375e2039d8d748cb8ad4dd02697250d3155eee39308ecc2925405a8c15e1cbe556cc4315d43ee5101003639bcb33bd6e27da3885888d7cca20b05cadbaa53941ef5282cde8f377c3bd0bf732cfac6b5d4d5597a1f72d81bc0d8af634a4c760b309fe8959bbde666ff10310377b313860bd52d56fd7cb149633beb1eb2e0076111df61e570a042f7cebae74a8de298a6f114938946230db42651ea4eddf5df2d7d2f3016464073da8a9dc715817b43586a61874e576da7b47a2bb6c2e19d4cbd5b1b39a24427e89b812cce6d30e0506e207f1eaab313c45a236068ea319958474237a5ffe02736e1c51c02a05999816c9253a557f09375c83acf5d7250f3bbc638e10c58fb274e2002eed841ecef6a9cbc57c3157a7c3cf47e66b1741e8173b6676ac973bc9715027a3225087cabad45407b891416330485891dc9a3875488a26428d20d581b629a8f4f42e3aa00cbcaae6c8e2b8f3fe033b874d1de6a3f8c321c92b77643f00d28e00000000");
        // assert_eq!(hex::encode(kes_pk.as_bytes()), "2e5823037de29647e495b97d9dd7bf739f7ebc11d3701c8d0720f55618e1b292");

        // TODO: Verify the opcert signature by the node's cold key
        // ...
        // ...

        let kes_pk_bytes = babbage_header
            .header_body
            .operational_cert
            .operational_cert_hot_vkey
            .as_slice();
        assert_eq!(
            hex::encode(kes_pk_bytes),
            "2e5823037de29647e495b97d9dd7bf739f7ebc11d3701c8d0720f55618e1b292"
        );
        let kes_pk = PublicKey::from_bytes(kes_pk_bytes).unwrap();
        // let kes_pk = PublicKey::from_bytes(&hex::decode("2e5823037de29647e495b97d9dd7bf739f7ebc11d3701c8d0720f55618e1b292").unwrap()).unwrap();

        // calculate the right period to verify the signature
        let opcert_kes_period = babbage_header
            .header_body
            .operational_cert
            .operational_cert_kes_period;
        assert_eq!(opcert_kes_period, 1001u64);
        let slot_kes_period = slot_to_kes_period(babbage_header.header_body.slot);
        assert_eq!(slot_kes_period, 1037u64);
        let kes_period = (slot_kes_period - opcert_kes_period) as u32;
        assert_eq!(kes_period, 36u32);

        let signature = Sum6KesSig::from_bytes(babbage_header.body_signature.as_slice()).unwrap();
        assert!(
            signature
                .verify(kes_period, &kes_pk, header_body_cbor)
                .is_ok(),
            "Signature verification failed"
        );
        // assert!(signature.verify(kes_period, &kes_pk, header_body_cbor.as_slice()).is_ok(), "Signature verification failed");
    }
}
