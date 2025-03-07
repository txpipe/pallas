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

pub type Multiasset<A> = NonEmptyKeyValuePairs<PolicyId, NonEmptyKeyValuePairs<AssetName, A>>;

pub type Mint = Multiasset<i64>;

// https://github.com/IntersectMBO/cardano-ledger/blob/33e90ea03447b44a389985ca2b158568e5f4ad65/libs/cardano-ledger-core/src/Cardano/Ledger/Credential.hs#L82
#[derive(Debug, Decode, Encode, Clone, Hash, PartialEq, Eq)]
#[cbor(flat)]
pub enum Credential {
    #[n(0)]
    ScriptHashObj(#[n(0)] ScriptHash),
    #[n(1)]
    KeyHashObj(#[n(0)] AddrKeyhash),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Certificate {
    StakeRegistration(StakeCredential),
    StakeDeregistration(StakeCredential),
    StakeDelegation(StakeCredential, PoolKeyhash),
    PoolRegistration {
        operator: PoolKeyhash,
        vrf_keyhash: VrfKeyhash,
        pledge: Coin,
        cost: Coin,
        margin: UnitInterval,
        reward_account: RewardAccount,
        pool_owners: Set<AddrKeyhash>,
        relays: Vec<Relay>,
        pool_metadata: Nullable<PoolMetadata>,
    },
    PoolRetirement(PoolKeyhash, Epoch),

    Reg(StakeCredential, Coin),
    UnReg(StakeCredential, Coin),
    VoteDeleg(StakeCredential, DRep),
    StakeVoteDeleg(StakeCredential, PoolKeyhash, DRep),
    StakeRegDeleg(StakeCredential, PoolKeyhash, Coin),
    VoteRegDeleg(StakeCredential, DRep, Coin),
    StakeVoteRegDeleg(StakeCredential, PoolKeyhash, DRep, Coin),

    AuthCommitteeHot(CommitteeColdCredential, CommitteeHotCredential),
    ResignCommitteeCold(CommitteeColdCredential, Nullable<Anchor>),
    RegDRepCert(DRepCredential, Coin, Nullable<Anchor>),
    UnRegDRepCert(DRepCredential, Coin),
    UpdateDRepCert(DRepCredential, Nullable<Anchor>),
}

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
    #[n(1)]
    ScriptHash(#[n(0)] ScriptHash),
    #[n(0)]
    AddrKeyhash(#[n(0)] AddrKeyhash),
}

pub type PoolKeyhash = Hash<28>;
pub type VrfKeyhash = Hash<32>;
pub type DRepCredential = StakeCredential;
pub type CommitteeColdCredential = StakeCredential;
pub type CommitteeHotCredential = StakeCredential;
pub type AddrKeyhash = Hash<28>;

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
pub enum Voter {
    ConstitutionalCommitteeKey(AddrKeyhash),
    ConstitutionalCommitteeScript(ScriptHash),
    DRepKey(AddrKeyhash),
    DRepScript(ScriptHash),
    StakePoolKey(AddrKeyhash),
}

#[derive(Encode, Decode, Debug, Clone, Eq, PartialEq)]
#[cbor(index_only)]
pub enum Language {
    #[n(0)]
    PlutusV1,
    #[n(1)]
    PlutusV2,
    #[n(2)]
    PlutusV3,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum PseudoScript<T1> {
    NativeScript(T1),
    PlutusV1Script(PlutusScript<1>),
    PlutusV2Script(PlutusScript<2>),
    PlutusV3Script(PlutusScript<3>),
}

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[cbor(transparent)]
pub struct PlutusScript<const VERSION: usize>(#[n(0)] pub Bytes);

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum NativeScript {
    ScriptPubkey(AddrKeyhash),
    ScriptAll(Vec<NativeScript>),
    ScriptAny(Vec<NativeScript>),
    ScriptNOfK(u32, Vec<NativeScript>),
    InvalidBefore(u64),
    InvalidHereafter(u64),
}

pub type ScriptRef = PseudoScript<NativeScript>;
