//! Ledger primitives and cbor codec for the Conway era
//!
//! Handcrafted, idiomatic rust artifacts based on based on the [CIP proposal](https://github.com/cardano-scaling/CIPs/blob/leios/CIP-0164/README.md) at the cardano-scaling repo.

use serde::{Deserialize, Serialize};

use pallas_codec::minicbor::{self, Decode, Encode};

pub use pallas_codec::codec_by_datatype;

pub use crate::{
    plutus_data::*, AddrKeyhash, AssetName, Bytes, Coin, CostModel, DnsName, Epoch, ExUnits,
    GenesisDelegateHash, Genesishash, Hash, IPv4, IPv6, KeepRaw, Metadata, Metadatum,
    MetadatumLabel, NetworkId, NonEmptySet, NonZeroInt, Nonce, NonceVariant, Nullable,
    PlutusScript, PolicyId, PoolKeyhash, PoolMetadata, PoolMetadataHash, Port, PositiveCoin,
    PositiveInterval, ProtocolVersion, RationalNumber, Relay, RewardAccount, ScriptHash, Set,
    StakeCredential, TransactionIndex, TransactionInput, UnitInterval, VrfCert, VrfKeyhash,
};

use crate::{babbage, conway, BTreeMap};

pub use babbage::{
    derive_tagged_vrf_output, DatumHash, DatumOption, OperationalCert, VrfDerivation,
};

pub use crate::alonzo::{AuxiliaryData, BootstrapWitness, NativeScript, VKeyWitness};

pub use conway::{
    Anchor, Certificate, CommitteeColdCredential, CommitteeHotCredential, Constitution, CostModels,
    DRep, DRepCredential, DRepVotingThresholds, ExUnitPrices, GovActionId, Language,
    LegacyTransactionOutput, Mint, Multiasset, PoolVotingThresholds, PostAlonzoAuxiliaryData,
    PostAlonzoTransactionOutput, ProposalProcedure, ProtocolParamUpdate, Redeemer, RedeemerTag,
    Redeemers, RedeemersKey, RedeemersValue, RequiredSigners, ScriptRef, TransactionBody,
    TransactionOutput, Tx, Update, Value, Vote, Voter, VotingProcedure, VotingProcedures,
    Withdrawals, WitnessSet,
};

/// Leios ranking block.
///
/// This is the original Praos (Conway) block, with additional fields for announcing and
/// and certifying previously announced endorser blocks.
#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Clone)]
pub struct Block<'b> {
    #[n(0)]
    pub header: KeepRaw<'b, Header>,

    #[b(1)]
    pub transaction_bodies: Vec<KeepRaw<'b, TransactionBody<'b>>>,

    #[n(2)]
    pub transaction_witness_sets: Vec<KeepRaw<'b, WitnessSet<'b>>>,

    #[n(3)]
    pub auxiliary_data_set: BTreeMap<TransactionIndex, KeepRaw<'b, AuxiliaryData>>,

    #[n(4)]
    pub eb_certificate: Option<KeepRaw<'b, LeiosCertificate>>,

    #[n(5)]
    pub invalid_transactions: Option<Vec<TransactionIndex>>,
}

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub struct EbAnnouncement {
    #[n(0)]
    pub announced_eb: Option<Hash<32>>,

    #[n(1)]
    pub announced_eb_size: Option<u32>,
}

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub struct HeaderBody {
    #[n(0)]
    pub block_number: u64,

    #[n(1)]
    pub slot: u64,

    #[n(2)]
    pub prev_hash: Option<Hash<32>>,

    #[n(3)]
    pub issuer_vkey: Bytes,

    #[n(4)]
    pub vrf_vkey: Bytes,

    #[n(5)]
    pub vrf_result: VrfCert,

    #[n(6)]
    pub block_body_size: u64,

    #[n(7)]
    pub block_body_hash: Hash<32>,

    #[n(8)]
    pub eb_announcement: Option<EbAnnouncement>,

    #[n(9)]
    pub certified_eb: Option<bool>,

    #[n(10)]
    pub operational_cert: OperationalCert,

    #[n(11)]
    pub protocol_version: ProtocolVersion,
}

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub struct Header {
    #[n(0)]
    pub header_body: HeaderBody,

    #[n(1)]
    pub body_signature: Bytes,
}

impl HeaderBody {
    pub fn leader_vrf_output(&self) -> Vec<u8> {
        derive_tagged_vrf_output(&self.vrf_result.0, VrfDerivation::Leader)
    }

    pub fn nonce_vrf_output(&self) -> Vec<u8> {
        derive_tagged_vrf_output(&self.vrf_result.0, VrfDerivation::Nonce)
    }
}

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub struct EndorserBlock {
    #[n(0)]
    pub transaction_references: Vec<TxReference>,
}

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub struct TxReference {
    #[n(0)]
    pub tx_hash: Hash<32>,

    #[n(1)]
    pub tx_size: u16,
}

pub type BlsSignature = Bytes; // 48 bytes

// pub type ElectionId = Bytes; // 8 bytes
// pub type PersistentVoterId = Bytes; // 2 bytes
// pub type EndorserBlockHash = Hash<32>;

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub struct LeiosCertificate {
    #[n(0)]
    pub election_id: Bytes,

    #[n(1)]
    pub endorser_block_hash: Hash<32>,

    #[n(2)]
    pub persistent_voters: Vec<BlsSignature>,

    #[n(3)]
    pub nonpersistent_voters: BTreeMap<PoolKeyhash, BlsSignature>,

    #[n(4)]
    pub aggregate_elig_sig: Option<BlsSignature>,

    #[n(5)]
    pub aggregate_vote_sig: BlsSignature,
}

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[cbor(flat)]
pub enum LeiosVote {
    #[n(0)]
    Persistent {
        #[n(0)]
        election_id: Bytes,

        #[n(1)]
        persistent_voter_id: Bytes,

        #[n(2)]
        endorser_block_hash: Hash<32>,

        #[n(3)]
        vote_signature: BlsSignature,
    },

    #[n(1)]
    NonPersistent {
        #[n(0)]
        election_id: Bytes,

        #[n(1)]
        pool_id: PoolKeyhash,

        #[n(2)]
        eligibility_signature: BlsSignature,

        #[n(3)]
        endorser_block_hash: Hash<32>,

        #[n(5)]
        vote_signature: BlsSignature,
    },
}
