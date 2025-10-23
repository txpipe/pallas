use core::fmt;
use std::{collections::HashMap, ops::Deref, str::FromStr};

use pallas_addresses::Address as PallasAddress;
use serde::{
    de::{self, Visitor},
    ser::SerializeMap,
    Deserialize, Deserializer, Serialize, Serializer,
};

use super::{
    model::{Address, Input, MintAssets, OutputAssets, RedeemerPurpose},
    Bytes, Bytes32, Bytes64, Hash28,
};

impl Serialize for Bytes32 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&hex::encode(self.0))
    }
}

impl<'de> Deserialize<'de> for Bytes32 {
    fn deserialize<D>(deserializer: D) -> Result<Bytes32, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(Bytes32Visitor)
    }
}

struct Bytes32Visitor;

impl Visitor<'_> for Bytes32Visitor {
    type Value = Bytes32;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("32 bytes hex encoded")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Bytes32(
            hex::decode(v)
                .map_err(|_| E::custom("invalid hex"))?
                .try_into()
                .map_err(|_| E::custom("invalid length"))?,
        ))
    }
}

impl Serialize for Hash28 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&hex::encode(self.0))
    }
}

impl<'de> Deserialize<'de> for Hash28 {
    fn deserialize<D>(deserializer: D) -> Result<Hash28, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(Hash28Visitor)
    }
}

struct Hash28Visitor;

impl Visitor<'_> for Hash28Visitor {
    type Value = Hash28;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("28 bytes hex encoded")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Hash28(
            hex::decode(v)
                .map_err(|_| E::custom("invalid hex"))?
                .try_into()
                .map_err(|_| E::custom("invalid length"))?,
        ))
    }
}

impl Serialize for Bytes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&hex::encode(&self.0))
    }
}

impl<'de> Deserialize<'de> for Bytes {
    fn deserialize<D>(deserializer: D) -> Result<Bytes, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(BytesVisitor)
    }
}

struct BytesVisitor;

impl Visitor<'_> for BytesVisitor {
    type Value = Bytes;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("bytes hex encoded")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Bytes(hex::decode(v).map_err(|_| E::custom("invalid hex"))?))
    }
}

impl Serialize for OutputAssets {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.deref().len()))?;

        for (policy, assets) in self.deref().iter() {
            let mut assets_map: HashMap<String, u64> = HashMap::new();

            for (asset, amount) in assets {
                assets_map.insert(hex::encode(&asset.0), *amount);
            }

            map.serialize_entry(policy, &assets_map)?;
        }

        map.end()
    }
}

impl<'de> Deserialize<'de> for OutputAssets {
    fn deserialize<D>(deserializer: D) -> Result<OutputAssets, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(OutputAssetsVisitor)
    }
}

struct OutputAssetsVisitor;

impl<'de> Visitor<'de> for OutputAssetsVisitor {
    type Value = OutputAssets;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(
            "map of hex encoded policy ids to map of hex encoded asset names to u64 amounts",
        )
    }

    fn visit_map<A>(self, mut access: A) -> Result<Self::Value, A::Error>
    where
        A: de::MapAccess<'de>,
    {
        let mut out_map = HashMap::new();

        while let Some((key, value)) = access.next_entry()? {
            out_map.insert(key, value);
        }

        Ok(OutputAssets::from_map(out_map))
    }
}

impl Serialize for MintAssets {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.deref().len()))?;

        for (policy, assets) in self.deref().iter() {
            let mut assets_map: HashMap<String, i64> = HashMap::new();

            for (asset, amount) in assets {
                assets_map.insert(hex::encode(&asset.0), *amount);
            }

            map.serialize_entry(policy, &assets_map)?;
        }

        map.end()
    }
}

impl<'de> Deserialize<'de> for MintAssets {
    fn deserialize<D>(deserializer: D) -> Result<MintAssets, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(MintAssetsVisitor)
    }
}

struct MintAssetsVisitor;

impl<'de> Visitor<'de> for MintAssetsVisitor {
    type Value = MintAssets;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(
            "map of hex encoded policy ids to map of hex encoded asset names to u64 amounts",
        )
    }

    fn visit_map<A>(self, mut access: A) -> Result<Self::Value, A::Error>
    where
        A: de::MapAccess<'de>,
    {
        let mut out_map = HashMap::new();

        while let Some((key, value)) = access.next_entry()? {
            out_map.insert(key, value);
        }

        Ok(MintAssets::from_map(out_map))
    }
}

impl Serialize for RedeemerPurpose {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let str = match self {
            RedeemerPurpose::Spend(Input { tx_hash, txo_index }) => {
                format!("spend:{}#{}", hex::encode(tx_hash.0), txo_index)
            }
            RedeemerPurpose::Mint(hash) => format!("mint:{}", hex::encode(hash.0)),
        };

        serializer.serialize_str(&str)
    }
}

impl<'de> Deserialize<'de> for RedeemerPurpose {
    fn deserialize<D>(deserializer: D) -> Result<RedeemerPurpose, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(RedeemerPurposeVisitor)
    }
}

struct RedeemerPurposeVisitor;

impl Visitor<'_> for RedeemerPurposeVisitor {
    type Value = RedeemerPurpose;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("'spend:{hex_txid}#{index}' or 'mint:{hex_policyid}'")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let (tag, item) = v
            .split_once(':')
            .ok_or(E::custom("invalid redeemer purpose"))?;

        match tag {
            "spend" => {
                let (hash, index) = item
                    .split_once('#')
                    .ok_or(E::custom("invalid spend redeemer item"))?;

                let tx_hash = Bytes32(
                    hex::decode(hash)
                        .map_err(|_| E::custom("invalid spend redeemer item txid hex"))?
                        .try_into()
                        .map_err(|_| E::custom("invalid spend redeemer txid len"))?,
                );
                let txo_index = index
                    .parse()
                    .map_err(|_| E::custom("invalid spend redeemer item index"))?;

                Ok(RedeemerPurpose::Spend(Input { tx_hash, txo_index }))
            }
            "mint" => {
                let hash = Hash28(
                    hex::decode(item)
                        .map_err(|_| E::custom("invalid mint redeemer item policy hex"))?
                        .try_into()
                        .map_err(|_| E::custom("invalid mint redeemer policy len"))?,
                );

                Ok(RedeemerPurpose::Mint(hash))
            }
            _ => Err(E::custom("invalid redeemer tag")),
        }
    }
}

impl Serialize for Address {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for Address {
    fn deserialize<D>(deserializer: D) -> Result<Address, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(AddressVisitor)
    }
}

struct AddressVisitor;

impl Visitor<'_> for AddressVisitor {
    type Value = Address;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("bech32 shelley address or base58 byron address")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Address(
            PallasAddress::from_str(v).map_err(|_| E::custom("invalid address"))?,
        ))
    }
}

impl Serialize for Bytes64 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&hex::encode(self.0))
    }
}

impl<'de> Deserialize<'de> for Bytes64 {
    fn deserialize<D>(deserializer: D) -> Result<Bytes64, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(Bytes64Visitor)
    }
}

struct Bytes64Visitor;

impl Visitor<'_> for Bytes64Visitor {
    type Value = Bytes64;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("64 bytes hex encoded")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Bytes64(
            hex::decode(v)
                .map_err(|_| E::custom("invalid hex"))?
                .try_into()
                .map_err(|_| E::custom("invalid length"))?,
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use pallas_addresses::Address as PallasAddress;
    use pallas_primitives::{babbage::PlutusData, Fragment, MaybeIndefArray};

    use crate::transaction::{model::*, Bytes64, DatumBytes, DatumHash, Hash28, TransactionStatus};

    use super::*;

    #[test]
    fn staging_json_roundtrip() {
        let mut datums: HashMap<DatumHash, DatumBytes> = HashMap::new();
        datums.insert(Bytes32([0; 32]), Bytes([0; 100].to_vec()));

        let tx = StagingTransaction {
            version: String::from("v1"),
            status: TransactionStatus::Staging,
            inputs: Some(
                vec![
                    Input {
                        tx_hash: Bytes32([0; 32]),
                        txo_index: 1
                    }
                ]
            ) ,
            reference_inputs: Some(vec![
                Input {
                    tx_hash: Bytes32([1; 32]),
                    txo_index: 0
                }
            ]),
            outputs: Some(vec![
                Output {
                    address: Address(PallasAddress::from_str("addr1g9ekml92qyvzrjmawxkh64r2w5xr6mg9ngfmxh2khsmdrcudevsft64mf887333adamant").unwrap()),
                    lovelace: 1337,
                    assets: Some(
                        OutputAssets::from_map(
                            vec![
                                (
                                    Hash28([0; 28]),
                                    (vec![(Bytes(vec![0]), 1337)]).into_iter().collect::<HashMap<_, _>>()
                                )
                            ].into_iter().collect::<HashMap<_, _>>()
                        )
                    ),
                    datum: Some(Datum { kind: DatumKind::Hash, bytes: Bytes([0; 32].to_vec()) }),
                    script: Some(Script { kind: ScriptKind::Native, bytes: Bytes([1; 100].to_vec()) }),
                }
            ]),
            fee: Some(1337),
            mint: Some(
                MintAssets::from_map(
                    vec![
                        (
                            Hash28([0; 28]),
                            (vec![(Bytes(vec![0]), -1337)]).into_iter().collect::<HashMap<_, _>>()
                        )
                    ].into_iter().collect::<HashMap<_, _>>()
                )
            ),
            valid_from_slot: Some(1337),
            invalid_from_slot: Some(1337),
            network_id: Some(1),
            collateral_inputs: Some(vec![
                Input {
                    tx_hash: Bytes32([2; 32]),
                    txo_index: 0
                }
            ]),
            collateral_output: Some(Output { address: Address(PallasAddress::from_str("addr1g9ekml92qyvzrjmawxkh64r2w5xr6mg9ngfmxh2khsmdrcudevsft64mf887333adamant").unwrap()), lovelace: 1337, assets: None, datum: None, script: None }),
            disclosed_signers: Some(vec![Hash28([0; 28])]),
            scripts: Some(
                vec![
                    (
                        Hash28([0; 28]),
                        Script { kind: ScriptKind::PlutusV1, bytes: Bytes([0; 100].to_vec()) }
                    )
                ].into_iter().collect::<HashMap<_, _>>()
            ),
            datums: Some(datums),
            redeemers: Some(Redeemers::from_map(vec![
                (RedeemerPurpose::Spend(Input { tx_hash: Bytes32([4; 32]), txo_index: 1 }), (Bytes(PlutusData::Array(MaybeIndefArray::Def(vec![])).encode_fragment().unwrap()), Some(ExUnits { mem: 1337, steps: 7331 }))),
                (RedeemerPurpose::Mint(Hash28([5; 28])), (Bytes(PlutusData::Array(MaybeIndefArray::Def(vec![])).encode_fragment().unwrap()), None)),
            ].into_iter().collect::<HashMap<_, _>>())),
            signature_amount_override: Some(5),
            change_address: Some(Address(PallasAddress::from_str("addr1g9ekml92qyvzrjmawxkh64r2w5xr6mg9ngfmxh2khsmdrcudevsft64mf887333adamant").unwrap())),
            script_data_hash: Some(Bytes32([0; 32])),
            language_view: Some(pallas_primitives::conway::LanguageView(1, vec![1, 2, 3])),
            auxiliary_data: None,
        };

        let serialised_tx = serde_json::to_string(&tx).unwrap();
        dbg!(&serialised_tx);

        let deserialised_tx: StagingTransaction = serde_json::from_str(&serialised_tx).unwrap();

        assert_eq!(tx, deserialised_tx)
    }

    #[test]
    fn built_json_roundtrip() {
        let tx = BuiltTransaction {
            version: "3".into(),
            status: TransactionStatus::Built,
            era: BuilderEra::Babbage,
            tx_hash: Bytes32([0; 32]),
            tx_bytes: Bytes([6; 100].to_vec()),
            signatures: Some(
                vec![(Bytes32([20; 32]), Bytes64([9; 64]))]
                    .into_iter()
                    .collect::<HashMap<_, _>>(),
            ),
        };

        let serialised_tx = serde_json::to_string(&tx).unwrap();

        let deserialised_tx: BuiltTransaction = serde_json::from_str(&serialised_tx).unwrap();

        assert_eq!(tx, deserialised_tx)
    }
}
