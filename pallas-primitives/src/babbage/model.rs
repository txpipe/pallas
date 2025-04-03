//! Ledger primitives and cbor codec for the Babbage era
//!
//! Handcrafted, idiomatic rust artifacts based on based on the [Babbage CDDL](https://github.com/input-output-hk/cardano-ledger/blob/master/eras/babbage/test-suite/cddl-files/babbage.cddl) file in IOHK repo.

use serde::{Deserialize, Serialize};

use pallas_codec::{
    minicbor::{self, Decode, Encode},
    utils::{Bytes, CborWrap, KeepRaw, Nullable},
};
use pallas_crypto::hash::{Hash, Hasher};

pub use pallas_codec::codec_by_datatype;

pub use crate::{
    plutus_data::*, AddrKeyhash, AssetName, DatumHash, DnsName, Epoch, ExUnitPrices, ExUnits,
    GenesisDelegateHash, Genesishash, IPv4, IPv6, Metadata, Metadatum, MetadatumLabel, NetworkId,
    Nonce, NonceVariant, PlutusScript, PolicyId, PoolKeyhash, PoolMetadata, PoolMetadataHash, Port,
    PositiveInterval, ProtocolVersion, RationalNumber, Relay, ScriptHash, StakeCredential,
    TransactionIndex, TransactionInput, UnitInterval, VrfCert, VrfKeyhash,
};

use crate::BTreeMap;

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
    pub operational_cert: OperationalCert,

    #[n(9)]
    pub protocol_version: ProtocolVersion,
}

#[derive(Serialize, Deserialize, Encode, Decode, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct OperationalCert {
    #[n(0)]
    pub operational_cert_hot_vkey: Bytes,

    #[n(1)]
    pub operational_cert_sequence_number: u64,

    #[n(2)]
    pub operational_cert_kes_period: u64,

    #[n(3)]
    pub operational_cert_sigma: Bytes,
}

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub struct Header {
    #[n(0)]
    pub header_body: HeaderBody,

    #[n(1)]
    pub body_signature: Bytes,
}

#[deprecated(since = "1.0.0-alpha", note = "use `KeepRaw<'_, Header>` instead")]
pub type MintedHeader<'a> = KeepRaw<'a, Header>;

pub use crate::alonzo::Multiasset;

pub use crate::alonzo::Mint;

pub use crate::alonzo::Coin;

pub use crate::alonzo::Value;

pub use crate::alonzo::TransactionOutput as LegacyTransactionOutput;

pub use crate::alonzo::InstantaneousRewardSource;

pub use crate::alonzo::InstantaneousRewardTarget;

pub use crate::alonzo::MoveInstantaneousReward;

pub use crate::alonzo::RewardAccount;

pub use crate::alonzo::Withdrawals;

pub use crate::alonzo::RequiredSigners;

pub use crate::alonzo::Certificate;

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[cbor(index_only)]
pub enum Language {
    #[n(0)]
    PlutusV1,

    #[n(1)]
    PlutusV2,
}

#[deprecated(since = "0.31.0", note = "use `CostModels` instead")]
pub type CostMdls = CostModels;

pub use crate::alonzo::CostModel;

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[cbor(map)]
pub struct CostModels {
    #[n(0)]
    pub plutus_v1: Option<CostModel>,

    #[n(1)]
    pub plutus_v2: Option<CostModel>,
}

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[cbor(map)]
pub struct ProtocolParamUpdate {
    #[n(0)]
    pub minfee_a: Option<u32>,
    #[n(1)]
    pub minfee_b: Option<u32>,
    #[n(2)]
    pub max_block_body_size: Option<u32>,
    #[n(3)]
    pub max_transaction_size: Option<u32>,
    #[n(4)]
    pub max_block_header_size: Option<u32>,
    #[n(5)]
    pub key_deposit: Option<Coin>,
    #[n(6)]
    pub pool_deposit: Option<Coin>,
    #[n(7)]
    pub maximum_epoch: Option<Epoch>,
    #[n(8)]
    pub desired_number_of_stake_pools: Option<u32>,
    #[n(9)]
    pub pool_pledge_influence: Option<RationalNumber>,
    #[n(10)]
    pub expansion_rate: Option<UnitInterval>,
    #[n(11)]
    pub treasury_growth_rate: Option<UnitInterval>,

    #[n(14)]
    pub protocol_version: Option<ProtocolVersion>,
    #[n(16)]
    pub min_pool_cost: Option<Coin>,
    #[n(17)]
    pub ada_per_utxo_byte: Option<Coin>,
    #[n(18)]
    pub cost_models_for_script_languages: Option<CostModels>,
    #[n(19)]
    pub execution_costs: Option<ExUnitPrices>,
    #[n(20)]
    pub max_tx_ex_units: Option<ExUnits>,
    #[n(21)]
    pub max_block_ex_units: Option<ExUnits>,
    #[n(22)]
    pub max_value_size: Option<u32>,
    #[n(23)]
    pub collateral_percentage: Option<u32>,
    #[n(24)]
    pub max_collateral_inputs: Option<u32>,
}

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub struct Update {
    #[n(0)]
    pub proposed_protocol_parameter_updates: BTreeMap<Genesishash, ProtocolParamUpdate>,

    #[n(1)]
    pub epoch: Epoch,
}

#[derive(Encode, Decode, Serialize, Deserialize, Debug, PartialEq, Clone)]
#[cbor(map)]
pub struct TransactionBody<'b> {
    #[n(0)]
    pub inputs: Vec<TransactionInput>,

    #[b(1)]
    pub outputs: Vec<KeepRaw<'b, TransactionOutput<'b>>>,

    #[n(2)]
    pub fee: u64,

    #[n(3)]
    pub ttl: Option<u64>,

    #[n(4)]
    pub certificates: Option<Vec<Certificate>>,

    #[n(5)]
    pub withdrawals: Option<BTreeMap<RewardAccount, Coin>>,

    #[n(6)]
    pub update: Option<Update>,

    #[n(7)]
    pub auxiliary_data_hash: Option<Bytes>,

    #[n(8)]
    pub validity_interval_start: Option<u64>,

    #[n(9)]
    pub mint: Option<Multiasset<i64>>,

    #[n(11)]
    pub script_data_hash: Option<Hash<32>>,

    #[n(13)]
    pub collateral: Option<Vec<TransactionInput>>,

    #[n(14)]
    pub required_signers: Option<Vec<AddrKeyhash>>,

    #[n(15)]
    pub network_id: Option<NetworkId>,

    #[n(16)]
    pub collateral_return: Option<KeepRaw<'b, TransactionOutput<'b>>>,

    #[n(17)]
    pub total_collateral: Option<Coin>,

    #[n(18)]
    pub reference_inputs: Option<Vec<TransactionInput>>,
}

#[deprecated(since = "1.0.0-alpha", note = "use `TransactionBody` instead")]
pub type MintedTransactionBody<'a> = TransactionBody<'a>;

pub enum VrfDerivation {
    Leader,
    Nonce,
}

pub fn derive_tagged_vrf_output(
    block_vrf_output_bytes: &[u8],
    derivation: VrfDerivation,
) -> Vec<u8> {
    let mut tagged_vrf: Vec<u8> = match derivation {
        VrfDerivation::Leader => vec![0x4C_u8], /* "L" */
        VrfDerivation::Nonce => vec![0x4E_u8],  /* "N" */
    };

    tagged_vrf.extend(block_vrf_output_bytes);
    Hasher::<256>::hash(&tagged_vrf).to_vec()
}

impl HeaderBody {
    pub fn leader_vrf_output(&self) -> Vec<u8> {
        derive_tagged_vrf_output(&self.vrf_result.0, VrfDerivation::Leader)
    }

    pub fn nonce_vrf_output(&self) -> Vec<u8> {
        derive_tagged_vrf_output(&self.vrf_result.0, VrfDerivation::Nonce)
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum GenTransactionOutput<'b, T> {
    Legacy(KeepRaw<'b, LegacyTransactionOutput>),
    PostAlonzo(KeepRaw<'b, T>),
}

// FIXME: Repeated since macro does not handle type generics yet.
codec_by_datatype! {
    TransactionOutput<'b>,
    Array | ArrayIndef => Legacy,
    Map | MapIndef => PostAlonzo,
    ()
}

pub type TransactionOutput<'b> = GenTransactionOutput<'b, PostAlonzoTransactionOutput<'b>>;

#[deprecated(since = "1.0.0-alpha", note = "use `TransactionOutput` instead")]
pub type MintedTransactionOutput<'b> = TransactionOutput<'b>;

#[derive(Encode, Decode, Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[cbor(map)]
pub struct GenPostAlonzoTransactionOutput<'b, V, S> {
    #[n(0)]
    pub address: Bytes,

    #[n(1)]
    pub value: V,

    #[b(2)]
    pub datum_option: Option<KeepRaw<'b, DatumOption<'b>>>,

    #[n(3)]
    pub script_ref: Option<CborWrap<S>>,
}

pub type PostAlonzoTransactionOutput<'b> = GenPostAlonzoTransactionOutput<'b, Value, ScriptRef<'b>>;

#[deprecated(since = "1.0.0-alpha", note = "use `PostAlonzoTransactionOutput` instead")]
pub type MintedPostAlonzoTransactionOutput<'b> = PostAlonzoTransactionOutput<'b>;

pub use crate::alonzo::VKeyWitness;

pub use crate::alonzo::NativeScript;

pub use crate::alonzo::RedeemerTag;

pub use crate::alonzo::Redeemer;

pub use crate::alonzo::BootstrapWitness;

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Clone)]
#[cbor(map)]
pub struct WitnessSet<'b> {
    #[n(0)]
    pub vkeywitness: Option<Vec<VKeyWitness>>,

    #[n(1)]
    pub native_script: Option<Vec<KeepRaw<'b, NativeScript>>>,

    #[n(2)]
    pub bootstrap_witness: Option<Vec<BootstrapWitness>>,

    #[n(3)]
    pub plutus_v1_script: Option<Vec<PlutusScript<1>>>,

    #[b(4)]
    pub plutus_data: Option<Vec<KeepRaw<'b, PlutusData>>>,

    #[n(5)]
    pub redeemer: Option<Vec<Redeemer>>,

    #[n(6)]
    pub plutus_v2_script: Option<Vec<PlutusScript<2>>>,
}

#[deprecated(since = "1.0.0-alpha", note = "use `WitnessSet` instead")]
pub type MintedWitnessSet<'b> = WitnessSet<'b>;

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Clone)]
#[cbor(map)]
pub struct PostAlonzoAuxiliaryData {
    #[n(0)]
    pub metadata: Option<Metadata>,

    #[n(1)]
    pub native_scripts: Option<Vec<NativeScript>>,

    #[n(2)]
    pub plutus_v1_scripts: Option<Vec<PlutusScript<1>>>,

    #[n(3)]
    pub plutus_v2_scripts: Option<Vec<PlutusScript<2>>>,
}

// datum_option = [ 0, $hash32 // 1, data ]
#[derive(Encode, Decode, Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
#[cbor(flat)]
pub enum DatumOption<'b> {
    #[n(0)]
    Hash(#[n(0)] DatumHash),
    #[n(1)]
    Data(#[b(0)] CborWrap<KeepRaw<'b, PlutusData>>),
}

#[deprecated(since = "1.0.0-alpha", note = "use `DatumOption` instead")]
pub type MintedDatumOption<'b> = DatumOption<'b>;

#[deprecated(since = "0.31.0", note = "use `PlutusScript<1>` instead")]
pub type PlutusV1Script = PlutusScript<1>;

#[deprecated(since = "0.31.0", note = "use `PlutusScript<2>` instead")]
pub type PlutusV2Script = PlutusScript<2>;

// script = [ 0, native_script // 1, plutus_v1_script // 2, plutus_v2_script ]
#[derive(Encode, Decode, Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[cbor(flat)]
pub enum ScriptRef<'b> {
    #[n(0)]
    NativeScript(#[b(0)] KeepRaw<'b, NativeScript>),
    #[n(1)]
    PlutusV1Script(#[n(0)] PlutusScript<1>),
    #[n(2)]
    PlutusV2Script(#[n(0)] PlutusScript<2>),
}

#[deprecated(since = "1.0.0-alpha", note = "use `ScriptRef` instead")]
pub type MintedScriptRef<'b> = ScriptRef<'b>;

pub use crate::alonzo::AuxiliaryData;

/// A memory representation of an already minted block
///
/// This structure allows to retrieve the
/// original CBOR bytes for each structure that might require hashing. In this
/// way, we make sure that the resulting hash matches what exists on-chain.
#[derive(Serialize, Encode, Decode, Debug, PartialEq, Clone)]
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
    pub invalid_transactions: Option<Vec<TransactionIndex>>,
}


#[deprecated(since = "1.0.0-alpha", note = "use `Block` instead")]
pub type MintedBlock<'b> = Block<'b>;

#[derive(Clone, Serialize, Deserialize, Encode, Decode, Debug)]
pub struct Tx<'b> {
    #[b(0)]
    pub transaction_body: KeepRaw<'b, TransactionBody<'b>>,

    #[n(1)]
    pub transaction_witness_set: KeepRaw<'b, WitnessSet<'b>>,

    #[n(2)]
    pub success: bool,

    #[n(3)]
    pub auxiliary_data: Nullable<KeepRaw<'b, AuxiliaryData>>,
}

#[deprecated(since = "1.0.0-alpha", note = "use `Tx` instead")]
pub type MintedTx<'b> = Tx<'b>;

#[cfg(test)]
mod tests {
    use pallas_codec::minicbor;

    use super::{Block, TransactionOutput};
    use crate::Fragment;

    type BlockWrapper<'b> = (u16, Block<'b>);

    #[test]
    fn block_isomorphic_decoding_encoding() {
        let test_blocks = [
            include_str!("../../../test_data/babbage1.block"),
            include_str!("../../../test_data/babbage2.block"),
            include_str!("../../../test_data/babbage3.block"),
            // peculiar block with single plutus cost model
            include_str!("../../../test_data/babbage4.block"),
            // peculiar block with i32 overlfow
            include_str!("../../../test_data/babbage5.block"),
            // peculiar block with map undef in plutus data
            include_str!("../../../test_data/babbage6.block"),
            // block with generic int in cbor
            include_str!("../../../test_data/babbage7.block"),
            // block with indef bytes for plutus data bignum
            include_str!("../../../test_data/babbage8.block"),
            // // block with inline datum that fails hashes
            // include_str!("../../../test_data/babbage9.block"),
            // block with pool margin numerator greater than i64::MAX
            include_str!("../../../test_data/babbage10.block"),
        ];

        for (idx, block_str) in test_blocks.iter().enumerate() {
            println!("decoding test block {}", idx + 1);
            let bytes = hex::decode(block_str).unwrap_or_else(|_| panic!("bad block file {idx}"));

            let block: BlockWrapper = minicbor::decode(&bytes[..])
                .unwrap_or_else(|e| panic!("error decoding cbor for file {idx}: {e:?}"));

            let bytes2 = minicbor::to_vec(block)
                .unwrap_or_else(|e| panic!("error encoding block cbor for file {idx}: {e:?}"));

            assert!(bytes.eq(&bytes2), "re-encoded bytes didn't match original");
        }
    }

    #[test]
    fn fragments_decoding() {
        // peculiar array of outputs used in an hydra transaction
        let hex = include_str!("../../../test_data/babbage1.fr");
        let bytes = hex::decode(hex).unwrap();
        let outputs = Vec::<TransactionOutput>::decode_fragment(&bytes).unwrap();

        dbg!(outputs);

        // add any loose fragment tests here
    }
}
