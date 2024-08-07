use thiserror::Error;
use vrf_dalek::vrf03::{PublicKey03, SecretKey03, VrfProof03};

#[derive(Error, Debug)]
pub enum Error {
    #[error("TryFromSlice {0}")]
    TryFromSlice(#[from] std::array::TryFromSliceError),

    #[error("VrfError {0}")]
    VrfError(#[from] vrf_dalek::errors::VrfError),
}

/// Sign a seed value with a vrf secret key and produce a proof signature
pub fn vrf_prove(secret_key: &[u8], seed: &[u8]) -> Result<Vec<u8>, Error> {
    let sk = SecretKey03::from_bytes(secret_key[..32].try_into()?);
    let pk = PublicKey03::from(&sk);
    let proof = VrfProof03::generate(&pk, &sk, seed);
    Ok(proof.to_bytes().to_vec())
}

/// Convert a proof signature to a hash
pub fn vrf_proof_to_hash(proof: &[u8]) -> Result<Vec<u8>, Error> {
    let proof = VrfProof03::from_bytes(proof[..80].try_into()?)?;
    Ok(proof.proof_to_hash().to_vec())
}

/// Verify a proof signature with a vrf public key. This will return a hash to compare with the original
/// signature hash, but any non-error result is considered a successful verification without needing
/// to do the extra comparison check.
pub fn vrf_verify(public_key: &[u8], signature: &[u8], seed: &[u8]) -> Result<Vec<u8>, Error> {
    let pk = PublicKey03::from_bytes(public_key.try_into()?);
    let proof = VrfProof03::from_bytes(signature.try_into()?)?;
    Ok(proof.verify(&pk, seed)?.to_vec())
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

        let vrf_skey = hex::decode("adb9c97bec60189aa90d01d113e3ef405f03477d82a94f81da926c90cd46a374e0ff2371508ac339431b50af7d69cde0f120d952bb876806d3136f9a7fda4381").unwrap();
        let vrf_vkey =
            hex::decode("e0ff2371508ac339431b50af7d69cde0f120d952bb876806d3136f9a7fda4381")
                .unwrap();

        // random seed to sign with vrf_skey
        let mut seed = [0u8; 64];
        thread_rng().fill(&mut seed);

        // create a proof signature and hash of the seed
        let proof_signature = vrf_prove(&vrf_skey, &seed).unwrap();
        let proof_hash = vrf_proof_to_hash(&proof_signature).unwrap();

        // verify the proof signature with the public vrf public key
        let verified_hash = vrf_verify(&vrf_vkey, &proof_signature, &seed).unwrap();
        assert_eq!(proof_hash, verified_hash);
    }
}
