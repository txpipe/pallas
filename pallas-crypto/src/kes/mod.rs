use kes_summed_ed25519::kes::{Sum6Kes, Sum6KesSig};
use kes_summed_ed25519::traits::{KesSig, KesSk};
use kes_summed_ed25519::PublicKey;
use thiserror::Error;

/// KES error
#[derive(Error, Debug)]
pub enum Error {
    #[error("KES error: {0}")]
    Kes(#[from] kes_summed_ed25519::errors::Error),
}

/// KES secret key
pub struct KesSecretKey<'a> {
    sum_6_kes: Sum6Kes<'a>,
}

impl KesSecretKey<'_> {
    /// Create a new KES secret key
    pub fn from_bytes(sk_bytes: &mut Vec<u8>) -> Result<KesSecretKey, Error> {
        // TODO: extend() could potentially re-allocate memory to a new location and copy the sk_bytes.
        // This would leave the original memory containing the secret key without being wiped.
        sk_bytes.extend([0u8; 4]); // default to period = 0
        let sum_6_kes = Sum6Kes::from_bytes(sk_bytes.as_mut_slice())?;
        Ok(KesSecretKey { sum_6_kes })
    }

    /// Get the internal representation of the KES secret key at the current period
    /// This value will include the period as the last 4 bytes in big-endian format
    ///
    /// # Safety
    /// This function is marked unsafe because we wished to highlight the
    /// importance of keeping the content of the secret key private.
    /// However there are reasons that may be valid to _leak_ the private
    /// key: to encrypt it and store securely.
    pub unsafe fn leak_into_bytes(&self) -> &[u8] {
        self.sum_6_kes.as_bytes()
    }

    /// Get the KesPublicKey from the KesSecretKey
    pub fn to_pk(&self) -> KesPublicKey {
        KesPublicKey {
            kes_pk: self.sum_6_kes.to_pk(),
        }
    }

    /// Get the current period of the KES secret key
    pub fn get_period(&self) -> u32 {
        self.sum_6_kes.get_period()
    }

    /// Update the KES secret key to the next period
    pub fn update(&mut self) -> Result<(), Error> {
        Ok(self.sum_6_kes.update()?)
    }
}

/// KES public key
pub struct KesPublicKey {
    kes_pk: PublicKey,
}

impl KesPublicKey {
    /// Create a new KES public key
    pub fn from_bytes(pk_bytes: &[u8]) -> Result<KesPublicKey, Error> {
        let kes_pk = PublicKey::from_bytes(pk_bytes)?;
        Ok(KesPublicKey { kes_pk })
    }

    /// Get the internal representation of the KES public key
    pub fn as_bytes(&self) -> &[u8] {
        self.kes_pk.as_bytes()
    }
}

/// KES signature
pub struct KesSignature {
    sum_6_kes_sig: Sum6KesSig,
}

impl KesSignature {
    /// Create a new KES signature
    pub fn from_bytes(sig_bytes: &[u8]) -> Result<KesSignature, Error> {
        let sum_6_kes_sig = Sum6KesSig::from_bytes(sig_bytes)?;
        Ok(KesSignature { sum_6_kes_sig })
    }

    /// Get the internal representation of the KES signature
    pub fn to_bytes(&self) -> [u8; 448] {
        self.sum_6_kes_sig.to_bytes()
    }

    /// Verify the KES signature
    pub fn verify(&self, kes_period: u32, kes_pk: &KesPublicKey, msg: &[u8]) -> Result<(), Error> {
        Ok(self.sum_6_kes_sig.verify(kes_period, &kes_pk.kes_pk, msg)?)
    }
}

#[cfg(test)]
mod tests {
    use crate::kes::{KesPublicKey, KesSecretKey, KesSignature};

    #[test]
    fn kes_key_evolution() {
        let mut kes_sk_bytes = hex::decode("68b77b6e61925be0499d1445fd9210cec5bdfd5dd92662802eb2720ff70bc68fd8964580ff18bd2b232eb716dfbbeef82e2844b466ddd5dacaad9f15d3c753b348354141e973d039b1147c48e71e5b7cadc6deb28c86e4ae4fc26e8bbe1695c3374d4eb1094a7a698722894301546466c750947778b18ac3270397efd2eced4d25ced55d2bd2c09e7c0fa7b849d41787ca11defc91609d930a9870881a56a587bff20b2c5c59f63ccb008be495917da3fcae536d05401b6771bb1f9356f031b3ddadbffbc426a9a23e34274b187f7e93892e990644f6273772a02d3e38bee7459ed6a9bb5760fe012e47a2e75880125e7fb072b2b7a626a5375e2039d8d748cb8ad4dd02697250d3155eee39308ecc2925405a8c15e1cbe556cc4315d43ee5101003639bcb33bd6e27da3885888d7cca20b05cadbaa53941ef5282cde8f377c3bd0bf732cfac6b5d4d5597a1f72d81bc0d8af634a4c760b309fe8959bbde666ff10310377b313860bd52d56fd7cb149633beb1eb2e0076111df61e570a042f7cebae74a8de298a6f114938946230db42651ea4eddf5df2d7d2f3016464073da8a9dc715817b43586a61874e576da7b47a2bb6c2e19d4cbd5b1b39a24427e89b812cce6d30e0506e207f1eaab313c45a236068ea319958474237a5ffe02736e1c51c02a05999816c9253a557f09375c83acf5d7250f3bbc638e10c58fb274e2002eed841ecef6a9cbc57c3157a7c3cf47e66b1741e8173b6676ac973bc9715027a3225087cabad45407b891416330485891dc9a3875488a26428d20d581b629a8f4f42e3aa00cbcaae6c8e2b8f3fe033b874d1de6a3f8c321c92b77643f00d28e").unwrap();
        let mut kes_sk = KesSecretKey::from_bytes(&mut kes_sk_bytes).unwrap();
        let kes_pk = kes_sk.to_pk();
        assert_eq!(kes_sk.get_period(), 0);
        assert_eq!(hex::encode(unsafe { kes_sk.leak_into_bytes() }), "68b77b6e61925be0499d1445fd9210cec5bdfd5dd92662802eb2720ff70bc68fd8964580ff18bd2b232eb716dfbbeef82e2844b466ddd5dacaad9f15d3c753b348354141e973d039b1147c48e71e5b7cadc6deb28c86e4ae4fc26e8bbe1695c3374d4eb1094a7a698722894301546466c750947778b18ac3270397efd2eced4d25ced55d2bd2c09e7c0fa7b849d41787ca11defc91609d930a9870881a56a587bff20b2c5c59f63ccb008be495917da3fcae536d05401b6771bb1f9356f031b3ddadbffbc426a9a23e34274b187f7e93892e990644f6273772a02d3e38bee7459ed6a9bb5760fe012e47a2e75880125e7fb072b2b7a626a5375e2039d8d748cb8ad4dd02697250d3155eee39308ecc2925405a8c15e1cbe556cc4315d43ee5101003639bcb33bd6e27da3885888d7cca20b05cadbaa53941ef5282cde8f377c3bd0bf732cfac6b5d4d5597a1f72d81bc0d8af634a4c760b309fe8959bbde666ff10310377b313860bd52d56fd7cb149633beb1eb2e0076111df61e570a042f7cebae74a8de298a6f114938946230db42651ea4eddf5df2d7d2f3016464073da8a9dc715817b43586a61874e576da7b47a2bb6c2e19d4cbd5b1b39a24427e89b812cce6d30e0506e207f1eaab313c45a236068ea319958474237a5ffe02736e1c51c02a05999816c9253a557f09375c83acf5d7250f3bbc638e10c58fb274e2002eed841ecef6a9cbc57c3157a7c3cf47e66b1741e8173b6676ac973bc9715027a3225087cabad45407b891416330485891dc9a3875488a26428d20d581b629a8f4f42e3aa00cbcaae6c8e2b8f3fe033b874d1de6a3f8c321c92b77643f00d28e00000000");
        assert_eq!(
            hex::encode(kes_pk.as_bytes()),
            "2e5823037de29647e495b97d9dd7bf739f7ebc11d3701c8d0720f55618e1b292"
        );
        kes_sk.update().unwrap();
        assert_eq!(kes_sk.get_period(), 1);
        assert_eq!(hex::encode(unsafe { kes_sk.leak_into_bytes() }), "d8964580ff18bd2b232eb716dfbbeef82e2844b466ddd5dacaad9f15d3c753b3000000000000000000000000000000000000000000000000000000000000000048354141e973d039b1147c48e71e5b7cadc6deb28c86e4ae4fc26e8bbe1695c3374d4eb1094a7a698722894301546466c750947778b18ac3270397efd2eced4d25ced55d2bd2c09e7c0fa7b849d41787ca11defc91609d930a9870881a56a587bff20b2c5c59f63ccb008be495917da3fcae536d05401b6771bb1f9356f031b3ddadbffbc426a9a23e34274b187f7e93892e990644f6273772a02d3e38bee7459ed6a9bb5760fe012e47a2e75880125e7fb072b2b7a626a5375e2039d8d748cb8ad4dd02697250d3155eee39308ecc2925405a8c15e1cbe556cc4315d43ee5101003639bcb33bd6e27da3885888d7cca20b05cadbaa53941ef5282cde8f377c3bd0bf732cfac6b5d4d5597a1f72d81bc0d8af634a4c760b309fe8959bbde666ff10310377b313860bd52d56fd7cb149633beb1eb2e0076111df61e570a042f7cebae74a8de298a6f114938946230db42651ea4eddf5df2d7d2f3016464073da8a9dc715817b43586a61874e576da7b47a2bb6c2e19d4cbd5b1b39a24427e89b812cce6d30e0506e207f1eaab313c45a236068ea319958474237a5ffe02736e1c51c02a05999816c9253a557f09375c83acf5d7250f3bbc638e10c58fb274e2002eed841ecef6a9cbc57c3157a7c3cf47e66b1741e8173b6676ac973bc9715027a3225087cabad45407b891416330485891dc9a3875488a26428d20d581b629a8f4f42e3aa00cbcaae6c8e2b8f3fe033b874d1de6a3f8c321c92b77643f00d28e00000001");
        kes_sk.update().unwrap();
        assert_eq!(kes_sk.get_period(), 2);
        assert_eq!(hex::encode(unsafe { kes_sk.leak_into_bytes() }), "ba1aacc52e63e6121c2a205ecb5ae8f34a6e49bcc24254ca495a4affcd27fd869dd961c480648623218bf9b5f69d557547586a4fca8391092c59e27ef6a0584f7bc8e482396722f2a0a36fc2ac660eb44d8f5a5cf6916bba31afda77ef53364ea5ea39cc7e01d4ec1fce69c4b1f359781f460f373b9a81e7d3034b5baa3853840000000000000000000000000000000000000000000000000000000000000000bff20b2c5c59f63ccb008be495917da3fcae536d05401b6771bb1f9356f031b3ddadbffbc426a9a23e34274b187f7e93892e990644f6273772a02d3e38bee7459ed6a9bb5760fe012e47a2e75880125e7fb072b2b7a626a5375e2039d8d748cb8ad4dd02697250d3155eee39308ecc2925405a8c15e1cbe556cc4315d43ee5101003639bcb33bd6e27da3885888d7cca20b05cadbaa53941ef5282cde8f377c3bd0bf732cfac6b5d4d5597a1f72d81bc0d8af634a4c760b309fe8959bbde666ff10310377b313860bd52d56fd7cb149633beb1eb2e0076111df61e570a042f7cebae74a8de298a6f114938946230db42651ea4eddf5df2d7d2f3016464073da8a9dc715817b43586a61874e576da7b47a2bb6c2e19d4cbd5b1b39a24427e89b812cce6d30e0506e207f1eaab313c45a236068ea319958474237a5ffe02736e1c51c02a05999816c9253a557f09375c83acf5d7250f3bbc638e10c58fb274e2002eed841ecef6a9cbc57c3157a7c3cf47e66b1741e8173b6676ac973bc9715027a3225087cabad45407b891416330485891dc9a3875488a26428d20d581b629a8f4f42e3aa00cbcaae6c8e2b8f3fe033b874d1de6a3f8c321c92b77643f00d28e00000002");
    }

    #[test]
    fn kes_signature_verify() {
        let kes_pk_bytes =
            hex::decode("2e5823037de29647e495b97d9dd7bf739f7ebc11d3701c8d0720f55618e1b292")
                .unwrap();
        let kes_pk = KesPublicKey::from_bytes(&kes_pk_bytes).unwrap();
        let kes_signature_bytes = hex::decode("20f1c8f9ae672e6ec75b0aa63a85e7ab7865b95f6b2907a26b54c14f49184ab52cf98ef441bb71de50380325b34f16d84fc78d137467a1b49846747cf8ee4701c56f08f198b94c468d46b67b271f5bc30ab2ad14b1bdbf2be0695a00fe4b02b3060fa52128f4cce9c5759df0ba8d71fe99456bd2e333671e45110908d03a2ec3b38599d26adf182ba63f79900fdb2732947cf8e940a4cf1e8db9b4cf4c001dbd37c60d0e38851de4910807896153be455e13161342d4c6f7bb3e4d2d35dbbbba0ebcd161be2f1ec030d2f5a6059ac89dfa70dc6b3d0bc2da179c62ae95c4f9c7ad9c0387b35bf2b45b325d1e0a18c0c783a0779003bf23e7a6b00cc126c5e3d51a57d41ff1707a76fb2c306a67c21473b41f1d9a7f64a670ec172a2421da03d796fa97086de8812304f4f96bd45243d0a2ad6c48a69d9e2c0afbb1333acee607d18eb3a33818c3c9d5bb72cade889379008bf60d436298cb0cfc6159332cb1af1de4f1d64e79c399d058ac4993704eed67917093f89db6cde830383e69aa400ba3225087cabad45407b891416330485891dc9a3875488a26428d20d581b629a8f4f42e3aa00cbcaae6c8e2b8f3fe033b874d1de6a3f8c321c92b77643f00d28e").unwrap();
        let kes_signature = KesSignature::from_bytes(&kes_signature_bytes).unwrap();
        let kes_period = 36u32;
        let kes_msg = hex::decode("8a1a00a50f121a0802d24458203deea82abe788d260b8987a522aadec86c9f098e88a57d7cfcdb24f474a7afb65820cad3c900ca6baee9e65bf61073d900bfbca458eeca6d0b9f9931f5b1017a8cd65820576d49e98adfab65623dc16f9fff2edd210e8dd1d4588bfaf8af250beda9d3c7825840d944b8c81000fc1182ec02194ca9eca510fd84995d22bfe1842190b39d468e5ecbd863969e0c717b0071a371f748d44c895fa9233094cefcd3107410baabb19a5850f2a29f985d37ca8eb671c2847fab9cc45c93738a430b4e43837e7f33028b190a7e55152b0e901548961a66d56eebe72d616f9e68fd13e9955ccd8611c201a5b422ac8ef56af74cb657b5b868ce9d850f1945d15820639d4986d17de3cac8079a3b25d671f339467aa3a9948e29992dafebf90f719f8458202e5823037de29647e495b97d9dd7bf739f7ebc11d3701c8d0720f55618e1b292171903e958401feeeabc7460b19370f4050e986b558b149fdc8724b4a4805af8fe45c8e7a7c6753894ad7a1b9c313da269ddc5922e150da3b378977f1dfea79fc52fd2c12f08820901").unwrap();
        assert!(kes_signature.verify(kes_period, &kes_pk, &kes_msg).is_ok());
    }
}
