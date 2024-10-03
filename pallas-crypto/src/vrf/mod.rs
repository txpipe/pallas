use crate::hash::Hash;
use thiserror::Error;
use vrf_dalek::vrf03::{PublicKey03, SecretKey03, VrfProof03};

/// error that can be returned if the verification of a [`VrfProof`] fails
/// see [`VrfProof::verify`]
///
#[derive(Error, Debug)]
#[error("VRF Proof Verification failed.")]
pub struct VerificationError(
    #[from]
    #[source]
    vrf_dalek::errors::VrfError,
);

pub const VRF_SEED_SIZE: usize = 32;
pub const VRF_PROOF_SIZE: usize = 80;
pub const VRF_PUBLIC_KEY_SIZE: usize = 32;
pub const VRF_SECRET_KEY_SIZE: usize = 32;
pub const VRF_PROOF_HASH_SIZE: usize = 64;

pub type VrfSeedBytes = [u8; VRF_SEED_SIZE];
pub type VrfProofBytes = [u8; VRF_PROOF_SIZE];
pub type VrfPublicKeyBytes = [u8; VRF_PUBLIC_KEY_SIZE];
pub type VrfSecretKeyBytes = [u8; VRF_SECRET_KEY_SIZE];
pub type VrfProofHashBytes = [u8; VRF_PROOF_HASH_SIZE];

// Wrapper for VRF secret key
pub struct VrfSecretKey {
    secret_key_03: SecretKey03,
}

// Wrapper for VRF public key
pub struct VrfPublicKey {
    public_key_03: PublicKey03,
}

// Wrapper for VRF proof
pub struct VrfProof {
    proof_03: VrfProof03,
}

// Create a VrfSecretKey from a slice
impl From<&VrfSecretKeyBytes> for VrfSecretKey {
    fn from(slice: &VrfSecretKeyBytes) -> Self {
        VrfSecretKey {
            secret_key_03: SecretKey03::from_bytes(slice),
        }
    }
}

// Create a VrfPublicKey from a slice
impl From<&VrfPublicKeyBytes> for VrfPublicKey {
    fn from(slice: &VrfPublicKeyBytes) -> Self {
        VrfPublicKey {
            public_key_03: PublicKey03::from_bytes(slice),
        }
    }
}

// Create a VrfProof from a slice
impl From<&VrfProofBytes> for VrfProof {
    fn from(slice: &VrfProofBytes) -> Self {
        VrfProof {
            proof_03: VrfProof03::from_bytes(slice).expect("Infallible"),
        }
    }
}

// Create a VrfPublicKey from a VrfSecretKey
impl From<&VrfSecretKey> for VrfPublicKey {
    fn from(secret_key: &VrfSecretKey) -> Self {
        VrfPublicKey {
            public_key_03: PublicKey03::from(&secret_key.secret_key_03),
        }
    }
}

impl VrfSecretKey {
    /// Sign a challenge message value with a vrf secret key and produce a proof signature
    pub fn prove(&self, challenge: &[u8]) -> VrfProof {
        let pk = PublicKey03::from(&self.secret_key_03);
        let proof = VrfProof03::generate(&pk, &self.secret_key_03, challenge);
        VrfProof { proof_03: proof }
    }
}

impl VrfProof {
    /// Return the created proof signature
    pub fn signature(&self) -> [u8; VRF_PROOF_SIZE] {
        self.proof_03.to_bytes()
    }

    /// Convert a proof signature to a hash
    pub fn to_hash(&self) -> Hash<VRF_PROOF_HASH_SIZE> {
        Hash::from(self.proof_03.proof_to_hash())
    }

    /// Verify a proof signature with a vrf public key. This will return a hash to compare with the original
    /// signature hash, but any non-error result is considered a successful verification without needing
    /// to do the extra comparison check.
    pub fn verify(
        &self,
        public_key: &VrfPublicKey,
        seed: &[u8],
    ) -> Result<Hash<VRF_PROOF_HASH_SIZE>, VerificationError> {
        Ok(Hash::from(
            self.proof_03.verify(&public_key.public_key_03, seed)?,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{thread_rng, Rng};

    #[test]
    fn vrf_prove_and_verify() {
        // Node operational VRF-Verification-Key: pool.vrf.vkey
        // {
        //    "type": "VrfVerificationKey_PraosVRF",
        //    "description": "VRF Verification Key",
        //    "cborHex": "5820e0ff2371508ac339431b50af7d69cde0f120d952bb876806d3136f9a7fda4381"
        // }
        //
        // Node operational VRF-Signing-Key: pool.vrf.skey
        // {
        //    "type": "VrfSigningKey_PraosVRF",
        //    "description": "VRF Signing Key",
        //    "cborHex": "5840adb9c97bec60189aa90d01d113e3ef405f03477d82a94f81da926c90cd46a374e0ff2371508ac339431b50af7d69cde0f120d952bb876806d3136f9a7fda4381"
        // }
        let raw_vrf_skey: Vec<u8> = hex::decode("adb9c97bec60189aa90d01d113e3ef405f03477d82a94f81da926c90cd46a374e0ff2371508ac339431b50af7d69cde0f120d952bb876806d3136f9a7fda4381").unwrap();
        let raw_vrf_vkey: Vec<u8> =
            hex::decode("e0ff2371508ac339431b50af7d69cde0f120d952bb876806d3136f9a7fda4381")
                .unwrap();

        let vrf_skey = VrfSecretKey::from(&raw_vrf_skey[..VRF_SECRET_KEY_SIZE].try_into().unwrap());
        let vrf_vkey =
            VrfPublicKey::from(&raw_vrf_vkey[..VRF_PUBLIC_KEY_SIZE].try_into().unwrap()
                as &[u8; VRF_PUBLIC_KEY_SIZE]);

        let calculated_vrf_vkey = VrfPublicKey::from(&vrf_skey);
        assert_eq!(
            vrf_vkey.public_key_03.as_bytes(),
            calculated_vrf_vkey.public_key_03.as_bytes()
        );

        // random challenge to sign with vrf_skey
        let mut challenge = [0u8; 64];
        thread_rng().fill(&mut challenge);

        // create a proof signature and hash of the seed
        let proof = vrf_skey.prove(&challenge);
        let proof_hash = proof.to_hash();

        // verify the proof signature with the public vrf public key
        let verified_hash = proof.verify(&vrf_vkey, &challenge).unwrap();
        assert_eq!(proof_hash, verified_hash);
    }
}
