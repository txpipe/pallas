use bech32::{FromBase32, ToBase32};
use bip39::rand_core::{CryptoRng, RngCore};
use bip39::{Language, Mnemonic};
use cryptoxide::{hmac::Hmac, pbkdf2::pbkdf2, sha2::Sha512};
use ed25519_bip32::{self, XPrv, XPub, XPRV_SIZE};
use pallas_crypto::key::ed25519::{self, SecretKeyExtended};

use crate::{Error, PrivateKey};

/// Ed25519-BIP32 HD Private Key
#[derive(Debug, PartialEq, Eq)]
pub struct Bip32PrivateKey(ed25519_bip32::XPrv);

impl Bip32PrivateKey {
    const BECH32_HRP: &'static str = "xprv";

    pub fn generate<T: RngCore + CryptoRng>(mut rng: T) -> Self {
        let mut buf = [0u8; XPRV_SIZE];
        rng.fill_bytes(&mut buf);
        let xprv = XPrv::normalize_bytes_force3rd(buf);

        Self(xprv)
    }

    pub fn generate_with_mnemonic<T: RngCore + CryptoRng>(
        mut rng: T,
        password: String,
    ) -> (Self, Mnemonic) {
        let mut buf = [0u8; 64];
        rng.fill_bytes(&mut buf);

        let bip39 = Mnemonic::generate_in_with(&mut rng, Language::English, 24).unwrap();

        let entropy = bip39.clone().to_entropy();

        let mut pbkdf2_result = [0; XPRV_SIZE];

        const ITER: u32 = 4096; // TODO: BIP39 says 2048, CML uses 4096?

        let mut mac = Hmac::new(Sha512::new(), password.as_bytes());
        pbkdf2(&mut mac, &entropy, ITER, &mut pbkdf2_result);

        (Self(XPrv::normalize_bytes_force3rd(pbkdf2_result)), bip39)
    }

    pub fn from_bytes(bytes: [u8; 96]) -> Result<Self, Error> {
        XPrv::from_bytes_verified(bytes)
            .map(Self)
            .map_err(Error::Xprv)
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        self.0.as_ref().to_vec()
    }

    pub fn from_bip39_mnenomic(mnemonic: String, password: String) -> Result<Self, Error> {
        let bip39 = Mnemonic::parse(mnemonic).map_err(Error::Mnemonic)?;
        let entropy = bip39.to_entropy();

        let mut pbkdf2_result = [0; XPRV_SIZE];

        const ITER: u32 = 4096; // TODO: BIP39 says 2048, CML uses 4096?

        let mut mac = Hmac::new(Sha512::new(), password.as_bytes());
        pbkdf2(&mut mac, &entropy, ITER, &mut pbkdf2_result);

        Ok(Self(XPrv::normalize_bytes_force3rd(pbkdf2_result)))
    }

    pub fn derive(&self, index: u32) -> Self {
        Self(self.0.derive(ed25519_bip32::DerivationScheme::V2, index))
    }

    pub fn to_ed25519_private_key(&self) -> PrivateKey {
        PrivateKey::Extended(unsafe {
            // The use of unsafe is allowed here. The key is an Extended Secret Key
            // already because it passed through the ed25519_bip32 crates checks
            SecretKeyExtended::from_bytes_unchecked(self.0.extended_secret_key())
        })
    }

    pub fn to_public(&self) -> Bip32PublicKey {
        Bip32PublicKey(self.0.public())
    }

    pub fn chain_code(&self) -> [u8; 32] {
        *self.0.chain_code()
    }

    pub fn to_bech32(&self) -> String {
        bech32::encode(
            Self::BECH32_HRP,
            self.as_bytes().to_base32(),
            bech32::Variant::Bech32,
        )
        .unwrap()
    }

    pub fn from_bech32(bech32: String) -> Result<Self, Error> {
        let (hrp, data, _) = bech32::decode(&bech32).map_err(Error::InvalidBech32)?;
        if hrp != Self::BECH32_HRP {
            Err(Error::InvalidBech32Hrp)
        } else {
            let data = Vec::<u8>::from_base32(&data).map_err(Error::InvalidBech32)?;
            Self::from_bytes(data.try_into().map_err(|_| Error::UnexpectedBech32Length)?)
        }
    }
}

/// Ed25519-BIP32 HD Public Key
#[derive(Debug, PartialEq, Eq)]
pub struct Bip32PublicKey(ed25519_bip32::XPub);

impl Bip32PublicKey {
    const BECH32_HRP: &'static str = "xpub";

    pub fn from_bytes(bytes: [u8; 64]) -> Self {
        Self(XPub::from_bytes(bytes))
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        self.0.as_ref().to_vec()
    }

    pub fn derive(&self, index: u32) -> Result<Self, Error> {
        self.0
            .derive(ed25519_bip32::DerivationScheme::V2, index)
            .map(Self)
            .map_err(Error::DerivationError)
    }

    pub fn to_ed25519_pubkey(&self) -> ed25519::PublicKey {
        self.0.public_key().into()
    }

    pub fn chain_code(&self) -> [u8; 32] {
        *self.0.chain_code()
    }

    pub fn to_bech32(&self) -> String {
        bech32::encode(
            Self::BECH32_HRP,
            self.as_bytes().to_base32(),
            bech32::Variant::Bech32,
        )
        .unwrap()
    }

    pub fn from_bech32(bech32: String) -> Result<Self, Error> {
        let (hrp, data, _) = bech32::decode(&bech32).map_err(Error::InvalidBech32)?;
        if hrp != Self::BECH32_HRP {
            Err(Error::InvalidBech32Hrp)
        } else {
            let data = Vec::<u8>::from_base32(&data).map_err(Error::InvalidBech32)?;
            Ok(Self::from_bytes(
                data.try_into().map_err(|_| Error::UnexpectedBech32Length)?,
            ))
        }
    }
}

#[cfg(test)]
mod test {
    use bip39::rand_core::OsRng;

    use super::{Bip32PrivateKey, Bip32PublicKey};

    #[test]
    fn mnemonic_roundtrip() {
        let (xprv, mne) = Bip32PrivateKey::generate_with_mnemonic(OsRng, "".into());

        let xprv_from_mne =
            Bip32PrivateKey::from_bip39_mnenomic(mne.to_string(), "".into()).unwrap();

        assert_eq!(xprv, xprv_from_mne)
    }

    #[test]
    fn bech32_roundtrip() {
        let xprv = Bip32PrivateKey::generate(OsRng);

        let xprv_bech32 = xprv.to_bech32();

        let decoded_xprv = Bip32PrivateKey::from_bech32(xprv_bech32).unwrap();

        assert_eq!(xprv, decoded_xprv);

        let xpub = xprv.to_public();

        let xpub_bech32 = xpub.to_bech32();

        let decoded_xpub = Bip32PublicKey::from_bech32(xpub_bech32).unwrap();

        assert_eq!(xpub, decoded_xpub)
    }
}
