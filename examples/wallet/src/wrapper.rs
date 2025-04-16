use cryptoxide::chacha20poly1305::ChaCha20Poly1305;
use cryptoxide::kdf::argon2;
use pallas_crypto::key::ed25519::{SecretKey, SecretKeyExtended};
use rand::{CryptoRng, RngCore};

use crate::{Error, PrivateKey};

const ITERATIONS: u32 = 2500;
const VERSION_SIZE: usize = 1;
const SALT_SIZE: usize = 16;
const NONCE_SIZE: usize = 12;
const TAG_SIZE: usize = 16;

pub fn encrypt_private_key<Rng>(mut rng: Rng, private_key: PrivateKey, password: &String) -> Vec<u8>
where
    Rng: RngCore + CryptoRng,
{
    let salt = {
        let mut salt = [0u8; SALT_SIZE];
        rng.fill_bytes(&mut salt);
        salt
    };

    let sym_key: [u8; 32] = argon2::argon2(
        &argon2::Params::argon2d().iterations(ITERATIONS).unwrap(),
        password.as_bytes(),
        &salt,
        &[],
        &[],
    );

    let nonce = {
        let mut nonce = [0u8; NONCE_SIZE];
        rng.fill_bytes(&mut nonce);
        nonce
    };

    let mut chacha20 = ChaCha20Poly1305::new(&sym_key, &nonce, &[]);

    let data_size = private_key.len();

    let (ciphertext, ct_tag) = {
        let mut ciphertext = vec![0u8; data_size];
        let mut ct_tag = [0u8; 16];
        chacha20.encrypt(&private_key.as_bytes(), &mut ciphertext, &mut ct_tag);

        (ciphertext, ct_tag)
    };

    // (version || salt || nonce || tag || ciphertext)
    let mut out = Vec::with_capacity(VERSION_SIZE + SALT_SIZE + NONCE_SIZE + TAG_SIZE + data_size);

    out.push(1);
    out.extend_from_slice(&salt);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ct_tag);
    out.extend_from_slice(&ciphertext);

    out
}

#[allow(unused)]
pub fn decrypt_private_key(password: &String, data: Vec<u8>) -> Result<PrivateKey, Error> {
    let data_len_without_ct = VERSION_SIZE + SALT_SIZE + NONCE_SIZE + TAG_SIZE;

    let ciphertext_len = if data.len() == (data_len_without_ct + SecretKey::SIZE) {
        SecretKey::SIZE
    } else if data.len() == (data_len_without_ct + SecretKeyExtended::SIZE) {
        SecretKeyExtended::SIZE
    } else {
        return Err(Error::WrapperDataInvalidSize);
    };

    let mut cursor = 0;

    let _version = &data[cursor];
    cursor += VERSION_SIZE;

    let salt = &data[cursor..cursor + SALT_SIZE];
    cursor += SALT_SIZE;

    let nonce = &data[cursor..cursor + NONCE_SIZE];
    cursor += NONCE_SIZE;

    let tag = &data[cursor..cursor + TAG_SIZE];
    cursor += TAG_SIZE;

    let ciphertext = &data[cursor..cursor + ciphertext_len];

    let sym_key: [u8; 32] = argon2::argon2(
        &argon2::Params::argon2d().iterations(ITERATIONS).unwrap(),
        password.as_bytes(),
        salt,
        &[],
        &[],
    );

    let mut chacha20 = ChaCha20Poly1305::new(&sym_key, nonce, &[]);

    match ciphertext_len {
        SecretKey::SIZE => {
            let mut plaintext = [0u8; SecretKey::SIZE];

            if chacha20.decrypt(ciphertext, &mut plaintext, tag) {
                let secret_key: SecretKey = plaintext.into();

                Ok(secret_key.into())
            } else {
                Err(Error::WrapperDataFailedToDecrypt)
            }
        }
        SecretKeyExtended::SIZE => {
            let mut plaintext = [0u8; SecretKeyExtended::SIZE];

            if chacha20.decrypt(ciphertext, &mut plaintext, tag) {
                let secret_key = SecretKeyExtended::from_bytes(plaintext)?;

                Ok(secret_key.into())
            } else {
                Err(Error::WrapperDataFailedToDecrypt)
            }
        }
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use pallas_crypto::key::ed25519::{SecretKey, SecretKeyExtended};
    use rand::rngs::OsRng;

    use crate::{
        wrapper::{decrypt_private_key, encrypt_private_key},
        PrivateKey,
    };

    #[test]
    fn private_key_encryption_roundtrip() {
        let password = "hunter123";

        // --- standard

        let private_key = PrivateKey::Normal(SecretKey::new(OsRng));

        let private_key_bytes = private_key.as_bytes();

        let encrypted_priv_key = encrypt_private_key(OsRng, private_key, &password.into());

        let decrypted_privkey = decrypt_private_key(&password.into(), encrypted_priv_key).unwrap();

        assert_eq!(private_key_bytes, decrypted_privkey.as_bytes());

        // --- extended

        let private_key = PrivateKey::Extended(SecretKeyExtended::new(OsRng));

        let private_key_bytes = private_key.as_bytes();

        let encrypted_priv_key = encrypt_private_key(OsRng, private_key, &password.into());

        let decrypted_privkey = decrypt_private_key(&password.into(), encrypted_priv_key).unwrap();

        assert_eq!(private_key_bytes, decrypted_privkey.as_bytes())
    }
}
