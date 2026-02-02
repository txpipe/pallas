//! Verifiable Random Functions (VRF) bindings
//!
//! This module re-exports Cardano-compatible VRF primitives from the `cardano-crypto`
//! crate so downstream users can generate proofs and verify outputs for both
//! draft-03 (Cardano standard) and draft-13 variants.

use cardano_crypto::common::{CryptoError, CryptoResult};
use cardano_crypto::vrf::CertifiedVrf;

pub use cardano_crypto::vrf::{
    cardano_compat::cardano_vrf_prove as prove_cardano,
    cardano_compat::cardano_vrf_verify as verify_cardano, CertifiedVrf as CertifiedOutput,
    OutputVrf as Output, VrfDraft03, VrfDraft13, VrfKeyPair, VrfProof, VrfSigningKey,
    VrfVerificationKey, DRAFT03_PROOF_SIZE, DRAFT13_PROOF_SIZE, OUTPUT_SIZE, PUBLIC_KEY_SIZE,
    SECRET_KEY_SIZE, SEED_SIZE,
};

/// Generate a draft-03 keypair from a 32-byte seed.
pub fn keypair_from_seed(seed: &[u8; SEED_SIZE]) -> VrfKeyPair {
    VrfKeyPair::generate(seed)
}

/// Produce a draft-03 VRF proof and output for the given message.
pub fn prove_draft03(sk: &VrfSigningKey, message: &[u8]) -> CryptoResult<(VrfProof, Output)> {
    let proof = VrfDraft03::prove(sk, message)?;
    let output = VrfDraft03::proof_to_hash(&proof)?;
    Ok((proof, Output::new(output)))
}

/// Verify a draft-03 VRF proof and return the output if valid.
pub fn verify_draft03(
    vk: &VrfVerificationKey,
    proof: &VrfProof,
    message: &[u8],
) -> CryptoResult<Output> {
    let output = VrfDraft03::verify(vk, proof, message)?;
    Ok(Output::new(output))
}

/// Convenience wrapper that runs draft-03 end-to-end and returns a certified output.
pub fn certify_draft03(sk: &VrfSigningKey, message: &[u8]) -> CryptoResult<CertifiedOutput> {
    CertifiedVrf::eval(sk, message)
}

/// Verify a certified draft-03 output.
pub fn verify_certified(
    vk: &VrfVerificationKey,
    certified: &CertifiedOutput,
    message: &[u8],
) -> CryptoResult<()> {
    certified.verify(vk, message)
}

/// Produce a draft-13 VRF proof and output for the given message.
pub fn prove_draft13(
    sk: &VrfSigningKey,
    message: &[u8],
) -> CryptoResult<([u8; cardano_crypto::vrf::draft13::PROOF_SIZE], Output)> {
    let proof = VrfDraft13::prove(sk, message)?;
    let output = VrfDraft13::proof_to_hash(&proof)?;
    Ok((proof, Output::new(output)))
}

/// Verify a draft-13 VRF proof and return the output if valid.
pub fn verify_draft13(
    vk: &VrfVerificationKey,
    proof: &[u8; cardano_crypto::vrf::draft13::PROOF_SIZE],
    message: &[u8],
) -> CryptoResult<Output> {
    let output = VrfDraft13::verify(vk, proof, message)?;
    Ok(Output::new(output))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draft03_roundtrip() {
        let seed = [7u8; SEED_SIZE];
        let (proof, output) =
            prove_draft03(&keypair_from_seed(&seed).signing_key, b"msg").expect("prove");
        let vk = keypair_from_seed(&seed).verification_key;
        let out2 = verify_draft03(&vk, &proof, b"msg").expect("verify");
        assert_eq!(output.as_bytes(), out2.as_bytes());
    }

    #[test]
    fn draft13_roundtrip() {
        let seed = [9u8; SEED_SIZE];
        let kp = keypair_from_seed(&seed);
        let (proof, output) = prove_draft13(&kp.signing_key, b"msg-13").expect("prove13");
        let out2 = verify_draft13(&kp.verification_key, &proof, b"msg-13").expect("verify13");
        assert_eq!(output.as_bytes(), out2.as_bytes());
    }
}

/// Error type alias for VRF operations.
pub type Error = CryptoError;
/// Result type alias for VRF operations.
pub type Result<T> = CryptoResult<T>;
