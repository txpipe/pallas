use blake2::Blake2bVar;
use blake2::digest::{Update, VariableOutput};
use bech32 :: {self, Variant, Error, ToBase32};
use hex::{self};
use std::error::Error as Err;

const DATA: &str = "asset";

pub struct AssetFingerprint {
    hash_buf : [u8;20]
}

impl AssetFingerprint {

    pub fn from_parts(policy_id: &str, asset_name: &str) -> Result<AssetFingerprint, Box<dyn Err>> {
        let mut hasher = Blake2bVar::new(20).unwrap();
        let c = format!("{}{}",policy_id,asset_name);
        let raw = hex::decode(c)?;
        hasher.update(raw.as_slice());
        let mut buf = [0u8; 20];
        hasher.finalize_variable(&mut buf)?;

        Ok(AssetFingerprint { hash_buf : buf })
    }

    pub fn finger_print(&self) -> Result<String, Error> {  
      
       bech32::encode(DATA, self.hash_buf.to_base32(), Variant::Bech32)
    }
}

#[cfg(test)]
mod tests {
    use crate::cip14::AssetFingerprint;

    #[test]
    fn finger_print_test1() {
        let af = AssetFingerprint::from_parts("7eae28af2208be856f7a119668ae52a49b73725e326dc16579dcc373", "").unwrap();
        let result = af.finger_print().unwrap();
        assert_eq!(result, "asset1rjklcrnsdzqp65wjgrg55sy9723kw09mlgvlc3");
    }


    #[test]
    fn finger_print_test2() {
        let af = AssetFingerprint::from_parts("1e349c9bdea19fd6c147626a5260bc44b71635f398b67c59881df209", "504154415445").unwrap();
        let result = af.finger_print().unwrap();
        assert_eq!(result, "asset1hv4p5tv2a837mzqrst04d0dcptdjmluqvdx9k3");
    }

}