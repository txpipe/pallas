use crate::hash::{Hash, Hasher};

/// A nonce generator function that calculates an epoch nonce from the eta_v value (nc) of the block right before
/// the stability window and the block hash of the first block from the previous epoch (nh).
pub fn generate_epoch_nonce(nc: Hash<32>, nh: Hash<32>, extra_entropy: Option<&[u8]>) -> Hash<32> {
    let mut hasher = Hasher::<256>::new();
    hasher.input(nc.as_ref());
    hasher.input(nh.as_ref());
    let epoch_nonce = hasher.finalize();
    if let Some(extra_entropy) = extra_entropy {
        let mut hasher = Hasher::<256>::new();
        hasher.input(epoch_nonce.as_ref());
        hasher.input(extra_entropy);
        hasher.finalize()
    } else {
        epoch_nonce
    }
}

/// A nonce generator function that calculates a rolling nonce (eta_v) by applying each cardano block in
/// the shelley era and beyond. These rolling nonce values are used to help calculate the epoch
/// nonce values used in consensus for the Ouroboros protocols (tpraos, praos, cpraos).
///
/// # Panic
///
/// This function may panic if the `block_eta_vrf_0` argument is not a slice of
/// either 32 bytes or 64 bytes.
///
pub fn generate_rolling_nonce(previous_block_eta_v: Hash<32>, block_eta_vrf_0: &[u8]) -> Hash<32> {
    assert!(
        block_eta_vrf_0.len() == 32 || block_eta_vrf_0.len() == 64,
        "Invalid block_eta_vrf_0 length: {}, expected 32 or 64",
        block_eta_vrf_0.len()
    );
    let mut hasher = Hasher::<256>::new();
    hasher.input(previous_block_eta_v.as_ref());
    hasher.input(Hasher::<256>::hash(block_eta_vrf_0).as_ref());
    hasher.finalize()
}

#[cfg(test)]
mod tests {
    use itertools::izip;

    use crate::hash::Hash;

    use super::*;

    #[test]
    fn test_epoch_nonce() {
        let nc_values = [hex::decode("e86e133bd48ff5e79bec43af1ac3e348b539172f33e502d2c96735e8c51bd04d")
                .unwrap(),
            hex::decode("d1340a9c1491f0face38d41fd5c82953d0eb48320d65e952414a0c5ebaf87587")
                .unwrap()];
        let nh_values = [hex::decode("d7a1ff2a365abed59c9ae346cba842b6d3df06d055dba79a113e0704b44cc3e9")
                .unwrap(),
            hex::decode("ee91d679b0a6ce3015b894c575c799e971efac35c7a8cbdc2b3f579005e69abd")
                .unwrap()];
        let ee = hex::decode("d982e06fd33e7440b43cefad529b7ecafbaa255e38178ad4189a37e4ce9bf1fa")
            .unwrap();
        let extra_entropy_values: Vec<Option<&[u8]>> = vec![None, Some(&ee)];
        let expected_epoch_nonces = [hex::decode("e536a0081ddd6d19786e9d708a85819a5c3492c0da7349f59c8ad3e17e4acd98")
                .unwrap(),
            hex::decode("0022cfa563a5328c4fb5c8017121329e964c26ade5d167b1bd9b2ec967772b60")
                .unwrap()];

        for (nc_value, nh_value, extra_entropy_value, expected_epoch_nonce) in izip!(
            nc_values.iter(),
            nh_values.iter(),
            extra_entropy_values.iter(),
            expected_epoch_nonces.iter()
        ) {
            let nc: Hash<32> = Hash::from(nc_value.as_slice());
            let nh: Hash<32> = Hash::from(nh_value.as_slice());
            let extra_entropy = *extra_entropy_value;
            let epoch_nonce = generate_epoch_nonce(nc, nh, extra_entropy);
            assert_eq!(epoch_nonce.as_ref(), expected_epoch_nonce.as_slice());
        }
    }

    #[test]
    fn test_rolling_nonce() {
        let shelley_genesis_hash =
            hex::decode("1a3be38bcbb7911969283716ad7aa550250226b76a61fc51cc9a9a35d9276d81")
                .unwrap();

        let eta_vrf_0_values = vec![
            hex::decode("36ec5378d1f5041a59eb8d96e61de96f0950fb41b49ff511f7bc7fd109d4383e1d24be7034e6749c6612700dd5ceb0c66577b88a19ae286b1321d15bce1ab736").unwrap(),
            hex::decode("e0bf34a6b73481302f22987cde4c12807cbc2c3fea3f7fcb77261385a50e8ccdda3226db3efff73e9fb15eecf841bbc85ce37550de0435ebcdcb205e0ed08467").unwrap(),
            hex::decode("7107ef8c16058b09f4489715297e55d145a45fc0df75dfb419cab079cd28992854a034ad9dc4c764544fb70badd30a9611a942a03523c6f3d8967cf680c4ca6b").unwrap(),
            hex::decode("6f561aad83884ee0d7b19fd3d757c6af096bfd085465d1290b13a9dfc817dfcdfb0b59ca06300206c64d1ba75fd222a88ea03c54fbbd5d320b4fbcf1c228ba4e").unwrap(),
            hex::decode("3d3ba80724db0a028783afa56a85d684ee778ae45b9aa9af3120f5e1847be1983bd4868caf97fcfd82d5a3b0b7c1a6d53491d75440a75198014eb4e707785cad").unwrap(),
            hex::decode("0b07976bc04321c2e7ba0f1acb3c61bd92b5fc780a855632e30e6746ab4ac4081490d816928762debd3e512d22ad512a558612adc569718df1784261f5c26aff").unwrap(),
            hex::decode("5e9e001fb1e2ddb0dc7ff40af917ecf4ba9892491d4bcbf2c81db2efc57627d40d7aac509c9bcf5070d4966faaeb84fd76bb285af2e51af21a8c024089f598c1").unwrap(),
            hex::decode("182e83f8c67ad2e6bddead128e7108499ebcbc272b50c42783ef08f035aa688fecc7d15be15a90dbfe7fe5d7cd9926987b6ec12b05f2eadfe0eb6cad5130aca4").unwrap(),
            hex::decode("275e7404b2385a9d606d67d0e29f5516fb84c1c14aaaf91afa9a9b3dcdfe09075efdadbaf158cfa1e9f250cc7c691ed2db4a29288d2426bd74a371a2a4b91b57").unwrap(),
            hex::decode("0f35c7217792f8b0cbb721ae4ae5c9ae7f2869df49a3db256aacc10d23997a09e0273261b44ebbcecd6bf916f2c1cd79cf25b0c2851645d75dd0747a8f6f92f5").unwrap(),
            hex::decode("14c28bf9b10421e9f90ffc9ab05df0dc8c8a07ffac1c51725fba7e2b7972d0769baea248f93ed0f2067d11d719c2858c62fc1d8d59927b41d4c0fbc68d805b32").unwrap(),
            hex::decode("e4ce96fee9deb9378a107db48587438cddf8e20a69e21e5e4fbd35ef0c56530df77eba666cb152812111ba66bbd333ed44f627c727115f8f4f15b31726049a19").unwrap(),
            hex::decode("b38f315e3ce369ea2551bf4f44e723dd15c7d67ba4b3763997909f65e46267d6540b9b00a7a65ae3d1f3a3316e57a821aeaac33e4e42ded415205073134cd185").unwrap(),
            hex::decode("4bcbf774af9c8ff24d4d96099001ec06a24802c88fea81680ea2411392d32dbd9b9828a690a462954b894708d511124a2db34ec4179841e07a897169f0f1ac0e").unwrap(),
            hex::decode("65247ace6355f978a12235265410c44f3ded02849ec8f8e6db2ac705c3f57d322ea073c13cf698e15d7e1d7f2bc95e7b3533be0dee26f58864f1664df0c1ebba").unwrap(),
            hex::decode("d0c2bb451d0a3465a7fef7770718e5e49bf092a85dbf5af66ea26ec9c1b359026905fc1457e2b98b01ede7ba42aedcc525301f747a0ed9a9b61c37f27f9d8812").unwrap(),
            hex::decode("250d9ec7ebec73e885798ae9427e1ea47b5ae66059b465b7c0fd132d17a9c2dcae29ba72863c1861cfb776d342812c4e9000981c4a40819430d0e84aa8bfeb0d").unwrap(),
            hex::decode("0549cc0a5e5b9920796b88784c49b7d9a04cf2e86ab18d5af7b00780e60fb0fb5a7129945f4f918201dbad5348d4ccface4370f266540f8e072cdb46d3705930").unwrap(),
            hex::decode("e543a26031dbdc8597b1beeba48a4f1cf6ab90c0e5b9343936b6e948a791198fc4fa22928e21edec812a04d0c9629772bf78e475d91a323cd8a8a6e005f92b4d").unwrap(),
            hex::decode("4e4be69ad170fb8b3b17835913391ee537098d49e4452844a71ab2147ac55e45871c8943271806034ee9450b31c9486db9d26942946f48040ece7eea81424af1").unwrap(),
            hex::decode("cb8a528288f902349250f9e8015e8334b0e24c2eeb9bb7d75e73c39024685804577565e62aca35948d2686ea38e9f8de97837ea30d2fb08347768394416e4a38").unwrap(),
            hex::decode("fce94c47196a56a5cb94d5151ca429daf1c563ae889d0a42c2d03cfe43c94a636221c7e21b0668de9e5b6b32ee1e78b2c9aabc16537bf79c7b85eb956f433ac7").unwrap(),
            hex::decode("fc8a125c9e2418c87907db4437a0ad6a378bba728ac8e0ce0e64f2a2f4b8201315e1b08d7983ce597cb68be2a2400d6d0d59b7359fe3dc9daca73d468da48972").unwrap(),
            hex::decode("49290417311420d67f029a80b013b754150dd0097aa64de1c14a2467ab2e26cc2724071c04cb90cb0cf6c6353cf31f63235af7849d6ba023fd0fc0bc79d32f0b").unwrap(),
            hex::decode("45c65effdc8007c9f2fc9057af986e94eb5c12b755465058d4b933ee37638452c5eeca4b43b8cbddabc60f29cbe5676b0bc55c0da88f8d0c36068e7d17ee603a").unwrap(),
            hex::decode("a51e4e0f28aee3024207d87a5a1965313bdba4df44c6b845f7ca3408e5dabfe873df6b6ba26000e841f83f69e1de7857122ba538b42f255da2d013208af806ba").unwrap(),
            hex::decode("5dbd891bf3bcfd5d054274759c13552aeaa187949875d81ee62ed394253ae25182e78b3a4a1976a7674e425bab860931d57f8a1d4fdc81fa4c3e8e8bf9016d5d").unwrap(),
            hex::decode("3b5b044026e9066d62ce2f5a1fb01052a8cfe200dea28d421fc70f42c4d2b890b90ffef5675de1e47e4a20c9ca8700ceea23a61338ac759a098d167fa71642cb").unwrap(),
            hex::decode("bb4017880cfa1e37f256dfe2a9cdb1349ed5dea8f69de75dc5933540dcf49e69afc33c837ba8a791857e16fad8581c4e9046778c49ca1ecd1fb675983be6d721").unwrap(),
            hex::decode("517bbdb6e9e5f4702193064543204e780f5d33a866d0dcd65ada19f05715dea60ca81b842de5dca8f6b84a9cf469c8fb81991369dba21571476cc9c8d4ff2136").unwrap(),
        ];

        let expected_eta_v_values = vec![
            hex::decode("2af15f57076a8ff225746624882a77c8d2736fe41d3db70154a22b50af851246")
                .unwrap(),
            hex::decode("a815ff978369b57df09b0072485c26920dc0ec8e924a852a42f0715981cf0042")
                .unwrap(),
            hex::decode("f112d91435b911b6b5acaf27198762905b1cdec8c5a7b712f925ce3c5c76bb5f")
                .unwrap(),
            hex::decode("5450d95d9be4194a0ded40fbb4036b48d1f1d6da796e933fefd2c5c888794b4b")
                .unwrap(),
            hex::decode("c5c0f406cb522ad3fead4ecc60bce9c31e80879bc17eb1bb9acaa9b998cdf8bf")
                .unwrap(),
            hex::decode("5857048c728580549de645e087ba20ef20bb7c51cc84b5bc89df6b8b0ed98c41")
                .unwrap(),
            hex::decode("d6f40ef403687115db061b2cb9b1ab4ddeb98222075d5a3e03c8d217d4d7c40e")
                .unwrap(),
            hex::decode("5489d75a9f4971c1824462b5e2338609a91f121241f21fee09811bd5772ae0a8")
                .unwrap(),
            hex::decode("04716326833ecdb595153adac9566a4b39e5c16e8d02526cb4166e4099a00b1a")
                .unwrap(),
            hex::decode("39db709f50c8a279f0a94adcefb9360dbda6cdce168aed4288329a9cd53492b6")
                .unwrap(),
            hex::decode("c784b8c8678e0a04748a3ad851dd7c34ed67141cd9dc0c50ceaff4df804699a7")
                .unwrap(),
            hex::decode("cc1a5861358c075de93a26a91c5a951d5e71190d569aa2dc786d4ca8fc80cc38")
                .unwrap(),
            hex::decode("514979c89313c49e8f59fb8445113fa7623e99375cc4917fe79df54f8d4bdfce")
                .unwrap(),
            hex::decode("6a783e04481b9e04e8f3498a3b74c90c06a1031fb663b6793ce592a6c26f56f4")
                .unwrap(),
            hex::decode("1190f5254599dcee4f3cf1afdf4181085c36a6db6c30f334bfe6e6f320a6ed91")
                .unwrap(),
            hex::decode("91c777d6db066fe58edd67cd751fc7240268869b365393f6910e0e8f0fa58af3")
                .unwrap(),
            hex::decode("c545d83926c011b5c68a72de9a4e2f9da402703f4aab1b967456eae73d9f89b3")
                .unwrap(),
            hex::decode("ec31d2348bf543482842843a61d5b32691dedf801f198d68126c423ddf391e8b")
                .unwrap(),
            hex::decode("de223867d5c972895dd99ac0280a3e02947a7fb018ed42ed048266f913d2dfc2")
                .unwrap(),
            hex::decode("4dd9801752aade9c6e06bf03e9d2ec8a30ef7c6f30106790a23a9599e90ee08a")
                .unwrap(),
            hex::decode("fcb183abd512271f40408a5872827ce79cc2dda685a986a7dbdc61d842495a91")
                .unwrap(),
            hex::decode("e834d8ffd6dd042167b13e38512c62afdaf4d635d5b1ab0d513e08e9bef0ef63")
                .unwrap(),
            hex::decode("270a78257a958cd5fdb26f0b9ab302df2d2196fd04989f7ca1bb703e4dd904f0")
                .unwrap(),
            hex::decode("7e324f67af787dfddee10354128c60c60bf601bd8147c867d2471749a7b0f334")
                .unwrap(),
            hex::decode("54521ed42e0e782b5268ec55f80cff582162bc23fdcee5cdaa0f1a2ce7fa1f02")
                .unwrap(),
            hex::decode("557c296a71d8c9cb3fe7dcd95fbf4d70f6a3974d93c71b450d62a41b9a85d5a1")
                .unwrap(),
            hex::decode("20e078301ca282857378bbf10ac40965445c4c9fa73a160e0a116b4cf808b4b4")
                .unwrap(),
            hex::decode("b5a741dd3ff6a5a3d27b4d046dfb7a3901aacd37df7e931ba05e1320ad155c1c")
                .unwrap(),
            hex::decode("8b445f35f4a7b76e5d279d71fa9e05376a7c4533ca8b2b98fd2dbaf814d3bf8f")
                .unwrap(),
            hex::decode("08e7b5277abc139deb50f61264375fa091c580f8a85f259be78a002f7023c31f")
                .unwrap(),
        ];

        let mut previous_block_eta_v = Hash::<32>::from(shelley_genesis_hash.as_slice());

        for (eta_vrf_0, expected_eta_v) in eta_vrf_0_values.iter().zip(expected_eta_v_values.iter())
        {
            let rolling_nonce = generate_rolling_nonce(previous_block_eta_v, eta_vrf_0);
            assert_eq!(rolling_nonce.as_ref(), expected_eta_v.as_slice());
            previous_block_eta_v = rolling_nonce;
        }
    }
}
