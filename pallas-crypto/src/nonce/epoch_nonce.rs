use crate::hash::{Hash, Hasher};
use crate::nonce::{Error, NonceGenerator};

/// A nonce generator that calculates an epoch nonce from the eta_v value (nc) of the block right before
/// the stability window and the block hash of the first block from the previous epoch (nh).
#[derive(Debug, Clone)]
pub struct EpochNonceGenerator {
    pub nonce: Hash<32>,
}

impl EpochNonceGenerator {
    /// Create a new [`EpochNonceGenerator`] generator.
    /// params:
    /// - nc: the eta_v value of the block right before the stability window.
    /// - nh: the block hash of the first block from the previous epoch.
    /// - extra_entropy: optional extra entropy to be used in the nonce calculation.
    pub fn new(nc: Hash<32>, nh: Hash<32>, extra_entropy: Option<&[u8]>) -> Self {
        let mut hasher = Hasher::<256>::new();
        hasher.input(nc.as_ref());
        hasher.input(nh.as_ref());
        let epoch_nonce = hasher.finalize();
        if let Some(extra_entropy) = extra_entropy {
            let mut hasher = Hasher::<256>::new();
            hasher.input(epoch_nonce.as_ref());
            hasher.input(extra_entropy);
            let extra_nonce = hasher.finalize();
            Self { nonce: extra_nonce }
        } else {
            Self { nonce: epoch_nonce }
        }
    }
}

impl NonceGenerator for EpochNonceGenerator {
    fn finalize(&mut self) -> Result<Hash<32>, Error> {
        Ok(self.nonce)
    }
}

#[cfg(test)]
mod tests {
    use itertools::izip;

    use crate::hash::Hash;

    use super::*;

    #[test]
    fn test_epoch_nonce() {
        let nc_values = vec![
            hex::decode("e86e133bd48ff5e79bec43af1ac3e348b539172f33e502d2c96735e8c51bd04d")
                .unwrap(),
            hex::decode("d1340a9c1491f0face38d41fd5c82953d0eb48320d65e952414a0c5ebaf87587")
                .unwrap(),
        ];
        let nh_values = vec![
            hex::decode("d7a1ff2a365abed59c9ae346cba842b6d3df06d055dba79a113e0704b44cc3e9")
                .unwrap(),
            hex::decode("ee91d679b0a6ce3015b894c575c799e971efac35c7a8cbdc2b3f579005e69abd")
                .unwrap(),
        ];
        let ee = hex::decode("d982e06fd33e7440b43cefad529b7ecafbaa255e38178ad4189a37e4ce9bf1fa")
            .unwrap();
        let extra_entropy_values: Vec<Option<&[u8]>> = vec![None, Some(&ee)];
        let expected_epoch_nonces = vec![
            hex::decode("e536a0081ddd6d19786e9d708a85819a5c3492c0da7349f59c8ad3e17e4acd98")
                .unwrap(),
            hex::decode("0022cfa563a5328c4fb5c8017121329e964c26ade5d167b1bd9b2ec967772b60")
                .unwrap(),
        ];

        for (nc_value, nh_value, extra_entropy_value, expected_epoch_nonce) in izip!(
            nc_values.iter(),
            nh_values.iter(),
            extra_entropy_values.iter(),
            expected_epoch_nonces.iter()
        ) {
            let nc: Hash<32> = Hash::from(nc_value.as_slice());
            let nh: Hash<32> = Hash::from(nh_value.as_slice());
            let extra_entropy = *extra_entropy_value;
            let mut epoch_nonce = EpochNonceGenerator::new(nc, nh, extra_entropy);
            let nonce = epoch_nonce.finalize().unwrap();
            assert_eq!(nonce.as_ref(), expected_epoch_nonce.as_slice());
        }
    }
}
