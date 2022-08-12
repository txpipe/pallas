use std::fmt;
use std::str::FromStr;

use serde::de::{Error, Unexpected, Visitor};
use serde::{Deserialize, Deserializer, Serialize};

use super::Hash;

impl<const BYTES: usize> Serialize for Hash<BYTES> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

struct HashVisitor<const BYTES: usize> {}

impl<'de, const BYTES: usize> Visitor<'de> for HashVisitor<BYTES> {
    type Value = Hash<BYTES>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a hex string representing {} bytes", BYTES)
    }

    fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        match Hash::<BYTES>::from_str(s) {
            Ok(x) => Ok(x),
            Err(_) => Err(Error::invalid_value(Unexpected::Str(s), &self)),
        }
    }
}

impl<'de, const BYTES: usize> Deserialize<'de> for Hash<BYTES> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(HashVisitor::<BYTES> {})
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Deserialize, Serialize)]
    struct Dummy {
        hash1: Hash<28>,
        hash2: Hash<32>,
    }

    #[test]
    fn roundtrip_ok() {
        let dummy = Dummy {
            hash1: "276fd18711931e2c0e21430192dbeac0e458093cd9d1fcd7210f64b3"
                .parse()
                .unwrap(),
            hash2: "0d8d00cdd4657ac84d82f0a56067634a7adfdf43da41cb534bcaa45060973d21"
                .parse()
                .unwrap(),
        };

        let json = serde_json::to_value(&dummy).unwrap();

        let dummy2: Dummy = serde_json::from_value(json).unwrap();

        assert_eq!(&dummy.hash1, &dummy2.hash1);
        assert_eq!(&dummy.hash2, &dummy2.hash2);
    }

    #[test]
    #[should_panic]
    fn invalid_str() {
        let data = r#"
        {
            "hash1": "27",
            "hash2": "0d
        }"#;

        let _: Dummy = serde_json::from_str(data).unwrap();
    }
}
