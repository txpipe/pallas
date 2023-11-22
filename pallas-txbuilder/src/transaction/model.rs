use pallas_addresses::Address as PallasAddress;
use pallas_crypto::{
    hash::{Hash, Hasher},
    key::ed25519,
};
use pallas_primitives::{babbage, Fragment};

use std::{collections::HashMap, ops::Deref};

use serde::{Deserialize, Serialize};

use crate::TxBuilderError;

use super::{
    AssetName, Bytes, Bytes32, Bytes64, DatumBytes, DatumHash, Hash28, PolicyId, PubKeyHash,
    PublicKey, ScriptBytes, ScriptHash, Signature, TransactionStatus, TxHash,
};

// TODO: Don't make wrapper types public
#[derive(Default, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct StagingTransaction {
    pub version: String,
    pub status: TransactionStatus,
    pub inputs: Option<Vec<Input>>,
    pub reference_inputs: Option<Vec<Input>>,
    pub outputs: Option<Vec<Output>>,
    pub fee: Option<u64>,
    pub mint: Option<MintAssets>,
    pub valid_from_slot: Option<u64>,
    pub invalid_from_slot: Option<u64>,
    pub network_id: Option<u8>,
    pub collateral_inputs: Option<Vec<Input>>,
    pub collateral_output: Option<Output>,
    pub disclosed_signers: Option<Vec<PubKeyHash>>,
    pub scripts: Option<HashMap<ScriptHash, Script>>,
    pub datums: Option<HashMap<DatumHash, DatumBytes>>,
    pub redeemers: Option<Redeemers>,
    pub script_data_hash: Option<Bytes32>,
    pub signature_amount_override: Option<u8>,
    pub change_address: Option<Address>,
    // pub certificates: TODO
    // pub withdrawals: TODO
    // pub updates: TODO
    // pub auxiliary_data: TODO
    // pub phase_2_valid: TODO
}

impl StagingTransaction {
    pub fn new() -> Self {
        Self {
            version: String::from("v1"),
            status: TransactionStatus::Staging,
            ..Default::default()
        }
    }

    pub fn input(mut self, input: Input) -> Self {
        let mut mut_txins = self.inputs.unwrap_or_default();
        mut_txins.push(input);
        self.inputs = Some(mut_txins);
        self
    }

    pub fn reference_input(mut self, input: Input) -> Self {
        let mut mut_ref_txins = self.reference_inputs.unwrap_or_default();
        mut_ref_txins.push(input);
        self.reference_inputs = Some(mut_ref_txins);
        self
    }

    pub fn output(mut self, output: Output) -> Self {
        let mut mut_txos = self.outputs.unwrap_or_default();
        mut_txos.push(output);
        self.outputs = Some(mut_txos);
        self
    }

    pub fn fee(mut self, fee: u64) -> Self {
        self.fee = Some(fee);
        self
    }

    pub fn mint_asset(
        mut self,
        policy: Hash<28>,
        name: Vec<u8>,
        amount: i64,
    ) -> Result<Self, TxBuilderError> {
        if name.len() > 32 {
            return Err(TxBuilderError::AssetNameTooLong);
        }

        let mut mint = self.mint.map(|x| x.0).unwrap_or_default();

        mint.entry(Hash28(*policy))
            .and_modify(|policy_map| {
                policy_map
                    .entry(name.clone().into())
                    .and_modify(|asset_map| {
                        *asset_map += amount;
                    })
                    .or_insert(amount);
            })
            .or_insert_with(|| {
                let mut map: HashMap<Bytes, i64> = HashMap::new();
                map.insert(name.clone().into(), amount);
                map
            });

        self.mint = Some(MintAssets(mint));

        Ok(self)
    }

    pub fn valid_from_slot(mut self, slot: u64) -> Self {
        self.valid_from_slot = Some(slot);
        self
    }

    pub fn invalid_from_slot(mut self, slot: u64) -> Self {
        self.invalid_from_slot = Some(slot);
        self
    }

    pub fn network_id(mut self, id: u8) -> Self {
        self.network_id = Some(id);
        self
    }

    pub fn collateral_input(mut self, input: Input) -> Self {
        let mut mut_collins = self.collateral_inputs.unwrap_or_default();
        mut_collins.push(input);
        self.collateral_inputs = Some(mut_collins);
        self
    }

    pub fn collateral_output(mut self, output: Output) -> Self {
        self.collateral_output = Some(output);
        self
    }

    pub fn disclosed_signer(mut self, pub_key_hash: Hash<28>) -> Self {
        let mut mut_disclosed_signers = self.disclosed_signers.unwrap_or_default();
        mut_disclosed_signers.push(Hash28(*pub_key_hash));
        self.disclosed_signers = Some(mut_disclosed_signers);
        self
    }

    pub fn script(mut self, language: ScriptKind, bytes: Vec<u8>) -> Self {
        let mut mut_scripts = self.scripts.unwrap_or_default();

        let hash = match language {
            ScriptKind::Native => Hasher::<224>::hash_tagged(bytes.as_ref(), 0),
            ScriptKind::PlutusV1 => Hasher::<224>::hash_tagged(bytes.as_ref(), 1),
            ScriptKind::PlutusV2 => Hasher::<224>::hash_tagged(bytes.as_ref(), 2),
        };

        mut_scripts.insert(
            Hash28(*hash),
            Script {
                kind: language,
                bytes: bytes.into(),
            },
        );

        self.scripts = Some(mut_scripts);
        self
    }

    pub fn datum(mut self, datum: Vec<u8>) -> Self {
        let mut mut_datums = self.datums.unwrap_or_default();

        let hash = Hasher::<256>::hash_cbor(&datum);

        mut_datums.insert(Bytes32(*hash), datum.into());
        self.datums = Some(mut_datums);
        self
    }

    pub fn add_spend_redeemer(
        mut self,
        input: Input,
        plutus_data: Vec<u8>,
        ex_units: Option<ExUnits>,
    ) -> Self {
        let mut rdmrs = self.redeemers.map(|x| x.0).unwrap_or_default();

        rdmrs.insert(
            RedeemerPurpose::Spend(input),
            (plutus_data.into(), ex_units),
        );

        self.redeemers = Some(Redeemers(rdmrs));

        self
    }

    pub fn add_mint_redeemer(
        mut self,
        policy: Hash<28>,
        plutus_data: Vec<u8>,
        ex_units: Option<ExUnits>,
    ) -> Self {
        let mut rdmrs = self.redeemers.map(|x| x.0).unwrap_or_default();

        rdmrs.insert(
            RedeemerPurpose::Mint(Hash28(*policy)),
            (plutus_data.into(), ex_units),
        );

        self.redeemers = Some(Redeemers(rdmrs));

        self
    }

    // TODO: script_data_hash computation
    pub fn script_data_hash(mut self, hash: Hash<32>) -> Self {
        self.script_data_hash = Some(Bytes32(*hash));
        self
    }
}

// TODO: Don't want our wrapper types in fields public
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Hash)]
pub struct Input {
    pub tx_hash: TxHash,
    pub txo_index: u64,
}

impl Input {
    pub fn new(tx_hash: Hash<32>, txo_index: u64) -> Self {
        Self {
            tx_hash: Bytes32(*tx_hash),
            txo_index,
        }
    }
}

// TODO: Don't want our wrapper types in fields public
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Output {
    pub address: Address,
    pub lovelace: u64,
    pub assets: Option<OutputAssets>,
    pub datum: Option<Datum>,
    pub script: Option<Script>,
}

impl Output {
    pub fn new(address: PallasAddress, lovelace: u64) -> Self {
        Self {
            address: Address(address),
            lovelace,
            assets: None,
            datum: None,
            script: None,
        }
    }

    pub fn add_asset(
        mut self,
        policy: Hash<28>,
        name: Vec<u8>,
        amount: u64,
    ) -> Result<Self, TxBuilderError> {
        if name.len() > 32 {
            return Err(TxBuilderError::AssetNameTooLong);
        }

        let mut assets = self.assets.map(|x| x.0).unwrap_or_default();

        assets
            .entry(Hash28(*policy))
            .and_modify(|policy_map| {
                policy_map
                    .entry(name.clone().into())
                    .and_modify(|asset_map| {
                        *asset_map += amount;
                    })
                    .or_insert(amount);
            })
            .or_insert_with(|| {
                let mut map: HashMap<Bytes, u64> = HashMap::new();
                map.insert(name.clone().into(), amount);
                map
            });

        self.assets = Some(OutputAssets(assets));

        Ok(self)
    }

    pub fn set_inline_datum(mut self, plutus_data: Vec<u8>) -> Self {
        self.datum = Some(Datum {
            kind: DatumKind::Inline,
            bytes: plutus_data.into(),
        });

        self
    }

    pub fn set_datum_hash(mut self, datum_hash: Hash<32>) -> Self {
        self.datum = Some(Datum {
            kind: DatumKind::Inline,
            bytes: datum_hash.to_vec().into(),
        });

        self
    }

    pub fn set_inline_script(mut self, language: ScriptKind, bytes: Vec<u8>) -> Self {
        self.script = Some(Script {
            kind: language,
            bytes: bytes.into(),
        });

        self
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct OutputAssets(HashMap<PolicyId, HashMap<AssetName, u64>>);

impl Deref for OutputAssets {
    type Target = HashMap<PolicyId, HashMap<Bytes, u64>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl OutputAssets {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn from_map(map: HashMap<PolicyId, HashMap<Bytes, u64>>) -> Self {
        Self(map)
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Default)]
pub struct MintAssets(HashMap<PolicyId, HashMap<AssetName, i64>>);

impl Deref for MintAssets {
    type Target = HashMap<PolicyId, HashMap<Bytes, i64>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl MintAssets {
    pub fn new() -> Self {
        MintAssets(HashMap::new())
    }

    pub fn from_map(map: HashMap<PolicyId, HashMap<Bytes, i64>>) -> Self {
        Self(map)
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum ScriptKind {
    Native,
    PlutusV1,
    PlutusV2,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Script {
    pub kind: ScriptKind,
    pub bytes: ScriptBytes,
}

impl Script {
    pub fn new(kind: ScriptKind, bytes: Vec<u8>) -> Self {
        Self {
            kind,
            bytes: bytes.into(),
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum DatumKind {
    Hash,
    Inline,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Datum {
    pub kind: DatumKind,
    pub bytes: DatumBytes,
}

#[derive(PartialEq, Eq, Hash, Debug)]
pub enum RedeemerPurpose {
    Spend(Input),
    Mint(PolicyId),
    // Reward TODO
    // Cert TODO
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct ExUnits {
    pub mem: u32,
    pub steps: u64,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct Redeemers(HashMap<RedeemerPurpose, (Bytes, Option<ExUnits>)>);

impl Deref for Redeemers {
    type Target = HashMap<RedeemerPurpose, (Bytes, Option<ExUnits>)>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Redeemers {
    pub fn new() -> Self {
        Redeemers(HashMap::new())
    }

    pub fn from_map(map: HashMap<RedeemerPurpose, (Bytes, Option<ExUnits>)>) -> Self {
        Self(map)
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Address(pub PallasAddress);

impl Deref for Address {
    type Target = PallasAddress;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<PallasAddress> for Address {
    fn from(value: PallasAddress) -> Self {
        Self(value)
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum BuilderEra {
    Babbage,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct BuiltTransaction {
    pub version: String,
    pub era: BuilderEra,
    pub status: TransactionStatus,
    pub tx_hash: TxHash,
    pub tx_bytes: Bytes,
    pub signatures: Option<HashMap<PublicKey, Signature>>,
}

impl BuiltTransaction {
    pub fn sign(mut self, secret_key: ed25519::SecretKey) -> Result<Self, TxBuilderError> {
        let pubkey: [u8; 32] = secret_key
            .public_key()
            .as_ref()
            .try_into()
            .map_err(|_| TxBuilderError::MalformedPrivateKey)?;

        let signature: [u8; 64] = secret_key.sign(self.tx_hash.0).as_ref().try_into().unwrap();

        match self.era {
            BuilderEra::Babbage => {
                let mut new_sigs = self.signatures.unwrap_or_default();

                new_sigs.insert(Bytes32(pubkey), Bytes64(signature));

                self.signatures = Some(new_sigs);

                // TODO: chance for serialisation round trip issues?
                let mut tx = babbage::Tx::decode_fragment(&self.tx_hash.0)
                    .map_err(|_| TxBuilderError::CorruptedTxBytes)?;

                let mut vkey_witnesses = tx.transaction_witness_set.vkeywitness.unwrap_or_default();

                vkey_witnesses.push(babbage::VKeyWitness {
                    vkey: Vec::from(pubkey.as_ref()).into(),
                    signature: Vec::from(signature.as_ref()).into(),
                });

                tx.transaction_witness_set.vkeywitness = Some(vkey_witnesses);

                self.tx_bytes = tx.encode_fragment().unwrap().into();
            }
        }

        Ok(self)
    }
}