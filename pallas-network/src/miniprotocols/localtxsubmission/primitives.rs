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
    utils::{Nullable, Set},
};

pub type Multiasset<A> = KeyValuePairs<PolicyId, KeyValuePairs<AssetName, A>>;

pub type Mint = Multiasset<i64>;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Value {
    Coin(Coin),
    Multiasset(Coin, Multiasset<Coin>),
}

impl<'b, C> minicbor::decode::Decode<'b, C> for Value {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        match d.datatype()? {
            minicbor::data::Type::U8 => Ok(Value::Coin(d.decode_with(ctx)?)),
            minicbor::data::Type::U16 => Ok(Value::Coin(d.decode_with(ctx)?)),
            minicbor::data::Type::U32 => Ok(Value::Coin(d.decode_with(ctx)?)),
            minicbor::data::Type::U64 => Ok(Value::Coin(d.decode_with(ctx)?)),
            minicbor::data::Type::Array => {
                d.array()?;
                let coin = d.decode_with(ctx)?;
                let multiasset = d.decode_with(ctx)?;
                Ok(Value::Multiasset(coin, multiasset))
            }
            _ => Err(minicbor::decode::Error::message(
                "unknown cbor data type for Alonzo Value enum",
            )),
        }
    }
}

impl<C> minicbor::encode::Encode<C> for Value {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        // TODO: check how to deal with uint variants (u32 vs u64)
        match self {
            Value::Coin(coin) => {
                e.encode_with(coin, ctx)?;
            }
            Value::Multiasset(coin, other) => {
                e.array(2)?;
                e.encode_with(coin, ctx)?;
                e.encode_with(other, ctx)?;
            }
        };

        Ok(())
    }
}

// https://github.com/IntersectMBO/cardano-ledger/blob/33e90ea03447b44a389985ca2b158568e5f4ad65/libs/cardano-ledger-core/src/Cardano/Ledger/Credential.hs#L82
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Credential {
    ScriptHashObj(ScriptHash),
    KeyHashObj(AddrKeyhash),
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
    #[n(3)]
    PlutusV3,
}
