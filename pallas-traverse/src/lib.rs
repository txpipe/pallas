//! A read-only, era-agnostic view over Cardano blocks and transactions.
//!
//! Where [`pallas-primitives`] exposes the raw typed CBOR per era, this crate
//! hides the era split behind `MultiEra*` enums so a single piece of
//! indexing or analysis code can run against everything from Byron to
//! Conway, and to Dijkstra with the `unstable` feature.
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
//! - [`MultiEraCert`], [`MultiEraRedeemer`], [`MultiEraRedeemerTag`],
//!   [`MultiEraWithdrawals`], [`MultiEraSigners`], [`MultiEraMeta`],
//!   [`MultiEraUpdate`], [`MultiEraProposal`], [`MultiEraGovAction`],
//!   [`MultiEraGovActionKind`], [`MultiEraParamUpdate`], [`MultiEraCostModels`],
//!   [`MultiEraScriptRef`], [`MultiEraNativeScript`], [`MultiEraNativeClause`]
//!   are the rest of the tx surface, normalised across eras.
//! - [`Era`] and [`Feature`] — discriminators for "which era is this" and
//!   "does this era support X" (multi-assets, smart contracts, CIP-1694, …).
//! - Trait-driven hashing: [`ComputeHash`] and [`OriginalHash`] give a
//!   uniform way to take Blake2b digests of typed structures.
//! - Per-aspect submodules for deeper helpers: [`block`], [`tx`], [`input`],
//!   [`output`], [`assets`], [`value`], [`cert`], [`redeemers`],
//!   [`witnesses`], [`signers`], [`hashes`], [`fees`], [`governance`],
//!   [`time`], [`header`], [`meta`], [`auxiliary`], [`probe`], [`size`],
//!   [`withdrawals`], [`wellknown`], [`script_ref`].
//!
//! # Feature flags
//!
//! - `unstable` — exposes APIs that are not yet considered stable and may
//!   change between minor releases. The Dijkstra era is behind this flag.
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

#[cfg(feature = "unstable")]
use pallas_primitives::dijkstra;

mod support;

#[cfg(test)]
mod testing;

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
/// Helpers for reference scripts attached to outputs.
pub mod script_ref;
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
///
/// `has_feature` compares variants with `ge` in declaration order, so a new
/// era is declared last.
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
    /// Adds inline block transactions, sub transactions and the Leios fields.
    #[cfg(feature = "unstable")]
    Dijkstra,
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
#[cfg_attr(feature = "unstable", non_exhaustive)]
pub enum MultiEraHeader<'b> {
    /// Byron epoch-boundary block header.
    EpochBoundary(Cow<'b, KeepRaw<'b, byron::EbbHead>>),
    /// Shelley / Allegra / Mary / Alonzo header (all share one shape).
    ShelleyCompatible(Cow<'b, KeepRaw<'b, alonzo::Header>>),
    /// Babbage / Conway header.
    BabbageCompatible(Cow<'b, KeepRaw<'b, babbage::Header>>),
    /// Byron main block header.
    Byron(Cow<'b, KeepRaw<'b, byron::BlockHead>>),
    /// Dijkstra header.
    #[cfg(feature = "unstable")]
    Dijkstra(Cow<'b, KeepRaw<'b, dijkstra::Header>>),
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
    /// Dijkstra block.
    #[cfg(feature = "unstable")]
    Dijkstra(Box<dijkstra::Block<'b>>),
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
    /// Dijkstra transaction, whose `success` flag is written last.
    #[cfg(feature = "unstable")]
    Dijkstra(Box<Cow<'b, dijkstra::BlockTransaction<'b>>>),
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
    /// Dijkstra output, whose reference script may be PlutusV4.
    #[cfg(feature = "unstable")]
    Dijkstra(Box<Cow<'b, dijkstra::TransactionOutput<'b>>>),
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
    /// Dijkstra certificate, whose pool registration may carry a `bls_key`.
    #[cfg(feature = "unstable")]
    Dijkstra(Box<Cow<'b, dijkstra::Certificate>>),
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
    /// Dijkstra redeemer, whose tag space gains `Guarding`.
    #[cfg(feature = "unstable")]
    Dijkstra(
        Box<Cow<'b, dijkstra::RedeemersKey>>,
        Box<Cow<'b, dijkstra::RedeemersValue>>,
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
    /// Dijkstra update, whose parameter update rule adds keys 34 to 48.
    #[cfg(feature = "unstable")]
    Dijkstra(Box<Cow<'b, dijkstra::Update>>),
}

/// Governance proposal procedure normalized across eras.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum MultiEraProposal<'b> {
    /// Conway proposal procedure.
    Conway(Box<Cow<'b, conway::ProposalProcedure>>),
    /// Dijkstra proposal procedure, whose action reaches this era's parameter update.
    #[cfg(feature = "unstable")]
    Dijkstra(Box<Cow<'b, dijkstra::ProposalProcedure>>),
}

/// Governance action carried by a [`MultiEraProposal`], normalized across eras.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum MultiEraGovAction<'b> {
    /// Conway governance action.
    Conway(Box<Cow<'b, conway::GovAction>>),
    /// Dijkstra governance action, whose parameter change arm carries this era's update.
    #[cfg(feature = "unstable")]
    Dijkstra(Box<Cow<'b, dijkstra::GovAction>>),
}

/// The payload a [`MultiEraGovAction`] proposes, normalized across eras.
///
/// In five variants the leading `Option<GovActionId>` names the most recently
/// enacted action of the same kind, which a proposal leaves unset when there
/// is none.
///
/// A committee update reports the credentials it removes as a slice, since
/// the two eras encode that set in different types.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum MultiEraGovActionKind<'b> {
    /// A parameter change proposes the update it carries, under the guardrails
    /// script it names.
    ParameterChange(
        Option<&'b conway::GovActionId>,
        MultiEraParamUpdate<'b>,
        Option<&'b conway::ScriptHash>,
    ),
    /// A hard fork initiation proposes the major and minor protocol version it
    /// carries.
    HardForkInitiation(Option<&'b conway::GovActionId>, &'b conway::ProtocolVersion),
    /// A treasury withdrawal proposes paying each reward account the amount it
    /// maps to, under the guardrails script it names.
    TreasuryWithdrawals(
        &'b BTreeMap<conway::RewardAccount, conway::Coin>,
        Option<&'b conway::ScriptHash>,
    ),
    /// A no confidence action proposes no confidence in the committee.
    NoConfidence(Option<&'b conway::GovActionId>),
    /// A committee update proposes removing the credentials in the slice,
    /// seating each credential in the map until the epoch it maps to, and
    /// setting the committee threshold.
    UpdateCommittee(
        Option<&'b conway::GovActionId>,
        &'b [conway::CommitteeColdCredential],
        &'b BTreeMap<conway::CommitteeColdCredential, conway::Epoch>,
        &'b conway::UnitInterval,
    ),
    /// A new constitution proposes the constitution it carries, an anchor and
    /// the guardrails script hash that constitution may name.
    NewConstitution(Option<&'b conway::GovActionId>, &'b conway::Constitution),
    /// An information action proposes nothing. The proposal that holds it
    /// carries the anchor.
    Information,
}

/// Protocol parameter update proposed by a governance action, normalized
/// across eras. Conway's rule stops at key 33, so each variant names its era.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum MultiEraParamUpdate<'b> {
    /// Update proposed by a Conway parameter change action.
    Conway(Box<Cow<'b, conway::ProtocolParamUpdate>>),
    /// Update proposed by a Dijkstra parameter change action.
    #[cfg(feature = "unstable")]
    Dijkstra(Box<Cow<'b, dijkstra::ProtocolParamUpdate>>),
}

/// The cost models a parameter update proposes, one field per Plutus language.
///
/// A language the update proposes no model for reads `None`. Conway's update
/// type has no field for PlutusV4, so a Conway update reads `None` there
/// whatever it proposes, and [`MultiEraParamUpdate::era`] says which of the
/// two cases a `None` is. A model under one of the keys the CDDL wildcard
/// permits is named by no field here and is read from the era's own update.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct MultiEraCostModels {
    /// The PlutusV1 model, under key 0 of the cost model map.
    pub plutus_v1: Option<conway::CostModel>,
    /// The PlutusV2 model, under key 1 of the cost model map.
    pub plutus_v2: Option<conway::CostModel>,
    /// The PlutusV3 model, under key 2 of the cost model map.
    pub plutus_v3: Option<conway::CostModel>,
    /// The PlutusV4 model, under key 3 of the cost model map, which Conway's
    /// type has no field for.
    pub plutus_v4: Option<conway::CostModel>,
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
    /// Dijkstra guards, which admit credentials as well as key hashes.
    #[cfg(feature = "unstable")]
    Dijkstra(&'b dijkstra::Guards),
}

/// A reference script attached to an output, normalized across eras.
///
/// The Conway variant carries a Babbage or Conway reference script. The
/// `unstable` build adds a Dijkstra variant, which may be PlutusV4.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum MultiEraScriptRef<'b> {
    /// Reference script from a Babbage or Conway output.
    Conway(Cow<'b, conway::ScriptRef<'b>>),
    /// Reference script from a Dijkstra output, which may be PlutusV4.
    #[cfg(feature = "unstable")]
    Dijkstra(Cow<'b, dijkstra::ScriptRef<'b>>),
}

/// A native script normalized across eras.
///
/// The `AlonzoCompatible` variant carries the type every era through Conway
/// shares. The `unstable` build adds a Dijkstra variant with a seventh clause.
///
/// Each variant keeps a [`KeepRaw`], so a script read from a witness set or a
/// reference script carries the bytes it arrived in and gives the hash the
/// ledger keys it by. A script read from auxiliary data carries none, because
/// the auxiliary data types hold a decoded script.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum MultiEraNativeScript<'b> {
    /// Native script from any era through Conway.
    AlonzoCompatible(Cow<'b, KeepRaw<'b, alonzo::NativeScript>>),
    /// Native script from a Dijkstra transaction, which may require a guard.
    #[cfg(feature = "unstable")]
    Dijkstra(Cow<'b, KeepRaw<'b, dijkstra::NativeScript>>),
}

/// The clause at the root of a native script, normalized across eras.
///
/// Every era through Conway names at most the first six. The `unstable` build
/// adds `RequireGuard`, the clause Dijkstra introduces.
///
/// A compound clause reports the scripts it holds in the same era variant as
/// this one. Each arrives decoded, so it carries no bytes of its own and
/// hashes its own re-encoding.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum MultiEraNativeClause<'b> {
    /// A signature by the key with this hash satisfies the script.
    Pubkey(&'b Hash<28>),
    /// Every script held satisfies the script together.
    All(Vec<MultiEraNativeScript<'b>>),
    /// Any one of the scripts held satisfies the script.
    Any(Vec<MultiEraNativeScript<'b>>),
    /// Any this many of the scripts held satisfy the script. The ledger CDDL
    /// types the threshold signed, not `uint`, so it may be negative.
    NOfK(i64, Vec<MultiEraNativeScript<'b>>),
    /// No slot before this one satisfies the script.
    InvalidBefore(u64),
    /// No slot from this one on satisfies the script.
    InvalidHereafter(u64),
    /// A guard on this credential satisfies the script, a clause new in Dijkstra.
    #[cfg(feature = "unstable")]
    RequireGuard(&'b dijkstra::StakeCredential),
}

/// The purpose a redeemer is supplied for, normalized across eras.
///
/// Every era through Conway names at most the first six. Dijkstra adds
/// `Guarding`, which no earlier era's tag space has a name for.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum MultiEraRedeemerTag {
    /// The redeemer unlocks a script address being spent.
    Spend,
    /// The redeemer authorises a mint or burn under a policy.
    Mint,
    /// The redeemer authorises a certificate.
    Cert,
    /// The redeemer authorises a reward account withdrawal.
    Reward,
    /// The redeemer authorises a governance vote.
    Vote,
    /// The redeemer authorises a governance proposal.
    Propose,
    /// The redeemer satisfies a guard, a purpose new in Dijkstra.
    #[cfg(feature = "unstable")]
    Guarding,
}

impl From<alonzo::RedeemerTag> for MultiEraRedeemerTag {
    fn from(tag: alonzo::RedeemerTag) -> Self {
        match tag {
            alonzo::RedeemerTag::Spend => Self::Spend,
            alonzo::RedeemerTag::Mint => Self::Mint,
            alonzo::RedeemerTag::Cert => Self::Cert,
            alonzo::RedeemerTag::Reward => Self::Reward,
        }
    }
}

impl From<conway::RedeemerTag> for MultiEraRedeemerTag {
    fn from(tag: conway::RedeemerTag) -> Self {
        match tag {
            conway::RedeemerTag::Spend => Self::Spend,
            conway::RedeemerTag::Mint => Self::Mint,
            conway::RedeemerTag::Cert => Self::Cert,
            conway::RedeemerTag::Reward => Self::Reward,
            conway::RedeemerTag::Vote => Self::Vote,
            conway::RedeemerTag::Propose => Self::Propose,
        }
    }
}

#[cfg(feature = "unstable")]
impl From<dijkstra::RedeemerTag> for MultiEraRedeemerTag {
    fn from(tag: dijkstra::RedeemerTag) -> Self {
        match tag {
            dijkstra::RedeemerTag::Spend => Self::Spend,
            dijkstra::RedeemerTag::Mint => Self::Mint,
            dijkstra::RedeemerTag::Cert => Self::Cert,
            dijkstra::RedeemerTag::Reward => Self::Reward,
            dijkstra::RedeemerTag::Vote => Self::Vote,
            dijkstra::RedeemerTag::Propose => Self::Propose,
            dijkstra::RedeemerTag::Guarding => Self::Guarding,
        }
    }
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

#[cfg(test)]
mod attribute_tests {
    const THIS_FILE: &str = include_str!("lib.rs");

    const ALWAYS: &str = "#[non_exhaustive]";
    const WITH_UNSTABLE: &str = "#[cfg_attr(feature = \"unstable\", non_exhaustive)]";

    /// Which builds apply `#[non_exhaustive]` to an enum.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum NonExhaustive {
        Never,
        WithUnstable,
        Always,
    }

    /// Every multi era enum this file defines with the marking it is required
    /// to carry, sorted by name.
    const MULTI_ERA_ENUMS: &[(&str, NonExhaustive)] = &[
        ("MultiEraAsset", NonExhaustive::Always),
        ("MultiEraBlock", NonExhaustive::Always),
        ("MultiEraCert", NonExhaustive::Always),
        ("MultiEraGovAction", NonExhaustive::Always),
        ("MultiEraGovActionKind", NonExhaustive::Always),
        ("MultiEraHeader", NonExhaustive::WithUnstable),
        ("MultiEraInput", NonExhaustive::Always),
        ("MultiEraMeta", NonExhaustive::Always),
        ("MultiEraNativeClause", NonExhaustive::Always),
        ("MultiEraNativeScript", NonExhaustive::Always),
        ("MultiEraOutput", NonExhaustive::Always),
        ("MultiEraParamUpdate", NonExhaustive::Always),
        ("MultiEraPolicyAssets", NonExhaustive::Always),
        ("MultiEraProposal", NonExhaustive::Always),
        ("MultiEraRedeemer", NonExhaustive::Always),
        ("MultiEraRedeemerTag", NonExhaustive::Always),
        ("MultiEraScriptRef", NonExhaustive::Always),
        ("MultiEraSigners", NonExhaustive::Always),
        ("MultiEraTx", NonExhaustive::Always),
        ("MultiEraUpdate", NonExhaustive::Always),
        ("MultiEraValue", NonExhaustive::Always),
        ("MultiEraWithdrawals", NonExhaustive::Always),
    ];

    fn multi_era_enums(source: &str) -> Vec<(String, NonExhaustive)> {
        let lines: Vec<&str> = source.lines().collect();

        lines
            .iter()
            .enumerate()
            .filter_map(|(i, line)| {
                let rest = line.trim().strip_prefix("pub enum MultiEra")?;
                let name: String = std::iter::once("MultiEra")
                    .chain(std::iter::once(
                        rest.split(['<', ' ', '{']).next().unwrap_or_default(),
                    ))
                    .collect();
                let attributes: Vec<&str> = lines[..i]
                    .iter()
                    .rev()
                    .take_while(|above| {
                        let above = above.trim();
                        above.starts_with("#[") || above.starts_with("//")
                    })
                    .map(|above| above.trim())
                    .collect();
                let marking = if attributes.contains(&ALWAYS) {
                    NonExhaustive::Always
                } else if attributes.contains(&WITH_UNSTABLE) {
                    NonExhaustive::WithUnstable
                } else {
                    NonExhaustive::Never
                };
                Some((name, marking))
            })
            .collect()
    }

    #[test]
    fn every_multi_era_enum_carries_the_marking_it_is_listed_with() {
        let mut found = multi_era_enums(THIS_FILE);
        found.sort_by(|left, right| left.0.cmp(&right.0));

        let expected: Vec<(String, NonExhaustive)> = MULTI_ERA_ENUMS
            .iter()
            .map(|(name, marking)| (name.to_string(), *marking))
            .collect();

        assert_eq!(
            found, expected,
            "a multi era enum is missing, is one the list does not name, or carries a marking other than the one it is listed with"
        );
    }

    #[test]
    fn the_attribute_scan_reads_all_three_markings() {
        let cases = [
            (
                "#[non_exhaustive]\npub enum MultiEraThing<'b> {",
                NonExhaustive::Always,
            ),
            (
                "#[non_exhaustive]\n#[derive(Debug)]\npub enum MultiEraThing<'b> {",
                NonExhaustive::Always,
            ),
            (
                "/// A thing.\n#[non_exhaustive]\n#[derive(Debug)]\npub enum MultiEraThing<'b> {",
                NonExhaustive::Always,
            ),
            (
                "#[cfg_attr(feature = \"unstable\", non_exhaustive)]\npub enum MultiEraThing<'b> {",
                NonExhaustive::WithUnstable,
            ),
            (
                "/// A thing.\n#[derive(Debug)]\n#[cfg_attr(feature = \"unstable\", non_exhaustive)]\npub enum MultiEraThing<'b> {",
                NonExhaustive::WithUnstable,
            ),
            (
                "#[derive(Debug)]\npub enum MultiEraThing<'b> {",
                NonExhaustive::Never,
            ),
        ];

        for (source, marking) in cases {
            assert_eq!(
                multi_era_enums(source),
                vec![("MultiEraThing".to_string(), marking)],
                "{source:?}"
            );
        }
    }

    /// The variants each listed enum has without the `unstable` feature. A
    /// variant added outside the gate makes one of these matches inexhaustive,
    /// so a build without the feature stops compiling.
    #[cfg(not(feature = "unstable"))]
    mod stable_variants {
        macro_rules! variants {
            ($($enum:ident => [$($variant:pat),+ $(,)?]),+ $(,)?) => {
                $(
                    #[allow(non_snake_case)]
                    fn $enum(value: &crate::$enum) {
                        match value {
                            $($variant => {}),+
                        }
                    }
                )+

                #[test]
                fn every_listed_enum_has_a_stable_variant_list() {
                    $(let _: fn(&crate::$enum) = $enum;)+

                    let mut covered = vec![$(stringify!($enum)),+];
                    covered.sort_unstable();

                    let mut listed: Vec<&str> = super::MULTI_ERA_ENUMS
                        .iter()
                        .map(|(name, _)| *name)
                        .collect();
                    listed.sort_unstable();

                    assert_eq!(
                        covered, listed,
                        "an enum the marking list names has no variant list here, or this list names one the marking list does not"
                    );
                }
            };
        }

        variants! {
            MultiEraAsset => [
                crate::MultiEraAsset::AlonzoCompatibleOutput(..),
                crate::MultiEraAsset::AlonzoCompatibleMint(..),
                crate::MultiEraAsset::ConwayOutput(..),
                crate::MultiEraAsset::ConwayMint(..),
            ],
            MultiEraBlock => [
                crate::MultiEraBlock::EpochBoundary(..),
                crate::MultiEraBlock::AlonzoCompatible(..),
                crate::MultiEraBlock::Babbage(..),
                crate::MultiEraBlock::Byron(..),
                crate::MultiEraBlock::Conway(..),
            ],
            MultiEraCert => [
                crate::MultiEraCert::NotApplicable,
                crate::MultiEraCert::AlonzoCompatible(..),
                crate::MultiEraCert::Conway(..),
            ],
            MultiEraGovAction => [
                crate::MultiEraGovAction::Conway(..),
            ],
            MultiEraHeader => [
                crate::MultiEraHeader::EpochBoundary(..),
                crate::MultiEraHeader::ShelleyCompatible(..),
                crate::MultiEraHeader::BabbageCompatible(..),
                crate::MultiEraHeader::Byron(..),
            ],
            MultiEraInput => [
                crate::MultiEraInput::Byron(..),
                crate::MultiEraInput::AlonzoCompatible(..),
            ],
            MultiEraMeta => [
                crate::MultiEraMeta::Empty,
                crate::MultiEraMeta::NotApplicable,
                crate::MultiEraMeta::AlonzoCompatible(..),
            ],
            MultiEraOutput => [
                crate::MultiEraOutput::AlonzoCompatible(..),
                crate::MultiEraOutput::Babbage(..),
                crate::MultiEraOutput::Conway(..),
                crate::MultiEraOutput::Byron(..),
            ],
            MultiEraPolicyAssets => [
                crate::MultiEraPolicyAssets::AlonzoCompatibleMint(..),
                crate::MultiEraPolicyAssets::AlonzoCompatibleOutput(..),
                crate::MultiEraPolicyAssets::ConwayMint(..),
                crate::MultiEraPolicyAssets::ConwayOutput(..),
            ],
            MultiEraProposal => [
                crate::MultiEraProposal::Conway(..),
            ],
            MultiEraRedeemer => [
                crate::MultiEraRedeemer::AlonzoCompatible(..),
                crate::MultiEraRedeemer::Conway(..),
            ],
            MultiEraSigners => [
                crate::MultiEraSigners::NotApplicable,
                crate::MultiEraSigners::Empty,
                crate::MultiEraSigners::AlonzoCompatible(..),
            ],
            MultiEraTx => [
                crate::MultiEraTx::AlonzoCompatible(..),
                crate::MultiEraTx::Babbage(..),
                crate::MultiEraTx::Byron(..),
                crate::MultiEraTx::Conway(..),
            ],
            MultiEraUpdate => [
                crate::MultiEraUpdate::Byron(..),
                crate::MultiEraUpdate::AlonzoCompatible(..),
                crate::MultiEraUpdate::Babbage(..),
                crate::MultiEraUpdate::Conway(..),
            ],
            MultiEraValue => [
                crate::MultiEraValue::Byron(..),
                crate::MultiEraValue::AlonzoCompatible(..),
                crate::MultiEraValue::Conway(..),
            ],
            MultiEraWithdrawals => [
                crate::MultiEraWithdrawals::NotApplicable,
                crate::MultiEraWithdrawals::Empty,
                crate::MultiEraWithdrawals::AlonzoCompatible(..),
                crate::MultiEraWithdrawals::Conway(..),
            ],
        }
    }
}
