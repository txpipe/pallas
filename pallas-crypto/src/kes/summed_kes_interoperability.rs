//! Tests to check for interoperability with Haskell function. To generate the
//! test data, available in ./data, we ran the following script using the `cardano-base`
//! implementation. Run the same code with `SumXCompactKES` to generate the data of
//! the compact version.
//!
//! ```haskell
//! import Cardano.Crypto.Seed
//! import Cardano.Crypto.DSIGN
//! import Cardano.Crypto.Hash
//! import Cardano.Crypto.KES
//!
//! import Data.Maybe (fromJust)
//! import qualified Data.ByteString.Char8 as Bytechar
//! import qualified Data.ByteString as B
//!
//!
//! main :: IO()
//! main = let
//!     seed = mkSeedFromBytes $ Bytechar.pack "test string of 32 byte of lenght"
//!     kesSk0 = genKeyKES @(Sum0KES Ed25519DSIGN) seed
//!     kesSk1 = genKeyKES @(Sum1KES Ed25519DSIGN Blake2b_256) seed
//!     kesSk =  genKeyKES @(Sum6KES Ed25519DSIGN Blake2b_256) seed
//!     kesSignature = signKES () 0 (Bytechar.pack "test message") kesSk
//!     kesSkTwoUpdate = fromJust (updateKES () kesSkOneUpdate 1)
//!     kesSkThreeUpdate = fromJust (updateKES () kesSkTwoUpdate 2)
//!     kesSkFourUpdate = fromJust (updateKES () kesSkThreeUpdate 3)
//!     kesSkFiveUpdate = fromJust (updateKES () kesSkFourUpdate 4)
//!     kesSignatureFive = signKES () 5 (Bytechar.pack "test message") kesSkFiveUpdate
//!
//!     in do
//!         B.writeFile "<PATH>/key1.bin" (rawSerialiseSignKeyKES kesSk1)
//!         B.writeFile "<PATH>/key6.bin" (rawSerialiseSignKeyKES kesSk)
//!         B.writeFile "<PATH>/key6Sig.bin" (rawSerialiseSigKES kesSignature)
//!         B.writeFile "<PATH>/key6update1.bin" (rawSerialiseSignKeyKES kesSkOneUpdate)
//!         B.writeFile "<PATH>/key6update5.bin" (rawSerialiseSignKeyKES kesSkFiveUpdate)
//!         B.writeFile "<PATH>/key6Sig5.bin" (rawSerialiseSigKES kesSignatureFive)
//! ```
//!

#[cfg(test)]
mod test {

    use crate::kes::summed_kes::*;
    use crate::kes::traits::KesSk;

    // The following function is required for parsing a Cardano Shelley formatted key
    // into a secret key in this library. Mainly, it appends the period = 0 formatted
    // as a u32.
    fn from_shelley_format<'a, KES: KesSk<'a>>(key: &'a mut Vec<u8>) -> KES {
        assert_eq!(key.len(), KES::SIZE, "Invalid input size");
        key.extend_from_slice(&[0u8; 4]);

        KES::from_bytes(key).expect("Invalid format")
    }

    #[test]
    fn haskel_depth_1() {
        // haskell generated key
        let mut h_key = include_bytes!("data/key1.bin").to_vec();
        let parsed_h_key: Sum1Kes<'_> = from_shelley_format(&mut h_key);

        let mut key_buffer = [0u8; Sum1Kes::SIZE + 4];
        let mut seed = [
            116, 101, 115, 116, 32, 115, 116, 114, 105, 110, 103, 32, 111, 102, 32, 51, 50, 32, 98,
            121, 116, 101, 32, 111, 102, 32, 108, 101, 110, 103, 104, 116,
        ];

        let (skey, _) = Sum1Kes::keygen(&mut key_buffer, &mut seed);

        assert_eq!(skey.as_bytes(), parsed_h_key.as_bytes());
    }

    #[test]
    fn haskell_depth_6() {
        let mut h_key = include_bytes!("data/key6.bin").to_vec();
        let parsed_h_key: Sum6Kes<'_> = from_shelley_format(&mut h_key);

        let mut key_buffer = [0u8; Sum6Kes::SIZE + 4];
        let mut seed = [
            116, 101, 115, 116, 32, 115, 116, 114, 105, 110, 103, 32, 111, 102, 32, 51, 50, 32, 98,
            121, 116, 101, 32, 111, 102, 32, 108, 101, 110, 103, 104, 116,
        ];
        let (mut skey, _) = Sum6Kes::keygen(&mut key_buffer, &mut seed);

        assert_eq!(skey.as_bytes(), parsed_h_key.as_bytes());

        let h_1update_key: &[u8; 608] = include_bytes!("data/key6update1.bin");
        skey.update().unwrap();
        assert_eq!(skey.as_bytes()[..Sum6Kes::SIZE], h_1update_key[..]);
    }

    #[test]
    fn haskell_signature_6() {
        // haskell generated signature
        let h_signature: &[u8; 448] = include_bytes!("data/key6Sig.bin");

        let mut key_buffer = [0u8; Sum6Kes::SIZE + 4];
        let mut seed = [
            116, 101, 115, 116, 32, 115, 116, 114, 105, 110, 103, 32, 111, 102, 32, 51, 50, 32, 98,
            121, 116, 101, 32, 111, 102, 32, 108, 101, 110, 103, 104, 116,
        ];
        let message = b"test message";
        let (skey, _) = Sum6Kes::keygen(&mut key_buffer, &mut seed);
        let signature = skey.sign(message);
        assert_eq!(&signature.to_bytes(), h_signature);
    }

    #[test]
    fn haskell_signature_6_update_5() {
        let h_signature: &[u8; 448] = include_bytes!("data/key6Sig5.bin");

        let mut key_buffer = [0u8; Sum6Kes::SIZE + 4];
        let mut seed = [
            116, 101, 115, 116, 32, 115, 116, 114, 105, 110, 103, 32, 111, 102, 32, 51, 50, 32, 98,
            121, 116, 101, 32, 111, 102, 32, 108, 101, 110, 103, 104, 116,
        ];
        let message = b"test message";
        let (mut skey, _) = Sum6Kes::keygen(&mut key_buffer, &mut seed);
        skey.update().unwrap();
        skey.update().unwrap();
        skey.update().unwrap();
        skey.update().unwrap();
        skey.update().unwrap();

        let signature = skey.sign(message);
        assert_eq!(&signature.to_bytes(), h_signature);
    }

    #[test]
    fn haskel_compact_depth_1() {
        // haskell generated key
        let mut h_key = include_bytes!("data/compactkey1.bin").to_vec();
        let parsed_h_key: Sum1CompactKes<'_> = from_shelley_format(&mut h_key);

        let mut key_buffer = [0u8; Sum1CompactKes::SIZE + 4];
        let mut seed = [
            116, 101, 115, 116, 32, 115, 116, 114, 105, 110, 103, 32, 111, 102, 32, 51, 50, 32, 98,
            121, 116, 101, 32, 111, 102, 32, 108, 101, 110, 103, 104, 116,
        ];
        let (skey, _) = Sum1CompactKes::keygen(&mut key_buffer, &mut seed);

        assert_eq!(skey.as_bytes(), parsed_h_key.as_bytes());
    }

    #[test]
    fn haskell_compact_depth_6() {
        let mut h_key = include_bytes!("data/compactkey6.bin").to_vec();
        let parsed_h_key: Sum6CompactKes<'_> = from_shelley_format(&mut h_key);

        let mut key_buffer = [0u8; Sum6CompactKes::SIZE + 4];
        let mut seed = [
            116, 101, 115, 116, 32, 115, 116, 114, 105, 110, 103, 32, 111, 102, 32, 51, 50, 32, 98,
            121, 116, 101, 32, 111, 102, 32, 108, 101, 110, 103, 104, 116,
        ];
        let (mut skey, _) = Sum6CompactKes::keygen(&mut key_buffer, &mut seed);

        assert_eq!(skey.as_bytes(), parsed_h_key.as_bytes());

        let h_1update_key: &[u8; 608] = include_bytes!("data/compactkey6update1.bin");
        skey.update().unwrap();
        assert_eq!(skey.as_bytes()[..Sum6CompactKes::SIZE], h_1update_key[..]);
    }

    #[test]
    fn haskell_compact_signature_6() {
        // haskell generated signature
        let h_signature: &[u8; 288] = include_bytes!("data/compactkey6Sig.bin");

        let mut key_buffer = [0u8; Sum6CompactKes::SIZE + 4];
        let mut seed = [
            116, 101, 115, 116, 32, 115, 116, 114, 105, 110, 103, 32, 111, 102, 32, 51, 50, 32, 98,
            121, 116, 101, 32, 111, 102, 32, 108, 101, 110, 103, 104, 116,
        ];
        let message = b"test message";
        let (skey, _) = Sum6CompactKes::keygen(&mut key_buffer, &mut seed);
        let signature = skey.sign(message);
        assert_eq!(&signature.to_bytes(), h_signature);
    }

    #[test]
    fn haskell_compact_signature_6_update_5() {
        let h_signature: &[u8; 288] = include_bytes!("data/compactkey6Sig5.bin");

        let mut key_buffer = [0u8; Sum6CompactKes::SIZE + 4];
        let mut seed = [
            116, 101, 115, 116, 32, 115, 116, 114, 105, 110, 103, 32, 111, 102, 32, 51, 50, 32, 98,
            121, 116, 101, 32, 111, 102, 32, 108, 101, 110, 103, 104, 116,
        ];
        let message = b"test message";
        let (mut skey, _) = Sum6CompactKes::keygen(&mut key_buffer, &mut seed);
        skey.update().unwrap();
        skey.update().unwrap();
        skey.update().unwrap();
        skey.update().unwrap();
        skey.update().unwrap();

        let signature = skey.sign(message);
        assert_eq!(&signature.to_bytes(), h_signature);
    }
}
