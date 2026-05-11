// Material brought from `pallas-primitives`
// TODO: Refactor in order to avoid repetition.
use crate::miniprotocols::localstate::queries_v16::{
    Anchor, AssetName, Coin, DRep, Epoch, PolicyId, PoolMetadata, Relay, RewardAccount, ScriptHash,
    UnitInterval,
};
pub use pallas_codec::utils::KeyValuePairs;
pub use pallas_crypto::hash::Hash;

use pallas_codec::{
    minicbor::{self, Decode, Encode},
    utils::{Bytes, NonEmptyKeyValuePairs, Nullable, Set},
};

/// Multi-asset value: `policy → asset_name → quantity`.
pub type Multiasset<A> = NonEmptyKeyValuePairs<PolicyId, NonEmptyKeyValuePairs<AssetName, A>>;

/// Mint/burn payload (signed quantities).
pub type Mint = Multiasset<i64>;

/// On-chain credential: a script hash or a verification-key hash.
// https://github.com/IntersectMBO/cardano-ledger/blob/33e90ea03447b44a389985ca2b158568e5f4ad65/libs/cardano-ledger-core/src/Cardano/Ledger/Credential.hs#L82
#[derive(Debug, Decode, Encode, Clone, Hash, PartialEq, Eq)]
#[cbor(flat)]
pub enum Credential {
    /// Credential backed by a script hash.
    #[n(0)]
    ScriptHashObj(#[n(0)] ScriptHash),
    /// Credential backed by a verification-key hash.
    #[n(1)]
    KeyHashObj(#[n(0)] AddrKeyhash),
}

/// On-chain certificate (Shelley + Conway era variants).
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Certificate {
    /// Register a stake credential (pre-Conway).
    StakeRegistration(StakeCredential),
    /// Deregister a stake credential (pre-Conway).
    StakeDeregistration(StakeCredential),
    /// Delegate a stake credential to a stake pool.
    StakeDelegation(StakeCredential, PoolKeyhash),
    /// Register a new stake pool with its full parameter set.
    PoolRegistration {
        /// Pool operator (cold) key hash.
        operator: PoolKeyhash,
        /// Pool VRF key hash.
        vrf_keyhash: VrfKeyhash,
        /// Pledge in lovelace.
        pledge: Coin,
        /// Per-epoch fixed cost in lovelace.
        cost: Coin,
        /// Pool margin (rational in `[0, 1]`).
        margin: UnitInterval,
        /// Reward account that collects pool fees.
        reward_account: RewardAccount,
        /// Hashes of accounts that own the pool.
        pool_owners: Set<AddrKeyhash>,
        /// Declared relays serving the pool.
        relays: Vec<Relay>,
        /// Optional pool metadata pointer.
        pool_metadata: Nullable<PoolMetadata>,
    },
    /// Retire a stake pool at the given epoch.
    PoolRetirement(PoolKeyhash, Epoch),

    /// Conway: register a stake credential with explicit deposit.
    Reg(StakeCredential, Coin),
    /// Conway: deregister a stake credential, returning its deposit.
    UnReg(StakeCredential, Coin),
    /// Conway: delegate voting rights to a DRep.
    VoteDeleg(StakeCredential, DRep),
    /// Conway: delegate both stake (to a pool) and voting rights (to a DRep).
    StakeVoteDeleg(StakeCredential, PoolKeyhash, DRep),
    /// Conway: register, take a deposit, and delegate stake in one certificate.
    StakeRegDeleg(StakeCredential, PoolKeyhash, Coin),
    /// Conway: register, take a deposit, and delegate voting rights in one certificate.
    VoteRegDeleg(StakeCredential, DRep, Coin),
    /// Conway: register and delegate both stake and voting rights in one certificate.
    StakeVoteRegDeleg(StakeCredential, PoolKeyhash, DRep, Coin),

    /// Conway: authorize a hot key for a constitutional-committee member.
    AuthCommitteeHot(CommitteeColdCredential, CommitteeHotCredential),
    /// Conway: resign a constitutional-committee cold credential.
    ResignCommitteeCold(CommitteeColdCredential, Nullable<Anchor>),
    /// Conway: register a DRep with deposit and optional anchor.
    RegDRepCert(DRepCredential, Coin, Nullable<Anchor>),
    /// Conway: deregister a DRep, returning its deposit.
    UnRegDRepCert(DRepCredential, Coin),
    /// Conway: update a DRep's anchor metadata.
    UpdateDRepCert(DRepCredential, Nullable<Anchor>),
}

/// On-chain credential controlling a stake address.
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Clone, Hash, Encode, Decode)]
// !! NOTE / IMPORTANT !!
// It is tempting to swap the order of the two constructors so that AddrKeyHash
// comes first. This indeed nicely maps the binary representation which
// associates 0 to AddrKeyHash and 1 to ScriptHash.
//
// However, for historical reasons, the ScriptHash variant comes first in the
// Haskell reference codebase. From this ordering is derived the `PartialOrd`
// and `Ord` instances; which impacts how Maps/Dictionnaries indexed by
// StakeCredential will be ordered. So, it is crucial to preserve this quirks to
// avoid hard to troubleshoot issues down the line.
#[cbor(flat)]
pub enum StakeCredential {
    /// Stake credential backed by a script hash.
    #[n(1)]
    ScriptHash(#[n(0)] ScriptHash),
    /// Stake credential backed by a verification-key hash.
    #[n(0)]
    AddrKeyhash(#[n(0)] AddrKeyhash),
}

/// Hash of a stake pool's cold key (Blake2b-224).
pub type PoolKeyhash = Hash<28>;
/// Hash of a VRF verification key (Blake2b-256).
pub type VrfKeyhash = Hash<32>;
/// Credential identifying a DRep (same shape as a stake credential).
pub type DRepCredential = StakeCredential;
/// Cold credential of a constitutional-committee member.
pub type CommitteeColdCredential = StakeCredential;
/// Hot credential of a constitutional-committee member.
pub type CommitteeHotCredential = StakeCredential;
/// Hash of an address verification key (Blake2b-224).
pub type AddrKeyhash = Hash<28>;

/// Identity of a voter in a Conway governance vote.
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
pub enum Voter {
    /// Vote cast by a constitutional-committee key.
    ConstitutionalCommitteeKey(AddrKeyhash),
    /// Vote cast by a constitutional-committee script.
    ConstitutionalCommitteeScript(ScriptHash),
    /// Vote cast by a DRep key.
    DRepKey(AddrKeyhash),
    /// Vote cast by a DRep script.
    DRepScript(ScriptHash),
    /// Vote cast by a stake-pool operator.
    StakePoolKey(AddrKeyhash),
}

/// Plutus language version.
#[derive(Encode, Decode, Debug, Clone, Eq, PartialEq)]
#[cbor(index_only)]
pub enum Language {
    /// Plutus V1.
    #[n(0)]
    PlutusV1,
    /// Plutus V2.
    #[n(1)]
    PlutusV2,
    /// Plutus V3.
    #[n(2)]
    PlutusV3,
}

/// Generic on-chain script: a native script or a Plutus script of any version.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum PseudoScript<T1> {
    /// Cardano native script (timelock and signature combinators).
    NativeScript(T1),
    /// Plutus V1 script bytes.
    PlutusV1Script(PlutusScript<1>),
    /// Plutus V2 script bytes.
    PlutusV2Script(PlutusScript<2>),
    /// Plutus V3 script bytes.
    PlutusV3Script(PlutusScript<3>),
}

/// Raw bytes of a Plutus script of language version `VERSION`.
#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[cbor(transparent)]
pub struct PlutusScript<const VERSION: usize>(#[n(0)] pub Bytes);

/// Native script — pre-Plutus combinators (signatures + timelocks).
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum NativeScript {
    /// Require a signature from the given key hash.
    ScriptPubkey(AddrKeyhash),
    /// Require every sub-script to be satisfied.
    ScriptAll(Vec<NativeScript>),
    /// Require any sub-script to be satisfied.
    ScriptAny(Vec<NativeScript>),
    /// Require at least `n` of the listed sub-scripts to be satisfied.
    ScriptNOfK(u32, Vec<NativeScript>),
    /// Valid only at or after the given slot.
    InvalidBefore(u64),
    /// Valid only before the given slot.
    InvalidHereafter(u64),
}

/// Script reference attached to a transaction output (CIP-33).
pub type ScriptRef = PseudoScript<NativeScript>;
