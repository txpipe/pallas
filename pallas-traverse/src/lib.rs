//! A read-only, era-agnostic view over Cardano blocks and transactions.
//!
//! Where [`pallas-primitives`] exposes the raw typed CBOR per era, this crate
//! hides the era split behind `MultiEra*` enums so a single piece of
//! indexing or analysis code can run against everything from Byron to
//! Conway.
//!
//! This is the read side of the ledger. For transaction construction see
//! [`pallas-txbuilder`]; for ledger-rule validation see [`pallas-validate`].
//!
//! [`pallas-primitives`]: https://crates.io/crates/pallas-primitives
//! [`pallas-txbuilder`]: https://crates.io/crates/pallas-txbuilder
//! [`pallas-validate`]: https://crates.io/crates/pallas-validate
//!
//! # Usage
//!
//! ```no_run
//! use pallas_traverse::MultiEraBlock;
//!
//! # let cbor_bytes: Vec<u8> = vec![];
//! let block = MultiEraBlock::decode(&cbor_bytes)?;
//!
//! println!("era={:?} slot={} hash={}", block.era(), block.slot(), block.hash());
//!
//! for tx in block.txs() {
//!     for output in tx.outputs() {
//!         println!("  → {} lovelace", output.lovelace_amount());
//!     }
//! }
//! # Ok::<_, Box<dyn std::error::Error>>(())
//! ```
//!
//! # Overview
//!
//! - [`MultiEraBlock`], [`MultiEraTx`], [`MultiEraHeader`] — top-level entry
//!   points with `decode` / `decode_for_era` constructors.
//! - [`MultiEraInput`], [`MultiEraOutput`], [`MultiEraValue`],
//!   [`MultiEraAsset`], [`MultiEraPolicyAssets`] — per-piece views.
//! - [`MultiEraCert`], [`MultiEraRedeemer`], [`MultiEraWithdrawals`],
//!   [`MultiEraSigners`], [`MultiEraMeta`], [`MultiEraUpdate`],
//!   [`MultiEraProposal`], [`MultiEraGovAction`] — the rest of the tx
//!   surface, normalised across eras.
//! - [`Era`] and [`Feature`] — discriminators for "which era is this" and
//!   "does this era support X" (multi-assets, smart contracts, CIP-1694, …).
//! - Trait-driven hashing: [`ComputeHash`] and [`OriginalHash`] give a
//!   uniform way to take Blake2b digests of typed structures.
//! - Per-aspect submodules for deeper helpers: [`block`], [`tx`], [`input`],
//!   [`output`], [`assets`], [`value`], [`cert`], [`redeemers`],
//!   [`witnesses`], [`signers`], [`hashes`], [`fees`], [`governance`],
//!   [`time`], [`header`], [`meta`], [`auxiliary`], [`probe`], [`size`],
//!   [`withdrawals`], [`wellknown`].
//!
//! # Feature flags
//!
//! - `unstable` — exposes APIs that are not yet considered stable and may
//!   change between minor releases.
//!
//! # Usage as part of `pallas`
//!
//! When depending on the umbrella [`pallas`] crate, this crate is re-exported
//! as `pallas::ledger::traverse`.
//!
//! [`pallas`]: https://crates.io/crates/pallas

use pallas_codec::utils::NonZeroInt;
use pallas_codec::utils::PositiveCoin;
use std::{borrow::Cow, collections::BTreeMap, fmt::Display, hash::Hash as StdHash};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use pallas_codec::utils::KeepRaw;
use pallas_crypto::hash::Hash;
use pallas_primitives::{alonzo, babbage, byron, conway};

mod support;

/// Helpers for inspecting native and Plutus assets inside outputs and mints.
pub mod assets;
/// Helpers for transaction auxiliary data (metadata, native scripts, plutus scripts).
pub mod auxiliary;
/// Block-level traversal: era detection, header access, transaction iteration.
pub mod block;
/// Helpers for inspecting on-chain certificates across eras.
pub mod cert;
/// Helpers for Cardano ledger eras and feature gating.
pub mod era;
/// Fee inspection and computation helpers.
pub mod fees;
/// Helpers for Conway governance actions, votes, and proposals.
pub mod governance;
/// Stable hash computation for ledger entities (`ComputeHash`, `OriginalHash`).
pub mod hashes;
/// Block-header traversal across era-specific header shapes.
pub mod header;
/// Helpers for transaction inputs across eras.
pub mod input;
/// Helpers for transaction metadata.
pub mod meta;
/// Helpers for transaction outputs across eras.
pub mod output;
/// Era detection by probing CBOR shape.
pub mod probe;
/// Helpers for Plutus redeemers.
pub mod redeemers;
/// Helpers for required-signer hashes.
pub mod signers;
/// Size accounting helpers for transactions and blocks.
pub mod size;
/// Slot / epoch / wall-clock conversion helpers.
pub mod time;
/// Transaction-level traversal: inputs, outputs, witnesses, etc.
pub mod tx;
/// Helpers for protocol-parameter update proposals.
pub mod update;
/// Helpers for ada plus multi-asset values across eras.
pub mod value;
/// Helpers for reward-account withdrawals.
pub mod withdrawals;
/// Helpers for Plutus and native witnesses.
pub mod witnesses;

// TODO: move to genesis crate
/// Well-known genesis hashes and parameter snapshots (will move to a genesis crate).
pub mod wellknown;

/// The Cardano ledger eras in chronological order.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Era {
    /// The original era (no staking, no multi-assets).
    Byron,
    /// Introduces staking, delegation, and reward accounting.
    Shelley,
    /// Adds time-locked native scripts.
    Allegra,
    /// Adds native multi-asset tokens.
    Mary,
    /// Adds Plutus V1 smart contracts.
    Alonzo,
    /// Adds CIP-31 reference inputs, CIP-32 inline datums, CIP-33 reference scripts.
    Babbage,
    /// Adds CIP-1694 on-chain governance.
    Conway,
}

/// Feature flags individual eras can be queried for.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Feature {
    /// Time-locked native scripts (Allegra+).
    TimeLocks,
    /// Native multi-asset tokens (Mary+).
    MultiAssets,
    /// Staking and reward accounting (Shelley+).
    Staking,
    /// Plutus smart contracts (Alonzo+).
    SmartContracts,
    /// Reference inputs (Babbage+).
    CIP31,
    /// Inline datums (Babbage+).
    CIP32,
    /// Reference scripts (Babbage+).
    CIP33,
    /// On-chain governance (Conway+).
    CIP1694,
}

/// A block header normalized across eras, keeping access to its raw CBOR.
#[derive(Debug)]
pub enum MultiEraHeader<'b> {
    /// Byron epoch-boundary block header.
    EpochBoundary(Cow<'b, KeepRaw<'b, byron::EbbHead>>),
    /// Shelley / Allegra / Mary / Alonzo header (all share one shape).
    ShelleyCompatible(Cow<'b, KeepRaw<'b, alonzo::Header>>),
    /// Babbage / Conway header.
    BabbageCompatible(Cow<'b, KeepRaw<'b, babbage::Header>>),
    /// Byron main block header.
    Byron(Cow<'b, KeepRaw<'b, byron::BlockHead>>),
}

/// A block normalized across eras.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum MultiEraBlock<'b> {
    /// Byron epoch-boundary block.
    EpochBoundary(Box<byron::EbBlock<'b>>),
    /// Block of any Alonzo-compatible era (Shelley/Allegra/Mary/Alonzo).
    AlonzoCompatible(Box<alonzo::Block<'b>>, Era),
    /// Babbage block.
    Babbage(Box<babbage::Block<'b>>),
    /// Byron main block.
    Byron(Box<byron::Block<'b>>),
    /// Conway block.
    Conway(Box<conway::Block<'b>>),
}

/// A transaction normalized across eras.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum MultiEraTx<'b> {
    /// Transaction of any Alonzo-compatible era (Shelley/Allegra/Mary/Alonzo).
    AlonzoCompatible(Box<Cow<'b, alonzo::Tx<'b>>>, Era),
    /// Babbage transaction.
    Babbage(Box<Cow<'b, babbage::Tx<'b>>>),
    /// Byron transaction payload.
    Byron(Box<Cow<'b, byron::TxPayload<'b>>>),
    /// Conway transaction.
    Conway(Box<Cow<'b, conway::Tx<'b>>>),
}

/// Ada-plus-multi-asset value normalized across eras.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum MultiEraValue<'b> {
    /// Byron value (lovelace only).
    Byron(u64),
    /// Value from any Alonzo-compatible era (lovelace + optional Mary assets).
    AlonzoCompatible(Cow<'b, alonzo::Value>),
    /// Conway value (uses [`PositiveCoin`] for token quantities).
    Conway(Cow<'b, conway::Value>),
}

/// Transaction output normalized across eras.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum MultiEraOutput<'b> {
    /// Output of any Alonzo-compatible era.
    AlonzoCompatible(Box<Cow<'b, alonzo::TransactionOutput>>, Era),
    /// Babbage output (supports inline datums and reference scripts).
    Babbage(Box<Cow<'b, babbage::TransactionOutput<'b>>>),
    /// Conway output.
    Conway(Box<Cow<'b, conway::TransactionOutput<'b>>>),
    /// Byron output.
    Byron(Box<Cow<'b, byron::TxOut>>),
}

/// Transaction input normalized across eras.
#[derive(Debug, Clone, PartialEq, Eq, StdHash)]
#[non_exhaustive]
pub enum MultiEraInput<'b> {
    /// Byron transaction input.
    Byron(Box<Cow<'b, byron::TxIn>>),
    /// Input of any Alonzo-compatible or later era.
    AlonzoCompatible(Box<Cow<'b, alonzo::TransactionInput>>),
}

/// On-chain certificate normalized across eras.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum MultiEraCert<'b> {
    /// Era does not carry certificates (Byron).
    NotApplicable,
    /// Certificate of any Alonzo-compatible or Babbage era.
    AlonzoCompatible(Box<Cow<'b, alonzo::Certificate>>),
    /// Conway-era certificate (adds governance-related variants).
    Conway(Box<Cow<'b, conway::Certificate>>),
}

/// Plutus redeemer normalized across eras.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum MultiEraRedeemer<'b> {
    /// Redeemer of any Alonzo-compatible or Babbage era.
    AlonzoCompatible(Box<Cow<'b, alonzo::Redeemer>>),
    /// Conway redeemer, split into key (tag + pointer) and value (data + ex-units).
    Conway(
        Box<Cow<'b, conway::RedeemersKey>>,
        Box<Cow<'b, conway::RedeemersValue>>,
    ),
}

/// Transaction metadata normalized across eras.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub enum MultiEraMeta<'b> {
    /// Metadata field present but empty.
    #[default]
    Empty,
    /// Era does not carry metadata (Byron).
    NotApplicable,
    /// Metadata from any Shelley-or-later era.
    AlonzoCompatible(&'b alonzo::Metadata),
}

/// Multi-asset bundle for a single policy, normalized across eras.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum MultiEraPolicyAssets<'b> {
    /// Mint/burn bundle from an Alonzo-compatible or Babbage era (signed quantities).
    AlonzoCompatibleMint(&'b alonzo::PolicyId, &'b BTreeMap<alonzo::AssetName, i64>),
    /// Output bundle from an Alonzo-compatible or Babbage era (unsigned quantities).
    AlonzoCompatibleOutput(&'b alonzo::PolicyId, &'b BTreeMap<alonzo::AssetName, u64>),
    /// Mint/burn bundle from the Conway era (non-zero signed quantities).
    ConwayMint(
        &'b alonzo::PolicyId,
        &'b BTreeMap<alonzo::AssetName, NonZeroInt>,
    ),
    /// Output bundle from the Conway era (strictly positive quantities).
    ConwayOutput(
        &'b alonzo::PolicyId,
        &'b BTreeMap<alonzo::AssetName, PositiveCoin>,
    ),
}

/// A single native or Plutus asset, normalized across eras.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum MultiEraAsset<'b> {
    /// Asset appearing in an Alonzo-compatible or Babbage output.
    AlonzoCompatibleOutput(&'b alonzo::PolicyId, &'b alonzo::AssetName, u64),
    /// Asset appearing in an Alonzo-compatible or Babbage mint field.
    AlonzoCompatibleMint(&'b alonzo::PolicyId, &'b alonzo::AssetName, i64),
    /// Asset appearing in a Conway output.
    ConwayOutput(&'b alonzo::PolicyId, &'b alonzo::AssetName, PositiveCoin),
    /// Asset appearing in a Conway mint field.
    ConwayMint(&'b alonzo::PolicyId, &'b alonzo::AssetName, NonZeroInt),
}

/// Reward-account withdrawals normalized across eras.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum MultiEraWithdrawals<'b> {
    /// Era does not carry withdrawals (Byron).
    NotApplicable,
    /// Withdrawals field present but empty.
    Empty,
    /// Withdrawals from any Alonzo-compatible or Babbage transaction.
    AlonzoCompatible(&'b alonzo::Withdrawals),
    /// Withdrawals from a Conway transaction.
    Conway(&'b conway::Withdrawals),
}

/// Protocol-parameter update proposal normalized across eras.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum MultiEraUpdate<'b> {
    /// Byron update proposal, tagged with the proposing epoch.
    Byron(u64, Box<Cow<'b, byron::UpProp>>),
    /// Update from any Alonzo-compatible era.
    AlonzoCompatible(Box<Cow<'b, alonzo::Update>>),
    /// Babbage update.
    Babbage(Box<Cow<'b, babbage::Update>>),
    /// Conway update.
    Conway(Box<Cow<'b, conway::Update>>),
}

/// Conway-era governance proposal procedure.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum MultiEraProposal<'b> {
    /// Conway proposal procedure.
    Conway(Box<Cow<'b, conway::ProposalProcedure>>),
}

/// Conway-era governance action carried by a [`MultiEraProposal`].
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum MultiEraGovAction<'b> {
    /// Conway governance action.
    Conway(Box<Cow<'b, conway::GovAction>>),
}

/// Required-signer hashes normalized across eras.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub enum MultiEraSigners<'b> {
    /// Era does not carry a required-signers field (Byron / Shelley / Allegra / Mary).
    NotApplicable,
    /// Required-signers field present but empty.
    #[default]
    Empty,
    /// Required signers from any Alonzo-compatible or later transaction.
    AlonzoCompatible(&'b alonzo::RequiredSigners),
}

/// Reference to a transaction output by transaction hash and output index.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct OutputRef(Hash<32>, u64);

/// Errors produced while traversing or decoding multi-era data.
#[derive(Debug, Error)]
pub enum Error {
    /// CBOR did not parse into the expected era-specific shape.
    #[error("Invalid CBOR structure: {0}")]
    InvalidCbor(String),

    /// CBOR did not match any known era shape.
    #[error("Unknown CBOR structure: {0}")]
    UnknownCbor(String),

    /// Era tag is not one this crate knows how to handle.
    #[error("Unknown era tag: {0}")]
    UnknownEra(u16),

    /// Operation requested in an era that does not support it.
    #[error("Invalid era for request: {0}")]
    InvalidEra(Era),

    /// String could not be parsed as a UTxO reference (`<tx_hash>#<index>`).
    #[error("Invalid UTxO ref: {0}")]
    InvalidUtxoRef(String),
}

impl Error {
    /// Construct an [`Error::InvalidCbor`] from any displayable error.
    pub fn invalid_cbor(error: impl Display) -> Self {
        Error::InvalidCbor(format!("{error}"))
    }

    /// Construct an [`Error::UnknownCbor`] from the offending bytes.
    pub fn unknown_cbor(bytes: &[u8]) -> Self {
        Error::UnknownCbor(hex::encode(bytes))
    }

    /// Construct an [`Error::InvalidUtxoRef`] from the offending string.
    pub fn invalid_utxo_ref(str: &str) -> Self {
        Error::InvalidUtxoRef(str.to_owned())
    }
}

/// Recompute the hash of a value from its current in-memory shape.
pub trait ComputeHash<const BYTES: usize> {
    /// Compute the hash from the current value (may differ from the on-wire hash if the value was modified).
    fn compute_hash(&self) -> pallas_crypto::hash::Hash<BYTES>;
}

/// Recover the hash that the value had on the wire, preserving any encoding quirks.
pub trait OriginalHash<const BYTES: usize> {
    /// Return the hash as computed over the value's original CBOR bytes.
    fn original_hash(&self) -> pallas_crypto::hash::Hash<BYTES>;
}
