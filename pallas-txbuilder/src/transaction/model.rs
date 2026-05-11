use pallas_addresses::Address as PallasAddress;
use pallas_crypto::{
    hash::{Hash, Hasher},
    key::ed25519,
};
use pallas_primitives::{
    conway::{self, AuxiliaryData},
    Fragment, NonEmptySet,
};

use std::{collections::HashMap, ops::Deref};

use serde::{Deserialize, Serialize};

use crate::TxBuilderError;

use super::{
    AssetName, Bytes, Bytes32, Bytes64, DatumBytes, DatumHash, Hash28, PolicyId, PubKeyHash,
    PublicKey, ScriptBytes, ScriptHash, Signature, TransactionStatus, TxHash,
};
use pallas_codec::minicbor;
// TODO: Don't make wrapper types public
/// In-progress transaction that converts to a [`BuiltTransaction`] via an era-specific build trait.
#[derive(Default, Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct StagingTransaction {
    /// Schema version of this staging document (for serialization compat).
    pub version: String,
    /// Lifecycle state of this transaction.
    pub status: TransactionStatus,
    /// Inputs consumed by this transaction.
    pub inputs: Option<Vec<Input>>,
    /// Reference inputs read but not consumed (CIP-31).
    pub reference_inputs: Option<Vec<Input>>,
    /// Outputs produced by this transaction.
    pub outputs: Option<Vec<Output>>,
    /// Fee in lovelace (must already include script execution costs).
    pub fee: Option<u64>,
    /// Tokens minted or burned by this transaction.
    pub mint: Option<MintAssets>,
    /// Earliest slot at which this transaction is valid.
    pub valid_from_slot: Option<u64>,
    /// Slot at which this transaction becomes invalid (TTL).
    pub invalid_from_slot: Option<u64>,
    /// Network id this transaction targets (0 testnet, 1 mainnet).
    pub network_id: Option<u8>,
    /// Inputs offered as collateral for phase-2 script evaluation.
    pub collateral_inputs: Option<Vec<Input>>,
    /// Output receiving leftover collateral when scripts succeed.
    pub collateral_output: Option<Output>,
    /// Required-signer key hashes hinted to script evaluation.
    pub disclosed_signers: Option<Vec<PubKeyHash>>,
    /// Native and Plutus scripts attached to the transaction, keyed by hash.
    pub scripts: Option<HashMap<ScriptHash, Script>>,
    /// Plutus datums attached to the transaction, keyed by datum hash.
    pub datums: Option<HashMap<DatumHash, DatumBytes>>,
    /// Plutus redeemers paired with their target script purpose.
    pub redeemers: Option<Redeemers>,
    /// Cached script-data hash; recomputed at build time when scripts are present.
    pub script_data_hash: Option<Bytes32>,
    /// Override for the assumed number of signatures (fee estimation).
    pub signature_amount_override: Option<u8>,
    /// Address receiving change when the builder balances the transaction.
    pub change_address: Option<Address>,
    /// Plutus language-view CBOR encodings used in the script-data hash.
    pub language_views: Option<pallas_primitives::conway::LanguageViews>,
    /// Auxiliary data (metadata, native scripts) attached to the transaction.
    pub auxiliary_data: Option<AuxiliaryData>,
    // pub certificates: TODO
    // pub withdrawals: TODO
    // pub updates: TODO
    // pub phase_2_valid: TODO
}

impl StagingTransaction {
    /// Create an empty staging transaction in the `Staging` status.
    pub fn new() -> Self {
        Self {
            version: String::from("v1"),
            status: TransactionStatus::Staging,
            ..Default::default()
        }
    }

    /// Append a consumed input.
    pub fn input(mut self, input: Input) -> Self {
        let mut txins = self.inputs.unwrap_or_default();
        txins.push(input);
        self.inputs = Some(txins);
        self
    }

    /// Remove a previously added consumed input.
    pub fn remove_input(mut self, input: Input) -> Self {
        let mut txins = self.inputs.unwrap_or_default();
        txins.retain(|x| *x != input);
        self.inputs = Some(txins);
        self
    }

    /// Append a reference input (CIP-31; read-only, not spent).
    pub fn reference_input(mut self, input: Input) -> Self {
        let mut ref_txins = self.reference_inputs.unwrap_or_default();
        ref_txins.push(input);
        self.reference_inputs = Some(ref_txins);
        self
    }

    /// Remove a previously added reference input.
    pub fn remove_reference_input(mut self, input: Input) -> Self {
        let mut ref_txins = self.reference_inputs.unwrap_or_default();
        ref_txins.retain(|x| *x != input);
        self.reference_inputs = Some(ref_txins);
        self
    }

    /// Append a produced output.
    pub fn output(mut self, output: Output) -> Self {
        let mut txouts = self.outputs.unwrap_or_default();
        txouts.push(output);
        self.outputs = Some(txouts);
        self
    }

    /// Remove the output at the given index.
    pub fn remove_output(mut self, index: usize) -> Self {
        let mut txouts = self.outputs.unwrap_or_default();
        txouts.remove(index);
        self.outputs = Some(txouts);
        self
    }

    /// Set the fee (lovelace).
    pub fn fee(mut self, fee: u64) -> Self {
        self.fee = Some(fee);
        self
    }

    /// Clear any previously set fee.
    pub fn clear_fee(mut self) -> Self {
        self.fee = None;
        self
    }

    /// Add (or accumulate) a mint/burn quantity for `(policy, name)`. Positive
    /// values mint; negative values burn. Fails if the asset name exceeds 32 bytes.
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

    /// Remove the mint/burn entry for `(policy, name)`.
    pub fn remove_mint_asset(mut self, policy: Hash<28>, name: Vec<u8>) -> Self {
        let mut mint = if let Some(mint) = self.mint {
            mint.0
        } else {
            return self;
        };

        if let Some(assets) = mint.get_mut(&Hash28(*policy)) {
            assets.remove(&name.into());
            if assets.is_empty() {
                mint.remove(&Hash28(*policy));
            }
        }

        self.mint = Some(MintAssets(mint));

        self
    }

    /// Set the earliest slot at which the transaction becomes valid.
    pub fn valid_from_slot(mut self, slot: u64) -> Self {
        self.valid_from_slot = Some(slot);
        self
    }

    /// Clear the lower validity bound.
    pub fn clear_valid_from_slot(mut self) -> Self {
        self.valid_from_slot = None;
        self
    }

    /// Set the slot at which the transaction becomes invalid (TTL).
    pub fn invalid_from_slot(mut self, slot: u64) -> Self {
        self.invalid_from_slot = Some(slot);
        self
    }

    /// Clear the TTL.
    pub fn clear_invalid_from_slot(mut self) -> Self {
        self.invalid_from_slot = None;
        self
    }

    /// Set the network id (0 = testnet, 1 = mainnet).
    pub fn network_id(mut self, id: u8) -> Self {
        self.network_id = Some(id);
        self
    }

    /// Clear the network id.
    pub fn clear_network_id(mut self) -> Self {
        self.network_id = None;
        self
    }

    /// Append a collateral input.
    pub fn collateral_input(mut self, input: Input) -> Self {
        let mut coll_ins = self.collateral_inputs.unwrap_or_default();
        coll_ins.push(input);
        self.collateral_inputs = Some(coll_ins);
        self
    }

    /// Remove a previously added collateral input.
    pub fn remove_collateral_input(mut self, input: Input) -> Self {
        let mut coll_ins = self.collateral_inputs.unwrap_or_default();
        coll_ins.retain(|x| *x != input);
        self.collateral_inputs = Some(coll_ins);
        self
    }

    /// Set the collateral-return output.
    pub fn collateral_output(mut self, output: Output) -> Self {
        self.collateral_output = Some(output);
        self
    }

    /// Clear the collateral-return output.
    pub fn clear_collateral_output(mut self) -> Self {
        self.collateral_output = None;
        self
    }

    /// Add a required-signer key hash.
    pub fn disclosed_signer(mut self, pub_key_hash: Hash<28>) -> Self {
        let mut disclosed_signers = self.disclosed_signers.unwrap_or_default();
        disclosed_signers.push(Hash28(*pub_key_hash));
        self.disclosed_signers = Some(disclosed_signers);
        self
    }

    /// Remove a previously added required-signer key hash.
    pub fn remove_disclosed_signer(mut self, pub_key_hash: Hash<28>) -> Self {
        let mut disclosed_signers = self.disclosed_signers.unwrap_or_default();
        disclosed_signers.retain(|x| *x != Hash28(*pub_key_hash));
        self.disclosed_signers = Some(disclosed_signers);
        self
    }

    /// Attach a native or Plutus script. The script is keyed by its
    /// language-tagged Blake2b-224 hash.
    pub fn script(mut self, language: ScriptKind, bytes: Vec<u8>) -> Self {
        let mut scripts = self.scripts.unwrap_or_default();

        let hash = match language {
            ScriptKind::Native => Hasher::<224>::hash_tagged(bytes.as_ref(), 0),
            ScriptKind::PlutusV1 => Hasher::<224>::hash_tagged(bytes.as_ref(), 1),
            ScriptKind::PlutusV2 => Hasher::<224>::hash_tagged(bytes.as_ref(), 2),
            ScriptKind::PlutusV3 => Hasher::<224>::hash_tagged(bytes.as_ref(), 3),
        };

        scripts.insert(
            Hash28(*hash),
            Script {
                kind: language,
                bytes: bytes.into(),
            },
        );

        self.scripts = Some(scripts);
        self
    }

    /// Remove the script with the given hash.
    pub fn remove_script_by_hash(mut self, script_hash: Hash<28>) -> Self {
        let mut scripts = self.scripts.unwrap_or_default();

        scripts.remove(&Hash28(*script_hash));

        self.scripts = Some(scripts);
        self
    }

    /// Attach a Plutus datum keyed by its Blake2b-256 hash.
    pub fn datum(mut self, datum: Vec<u8>) -> Self {
        let mut datums = self.datums.unwrap_or_default();

        let hash = Hasher::<256>::hash_cbor(&datum);

        datums.insert(Bytes32(*hash), datum.into());
        self.datums = Some(datums);
        self
    }

    /// Remove a previously attached datum identified by its raw bytes.
    pub fn remove_datum(mut self, datum: Vec<u8>) -> Self {
        let mut datums = self.datums.unwrap_or_default();

        let hash = Hasher::<256>::hash_cbor(&datum);

        datums.remove(&Bytes32(*hash));
        self.datums = Some(datums);
        self
    }

    /// Remove a previously attached datum identified by its hash.
    pub fn remove_datum_by_hash(mut self, datum_hash: Hash<32>) -> Self {
        let mut datums = self.datums.unwrap_or_default();

        datums.remove(&Bytes32(*datum_hash));
        self.datums = Some(datums);
        self
    }

    /// Replace the Plutus language-views map used to compute the script-data hash.
    pub fn language_views(mut self, views: pallas_primitives::conway::LanguageViews) -> Self {
        self.language_views = Some(views);
        self
    }

    /// Add or replace the cost model for a single Plutus language version.
    /// Native scripts are a no-op.
    pub fn add_language(mut self, plutus_version: ScriptKind, cost_model: Vec<i64>) -> Self {
        let version = match plutus_version {
            ScriptKind::PlutusV1 => 0,
            ScriptKind::PlutusV2 => 1,
            ScriptKind::PlutusV3 => 2,
            ScriptKind::Native => return self,
        };
        let mut map = self
            .language_views
            .as_ref()
            .map(|v| v.0.clone())
            .unwrap_or_default();
        map.insert(version, cost_model);
        self.language_views = Some(pallas_primitives::conway::LanguageViews(map));
        self
    }

    /// Attach a spend redeemer targeting `input`. Pass `ex_units = None` to
    /// have the builder compute them later.
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

    /// Remove the spend redeemer targeting `input`.
    pub fn remove_spend_redeemer(mut self, input: Input) -> Self {
        let mut rdmrs = self.redeemers.map(|x| x.0).unwrap_or_default();

        rdmrs.remove(&RedeemerPurpose::Spend(input));

        self.redeemers = Some(Redeemers(rdmrs));

        self
    }

    /// Attach a mint redeemer targeting `policy`. Pass `ex_units = None` to
    /// have the builder compute them later.
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

    /// Remove the mint redeemer targeting `policy`.
    pub fn remove_mint_redeemer(mut self, policy: Hash<28>) -> Self {
        let mut rdmrs = self.redeemers.map(|x| x.0).unwrap_or_default();

        rdmrs.remove(&RedeemerPurpose::Mint(Hash28(*policy)));

        self.redeemers = Some(Redeemers(rdmrs));

        self
    }

    /// Override the assumed signature count used in fee estimation.
    pub fn signature_amount_override(mut self, amount: u8) -> Self {
        self.signature_amount_override = Some(amount);
        self
    }

    /// Clear the signature-count override.
    pub fn clear_signature_amount_override(mut self) -> Self {
        self.signature_amount_override = None;
        self
    }

    /// Set the address receiving change when the builder balances the transaction.
    pub fn change_address(mut self, address: PallasAddress) -> Self {
        self.change_address = Some(Address(address));
        self
    }

    /// Clear the change address.
    pub fn clear_change_address(mut self) -> Self {
        self.change_address = None;
        self
    }

    /// Attach auxiliary data parsed from raw CBOR. Invalid CBOR is silently ignored.
    pub fn add_auxiliary_data(mut self, data: Vec<u8>) -> Self {
        if let Ok(aux) = minicbor::decode::<AuxiliaryData>(data.as_ref()) {
            self.auxiliary_data = Some(aux);
        }
        self
    }

    /// Clear any attached auxiliary data.
    pub fn clear_auxiliary_data(mut self) -> Self {
        self.auxiliary_data = None;
        self
    }
}

// TODO: Don't want our wrapper types in fields public
/// Reference to a single transaction output (consumed or referenced).
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Hash, Clone)]
pub struct Input {
    /// Hash of the transaction that produced the output.
    pub tx_hash: TxHash,
    /// Index of the output within that transaction.
    pub txo_index: u64,
}

impl Input {
    /// Build an input from a transaction hash and output index.
    pub fn new(tx_hash: Hash<32>, txo_index: u64) -> Self {
        Self {
            tx_hash: Bytes32(*tx_hash),
            txo_index,
        }
    }
}

// TODO: Don't want our wrapper types in fields public
/// Transaction output being produced.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Output {
    /// Destination address.
    pub address: Address,
    /// Lovelace amount.
    pub lovelace: u64,
    /// Optional native-token bundle paired with the lovelace.
    pub assets: Option<OutputAssets>,
    /// Optional datum (inline data or hash) attached to the output.
    pub datum: Option<Datum>,
    /// Optional script reference attached to the output (CIP-33).
    pub script: Option<Script>,
}

impl Output {
    /// Build an output paying `lovelace` to `address`.
    pub fn new(address: PallasAddress, lovelace: u64) -> Self {
        Self {
            address: Address(address),
            lovelace,
            assets: None,
            datum: None,
            script: None,
        }
    }

    /// Add (or accumulate) a quantity of `(policy, name)` to this output.
    /// Fails if the asset name exceeds 32 bytes.
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

    /// Attach an inline datum (CIP-32) to this output.
    pub fn set_inline_datum(mut self, plutus_data: Vec<u8>) -> Self {
        self.datum = Some(Datum {
            kind: DatumKind::Inline,
            bytes: plutus_data.into(),
        });

        self
    }

    /// Attach a datum hash to this output (datum body provided separately).
    pub fn set_datum_hash(mut self, datum_hash: Hash<32>) -> Self {
        self.datum = Some(Datum {
            kind: DatumKind::Hash,
            bytes: datum_hash.to_vec().into(),
        });

        self
    }

    /// Attach an inline script reference (CIP-33) to this output.
    pub fn set_inline_script(mut self, language: ScriptKind, bytes: Vec<u8>) -> Self {
        self.script = Some(Script {
            kind: language,
            bytes: bytes.into(),
        });

        self
    }
}

/// Native-token bundle attached to a transaction output.
#[derive(PartialEq, Eq, Debug, Clone, Default)]
pub struct OutputAssets(HashMap<PolicyId, HashMap<AssetName, u64>>);

impl Deref for OutputAssets {
    type Target = HashMap<PolicyId, HashMap<Bytes, u64>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl OutputAssets {
    /// Build an [`OutputAssets`] from a pre-constructed policy → asset map.
    pub fn from_map(map: HashMap<PolicyId, HashMap<Bytes, u64>>) -> Self {
        Self(map)
    }
}

/// Mint/burn bundle attached to a transaction (signed quantities).
#[derive(PartialEq, Eq, Debug, Clone, Default)]
pub struct MintAssets(HashMap<PolicyId, HashMap<AssetName, i64>>);

impl Deref for MintAssets {
    type Target = HashMap<PolicyId, HashMap<Bytes, i64>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl MintAssets {
    /// Create an empty mint bundle.
    pub fn new() -> Self {
        MintAssets(HashMap::new())
    }

    /// Build a [`MintAssets`] from a pre-constructed policy → asset map.
    pub fn from_map(map: HashMap<PolicyId, HashMap<Bytes, i64>>) -> Self {
        Self(map)
    }
}

/// Discriminator selecting a script language.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum ScriptKind {
    /// Cardano native script (timelock and signature combinators).
    Native,
    /// Plutus V1 script.
    PlutusV1,
    /// Plutus V2 script.
    PlutusV2,
    /// Plutus V3 script.
    PlutusV3,
}

/// A native or Plutus script and its language.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Script {
    /// Script language.
    pub kind: ScriptKind,
    /// Serialized script bytes.
    pub bytes: ScriptBytes,
}

impl Script {
    /// Build a script from its language tag and bytes.
    pub fn new(kind: ScriptKind, bytes: Vec<u8>) -> Self {
        Self {
            kind,
            bytes: bytes.into(),
        }
    }
}

/// Discriminator for how a datum is referenced from an output.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum DatumKind {
    /// `bytes` is the 32-byte Blake2b-256 hash of the datum.
    Hash,
    /// `bytes` is the inline datum body (CIP-32).
    Inline,
}

/// Datum attached to an output, either by hash or inline.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Datum {
    /// Whether `bytes` holds a hash or the inline body.
    pub kind: DatumKind,
    /// Hash or inline payload, per [`DatumKind`].
    pub bytes: DatumBytes,
}

/// Target a Plutus redeemer applies to.
#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub enum RedeemerPurpose {
    /// Spend redeemer targeting a specific input.
    Spend(Input),
    /// Mint redeemer targeting a specific minting policy.
    Mint(PolicyId),
    // Reward TODO
    // Cert TODO
}

/// Plutus script execution budget.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct ExUnits {
    /// Memory units consumed.
    pub mem: u64,
    /// CPU step units consumed.
    pub steps: u64,
}

/// Plutus redeemers keyed by their purpose.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Default, Clone)]
pub struct Redeemers(HashMap<RedeemerPurpose, (Bytes, Option<ExUnits>)>);

impl Deref for Redeemers {
    type Target = HashMap<RedeemerPurpose, (Bytes, Option<ExUnits>)>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Redeemers {
    /// Build a [`Redeemers`] from a pre-constructed purpose → `(datum, ex_units)` map.
    pub fn from_map(map: HashMap<RedeemerPurpose, (Bytes, Option<ExUnits>)>) -> Self {
        Self(map)
    }
}

/// Newtype wrapper around [`pallas_addresses::Address`] usable in serde contexts.
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

/// Era the builder targeted when producing a [`BuiltTransaction`].
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum BuilderEra {
    /// Babbage-era output shape.
    Babbage,
    /// Conway-era output shape.
    Conway,
}

/// Anything that can produce Ed25519 signatures used by [`BuiltTransaction::sign`].
pub trait Ed25519Signer {
    /// Return the public key paired with this signer.
    fn public_key(&self) -> ed25519::PublicKey;
    /// Sign `msg` with this signer.
    fn sign<T: AsRef<[u8]>>(&self, msg: T) -> ed25519::Signature;
}

impl Ed25519Signer for ed25519::SecretKey {
    fn public_key(&self) -> ed25519::PublicKey {
        self.public_key()
    }

    fn sign<T: AsRef<[u8]>>(&self, msg: T) -> ed25519::Signature {
        self.sign(msg)
    }
}

impl Ed25519Signer for ed25519::SecretKeyExtended {
    fn public_key(&self) -> ed25519::PublicKey {
        self.public_key()
    }

    fn sign<T: AsRef<[u8]>>(&self, msg: T) -> ed25519::Signature {
        self.sign(msg)
    }
}

/// A fully built (and possibly signed) transaction ready to submit.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct BuiltTransaction {
    /// Schema version of this built document (matches the source [`StagingTransaction`]).
    pub version: String,
    /// Era the transaction was built for.
    pub era: BuilderEra,
    /// Lifecycle state — typically `TransactionStatus::Built`.
    pub status: TransactionStatus,
    /// Hash of the transaction body (the message signers attest to).
    pub tx_hash: TxHash,
    /// CBOR-encoded transaction bytes.
    pub tx_bytes: Bytes,
    /// Map of public-key → signature for each attached witness.
    pub signatures: Option<HashMap<PublicKey, Signature>>,
}

impl BuiltTransaction {
    /// Sign this transaction with `private_key` and embed the witness in
    /// `tx_bytes`. Calling multiple times accumulates witnesses.
    pub fn sign<K: Ed25519Signer>(mut self, private_key: &K) -> Result<Self, TxBuilderError> {
        let pubkey: [u8; 32] = private_key
            .public_key()
            .as_ref()
            .try_into()
            .map_err(|_| TxBuilderError::MalformedKey)?;

        let signature: [u8; ed25519::Signature::SIZE] = private_key
            .sign(self.tx_hash.0)
            .as_ref()
            .try_into()
            .unwrap();

        match self.era {
            BuilderEra::Conway => {
                let mut new_sigs = self.signatures.unwrap_or_default();

                new_sigs.insert(Bytes32(pubkey), Bytes64(signature));

                self.signatures = Some(new_sigs);

                // TODO: chance for serialisation round trip issues?
                let mut tx = conway::Tx::decode_fragment(&self.tx_bytes.0)
                    .map_err(|_| TxBuilderError::CorruptedTxBytes)?;

                let mut vkey_witnesses = tx
                    .transaction_witness_set
                    .vkeywitness
                    .as_ref()
                    .map(|x| x.clone().to_vec())
                    .unwrap_or_default();

                vkey_witnesses.push(conway::VKeyWitness {
                    vkey: Vec::from(pubkey.as_ref()).into(),
                    signature: Vec::from(signature.as_ref()).into(),
                });

                tx.transaction_witness_set.vkeywitness =
                    Some(NonEmptySet::from_vec(vkey_witnesses).unwrap());

                self.tx_bytes = tx.encode_fragment().unwrap().into();
            }
            _ => return Err(TxBuilderError::UnsupportedEra),
        }

        Ok(self)
    }

    /// Embed a signature produced out-of-band. Useful for HSM / hardware-wallet flows.
    pub fn add_signature(
        mut self,
        pub_key: ed25519::PublicKey,
        signature: [u8; 64],
    ) -> Result<Self, TxBuilderError> {
        match self.era {
            BuilderEra::Conway => {
                let mut new_sigs = self.signatures.unwrap_or_default();

                new_sigs.insert(
                    Bytes32(
                        pub_key
                            .as_ref()
                            .try_into()
                            .map_err(|_| TxBuilderError::MalformedKey)?,
                    ),
                    Bytes64(signature),
                );

                self.signatures = Some(new_sigs);

                // TODO: chance for serialisation round trip issues?
                let mut tx = conway::Tx::decode_fragment(&self.tx_bytes.0)
                    .map_err(|_| TxBuilderError::CorruptedTxBytes)?;

                let mut vkey_witnesses = tx
                    .transaction_witness_set
                    .vkeywitness
                    .as_ref()
                    .map(|x| x.clone().to_vec())
                    .unwrap_or_default();

                vkey_witnesses.push(conway::VKeyWitness {
                    vkey: Vec::from(pub_key.as_ref()).into(),
                    signature: Vec::from(signature.as_ref()).into(),
                });

                tx.transaction_witness_set.vkeywitness =
                    Some(NonEmptySet::from_vec(vkey_witnesses).unwrap());

                self.tx_bytes = tx.encode_fragment().unwrap().into();
            }
            _ => return Err(TxBuilderError::UnsupportedEra),
        }

        Ok(self)
    }

    /// Remove the witness attached for `pub_key`, if any.
    pub fn remove_signature(mut self, pub_key: ed25519::PublicKey) -> Result<Self, TxBuilderError> {
        match self.era {
            BuilderEra::Conway => {
                let mut new_sigs = self.signatures.unwrap_or_default();

                let pk = Bytes32(
                    pub_key
                        .as_ref()
                        .try_into()
                        .map_err(|_| TxBuilderError::MalformedKey)?,
                );

                new_sigs.remove(&pk);

                self.signatures = Some(new_sigs);

                // TODO: chance for serialisation round trip issues?
                let mut tx = conway::Tx::decode_fragment(&self.tx_bytes.0)
                    .map_err(|_| TxBuilderError::CorruptedTxBytes)?;

                let mut vkey_witnesses = tx
                    .transaction_witness_set
                    .vkeywitness
                    .as_ref()
                    .map(|x| x.clone().to_vec())
                    .unwrap_or_default();

                vkey_witnesses.retain(|x| *x.vkey != pk.0.to_vec());

                tx.transaction_witness_set.vkeywitness =
                    Some(NonEmptySet::from_vec(vkey_witnesses).unwrap());

                self.tx_bytes = tx.encode_fragment().unwrap().into();
            }
            _ => return Err(TxBuilderError::UnsupportedEra),
        }

        Ok(self)
    }
}
