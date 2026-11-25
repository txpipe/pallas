//! Ledger primitives and cbor codec for the Alonzo era
//!
//! Handcrafted, idiomatic rust artifacts based on based on the [Alonzo CDDL](https://github.com/input-output-hk/cardano-ledger/blob/master/eras/alonzo/test-suite/cddl-files/alonzo.cddl) file in IOHK repo.

use serde::{Deserialize, Serialize};

use pallas_codec::minicbor::{self, Decode, Encode};

pub use pallas_codec::codec_by_datatype;

pub use crate::{
    AddrKeyhash, AssetName, Bytes, Coin, CostModel, DatumHash, DnsName, Epoch, ExUnitPrices,
    ExUnits, GenesisDelegateHash, Genesishash, Hash, IPv4, IPv6, Int, KeepRaw, Metadata, Metadatum,
    MetadatumLabel, NetworkId, Nonce, NonceVariant, Nullable, PlutusScript, PolicyId, PoolKeyhash,
    PoolMetadata, PoolMetadataHash, Port, PositiveInterval, ProtocolVersion, RationalNumber, Relay,
    RewardAccount, ScriptHash, StakeCredential, TransactionIndex, TransactionInput, UnitInterval,
    VrfCert, VrfKeyhash, plutus_data::*,
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
    pub nonce_vrf: VrfCert,

    #[n(6)]
    pub leader_vrf: VrfCert,

    #[n(7)]
    pub block_body_size: u64,

    #[n(8)]
    pub block_body_hash: Hash<32>,

    #[n(9)]
    pub operational_cert_hot_vkey: Bytes,

    #[n(10)]
    pub operational_cert_sequence_number: u64,

    #[n(11)]
    pub operational_cert_kes_period: u64,

    #[n(12)]
    pub operational_cert_sigma: Bytes,

    #[n(13)]
    pub protocol_major: u64,

    #[n(14)]
    pub protocol_minor: u64,
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

pub type Multiasset<A> = BTreeMap<PolicyId, BTreeMap<AssetName, A>>;

pub type Mint = Multiasset<i64>;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum Value {
    Coin(Coin),
    Multiasset(Coin, Multiasset<Coin>),
}

#[cfg(not(feature = "unstable"))]
codec_by_datatype! {
    Value,
    U8 | U16 | U32 | U64 => Coin,
    (coin, multi => Multiasset)
}

/// Implements the codec of a value enum, reading its `Multiasset` variant from an array of either length form.
#[cfg(feature = "unstable")]
macro_rules! value_codec {
    ($name:ident) => {
        impl<'b, C> pallas_codec::minicbor::Decode<'b, C> for $name {
            fn decode(
                d: &mut pallas_codec::minicbor::Decoder<'b>,
                ctx: &mut C,
            ) -> Result<Self, pallas_codec::minicbor::decode::Error> {
                use pallas_codec::minicbor::data::Type;
                match d.datatype()? {
                    Type::Array | Type::ArrayIndef => {
                        let len = d.array()?;
                        let value = $name::Multiasset(d.decode_with(ctx)?, d.decode_with(ctx)?);
                        pallas_codec::utils::skip_array_rest(d, len, 2)?;
                        Ok(value)
                    }
                    Type::U8 | Type::U16 | Type::U32 | Type::U64 => {
                        Ok($name::Coin(d.decode_with(ctx)?))
                    }
                    _ => Err(pallas_codec::minicbor::decode::Error::message(
                        "Unknown cbor data type for this macro-defined enum.",
                    )),
                }
            }
        }

        impl<C> pallas_codec::minicbor::Encode<C> for $name {
            fn encode<W: pallas_codec::minicbor::encode::Write>(
                &self,
                e: &mut pallas_codec::minicbor::Encoder<W>,
                ctx: &mut C,
            ) -> Result<(), pallas_codec::minicbor::encode::Error<W::Error>> {
                match self {
                    $name::Multiasset(coin, multi) => {
                        e.array(2)?;
                        e.encode_with(coin, ctx)?;
                        e.encode_with(multi, ctx)?;
                    }
                    $name::Coin(coin) => {
                        e.encode_with(coin, ctx)?;
                    }
                }
                Ok(())
            }
        }
    };
}
#[cfg(feature = "unstable")]
pub(crate) use value_codec;

#[cfg(feature = "unstable")]
value_codec!(Value);

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub struct TransactionOutput {
    #[n(0)]
    pub address: Bytes,

    #[n(1)]
    pub amount: Value,

    #[n(2)]
    pub datum_hash: Option<DatumHash>,
}

/* move_instantaneous_reward = [ 0 / 1, { * stake_credential => delta_coin } / coin ]
; The first field determines where the funds are drawn from.
; 0 denotes the reserves, 1 denotes the treasury.
; If the second field is a map, funds are moved to stake credentials,
; otherwise the funds are given to the other accounting pot.
 */

#[derive(Encode, Decode, Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[cbor(index_only)]
pub enum InstantaneousRewardSource {
    #[n(0)]
    Reserves,
    #[n(1)]
    Treasury,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum InstantaneousRewardTarget {
    StakeCredentials(BTreeMap<StakeCredential, i64>),
    OtherAccountingPot(Coin),
}

codec_by_datatype! {
    InstantaneousRewardTarget,
    Map | MapIndef => StakeCredentials,
    U8 | U16 | U32 | U64 | I8 | I16 | I32 | I64 | Int => OtherAccountingPot,
    ()
}

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[cbor()]
pub struct MoveInstantaneousReward {
    #[n(0)]
    pub source: InstantaneousRewardSource,

    #[n(1)]
    pub target: InstantaneousRewardTarget,
}

pub type Withdrawals = BTreeMap<RewardAccount, Coin>;

pub type RequiredSigners = Vec<AddrKeyhash>;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Encode, Decode)]
#[cbor(flat)]
pub enum Certificate {
    #[n(0)]
    StakeRegistration(#[n(0)] StakeCredential),
    #[n(1)]
    StakeDeregistration(#[n(0)] StakeCredential),
    #[n(2)]
    StakeDelegation(#[n(0)] StakeCredential, #[n(1)] PoolKeyhash),
    #[n(3)]
    PoolRegistration {
        #[n(0)]
        operator: PoolKeyhash,
        #[n(1)]
        vrf_keyhash: VrfKeyhash,
        #[n(2)]
        pledge: Coin,
        #[n(3)]
        cost: Coin,
        #[n(4)]
        margin: UnitInterval,
        #[n(5)]
        reward_account: RewardAccount,
        #[n(6)]
        pool_owners: Vec<AddrKeyhash>,
        #[n(7)]
        relays: Vec<Relay>,
        #[n(8)]
        pool_metadata: Option<PoolMetadata>,
    },
    #[n(4)]
    PoolRetirement(#[n(0)] PoolKeyhash, #[n(1)] Epoch),
    #[n(5)]
    GenesisKeyDelegation(
        #[n(0)] Genesishash,
        #[n(1)] GenesisDelegateHash,
        #[n(2)] VrfKeyhash,
    ),
    #[n(6)]
    MoveInstantaneousRewardsCert(#[n(0)] MoveInstantaneousReward),
}

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[cbor(index_only)]
pub enum Language {
    #[n(0)]
    PlutusV1,
}

#[deprecated(since = "0.31.0", note = "use `CostModels` instead")]
pub type CostMdls = CostModels;

pub type CostModels = BTreeMap<Language, CostModel>;

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
    #[n(12)]
    pub decentralization_constant: Option<UnitInterval>,
    #[n(13)]
    pub extra_entropy: Option<Nonce>,
    #[n(14)]
    #[cfg_attr(
        feature = "unstable",
        cbor(decode_with = "crate::protocol_version::decode_option")
    )]
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

// Can't derive encode for TransactionBody because it seems to require a very
// particular order for each key in the map
#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[cbor(map)]
pub struct TransactionBody {
    #[n(0)]
    pub inputs: Vec<TransactionInput>,

    #[n(1)]
    pub outputs: Vec<TransactionOutput>,

    #[n(2)]
    pub fee: u64,

    #[n(3)]
    pub ttl: Option<u64>,

    #[n(4)]
    pub certificates: Option<Vec<Certificate>>,

    #[n(5)]
    pub withdrawals: Option<Withdrawals>,

    #[n(6)]
    pub update: Option<Update>,

    #[n(7)]
    pub auxiliary_data_hash: Option<Hash<32>>,

    #[n(8)]
    pub validity_interval_start: Option<u64>,

    #[n(9)]
    pub mint: Option<Multiasset<i64>>,

    #[n(11)]
    pub script_data_hash: Option<Hash<32>>,

    #[n(13)]
    pub collateral: Option<Vec<TransactionInput>>,

    #[n(14)]
    pub required_signers: Option<RequiredSigners>,

    #[n(15)]
    pub network_id: Option<NetworkId>,
}

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub struct VKeyWitness {
    #[n(0)]
    pub vkey: Bytes,

    #[n(1)]
    pub signature: Bytes,
}

/// A native script tree with stack-safe CBOR, clone, equality and drop operations.
///
/// This type implements `Drop` to dismantle deep trees iteratively. To consume
/// a child list, match by mutable reference and use [`std::mem::take`]. This
/// stack-safety guarantee does not cover Serde, debug formatting or downstream
/// tree visitors.
#[derive(Serialize, Deserialize, Debug, Eq)]
pub enum NativeScript {
    ScriptPubkey(AddrKeyhash),
    ScriptAll(Vec<NativeScript>),
    ScriptAny(Vec<NativeScript>),
    /// The ledger CDDL types this threshold signed (`int32` in Shelley,
    /// widened to `int64` in Allegra), not `uint`; do not narrow it back.
    ScriptNOfK(i64, Vec<NativeScript>),
    InvalidBefore(u64),
    InvalidHereafter(u64),
}

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone, Copy)]
#[cbor(index_only)]
pub enum RedeemerTag {
    #[n(0)]
    Spend,
    #[n(1)]
    Mint,
    #[n(2)]
    Cert,
    #[n(3)]
    Reward,
}

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub struct Redeemer {
    #[n(0)]
    pub tag: RedeemerTag,

    #[n(1)]
    pub index: u32,

    #[n(2)]
    pub data: PlutusData,

    #[n(3)]
    pub ex_units: ExUnits,
}

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone, Copy)]
pub struct RedeemerPointer {
    #[n(0)]
    pub tag: RedeemerTag,

    #[n(1)]
    pub index: u32,
}

/* bootstrap_witness =
[ public_key : $vkey
, signature  : $signature
, chain_code : bytes .size 32
, attributes : bytes
] */

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub struct BootstrapWitness {
    #[n(0)]
    pub public_key: Bytes,

    #[n(1)]
    pub signature: Bytes,

    #[n(2)]
    pub chain_code: Bytes,

    #[n(3)]
    pub attributes: Bytes,
}

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
    pub plutus_script: Option<Vec<PlutusScript<1>>>,

    #[b(4)]
    pub plutus_data: Option<Vec<KeepRaw<'b, PlutusData>>>,

    #[n(5)]
    pub redeemer: Option<Vec<Redeemer>>,
}

#[deprecated(since = "1.0.0-alpha", note = "use `WitnessSet` instead")]
pub type MintedWitnessSet<'b> = WitnessSet<'b>;

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Clone, Eq)]
#[cbor(map, tag(259))]
pub struct PostAlonzoAuxiliaryData {
    #[n(0)]
    pub metadata: Option<Metadata>,

    #[n(1)]
    pub native_scripts: Option<Vec<NativeScript>>,

    #[n(2)]
    pub plutus_scripts: Option<Vec<PlutusScript<1>>>,
}

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Clone, Eq)]
pub struct ShelleyMaAuxiliaryData {
    #[n(0)]
    pub transaction_metadata: Metadata,

    #[n(1)]
    pub auxiliary_scripts: Option<Vec<NativeScript>>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Eq)]
pub enum AuxiliaryData {
    Shelley(Metadata),
    ShelleyMa(ShelleyMaAuxiliaryData),
    PostAlonzo(PostAlonzoAuxiliaryData),
}

codec_by_datatype! {
    AuxiliaryData,
    Map | MapIndef => Shelley,
    Array | ArrayIndef => ShelleyMa,
    Tag => PostAlonzo,
    ()
}

/// A memory representation of an already minted block
///
/// This structure allows to retrieve the
/// original CBOR bytes for each structure that might require hashing. In this
/// way, we make sure that the resulting hash matches what exists on-chain.
#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Clone)]
pub struct Block<'b> {
    #[n(0)]
    pub header: KeepRaw<'b, Header>,

    #[b(1)]
    pub transaction_bodies: Vec<KeepRaw<'b, TransactionBody>>,

    #[n(2)]
    pub transaction_witness_sets: Vec<KeepRaw<'b, WitnessSet<'b>>>,

    #[n(3)]
    pub auxiliary_data_set: BTreeMap<TransactionIndex, KeepRaw<'b, AuxiliaryData>>,

    #[n(4)]
    pub invalid_transactions: Option<Vec<TransactionIndex>>,
}

#[deprecated(since = "1.0.0-alpha", note = "use `Block` instead")]
pub type MintedBlock<'b> = Block<'b>;

#[derive(Encode, Decode, Debug, Clone)]
pub struct Tx<'b> {
    #[b(0)]
    pub transaction_body: KeepRaw<'b, TransactionBody>,

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
    use pallas_codec::minicbor::{self, to_vec};

    use crate::{Fragment, alonzo::PlutusData};

    use super::{Block, Header};
    use crate::alonzo::NativeScript;

    type BlockWrapper<'b> = (u16, Block<'b>);

    #[cfg(feature = "unstable")]
    #[test]
    fn a_native_script_reads_either_length_form() {
        let script = format!("8200581c{}", "11".repeat(28));
        crate::flat_enum::assert_reads_either_length_form::<NativeScript>(&[
            script.clone(),
            format!("820181{script}"),
            "820280".to_string(),
            "83030180".to_string(),
            "820401".to_string(),
            "820501".to_string(),
        ]);
    }

    #[cfg(feature = "unstable")]
    #[test]
    fn a_native_script_with_extra_elements_reads_to_the_end_of_its_array() {
        for (bytes, expected) in [
            ("83050100", NativeScript::InvalidHereafter(1)),
            ("9f05019f00ff00ff", NativeScript::InvalidHereafter(1)),
            ("83018000", NativeScript::ScriptAll(vec![])),
            ("9f019fff00ff", NativeScript::ScriptAll(vec![])),
        ] {
            let bytes = hex::decode(bytes).unwrap();
            let mut d = minicbor::Decoder::new(&bytes);
            let script: NativeScript = d.decode().unwrap();
            assert_eq!(script, expected);
            assert_eq!(d.position(), bytes.len());
        }
    }

    #[test]
    fn a_native_script_without_its_fields_or_with_an_unknown_index_is_refused() {
        for bytes in ["80", "8105", "9f05ff", "9fff", "820701", "9f0701ff"] {
            assert!(
                minicbor::decode::<NativeScript>(&hex::decode(bytes).unwrap()).is_err(),
                "{bytes} must be refused"
            );
        }
    }

    #[test]
    fn block_isomorphic_decoding_encoding() {
        let test_blocks = vec![
            include_str!("../../../test_data/alonzo1.block"),
            include_str!("../../../test_data/alonzo2.block"),
            include_str!("../../../test_data/alonzo3.block"),
            include_str!("../../../test_data/alonzo4.block"),
            include_str!("../../../test_data/alonzo5.block"),
            include_str!("../../../test_data/alonzo6.block"),
            include_str!("../../../test_data/alonzo7.block"),
            include_str!("../../../test_data/alonzo8.block"),
            // include_str!("../../../test_data/alonzo9.block"),
            // old block without invalid_transactions fields
            include_str!("../../../test_data/alonzo10.block"),
            // peculiar block with protocol update params
            include_str!("../../../test_data/alonzo11.block"),
            // peculiar block with decoding issue
            // https://github.com/txpipe/oura/issues/37
            include_str!("../../../test_data/alonzo12.block"),
            // peculiar block with protocol update params, including nonce
            include_str!("../../../test_data/alonzo13.block"),
            // peculiar block with overflow crash
            // https://github.com/txpipe/oura/issues/113
            include_str!("../../../test_data/alonzo14.block"),
            // peculiar block with many move-instantaneous-rewards certs
            include_str!("../../../test_data/alonzo15.block"),
            // peculiar block with protocol update values
            include_str!("../../../test_data/alonzo16.block"),
            // peculiar block with missing nonce hash
            include_str!("../../../test_data/alonzo17.block"),
            // peculiar block with strange AuxiliaryData variant
            include_str!("../../../test_data/alonzo18.block"),
            // peculiar block with strange AuxiliaryData variant
            include_str!("../../../test_data/alonzo18.block"),
            // peculiar block with nevative i64 overflow
            include_str!("../../../test_data/alonzo19.block"),
            // peculiar block with very BigInt in plutus code
            include_str!("../../../test_data/alonzo20.block"),
            // // peculiar block with bad tx hash
            // include_str!("../../../test_data/alonzo21.block"),
            // peculiar block with bad tx hash
            include_str!("../../../test_data/alonzo22.block"),
            // peculiar block with indef byte array in plutus data
            include_str!("../../../test_data/alonzo23.block"),
            // peculiar block with invalid address (pointer overflow)
            include_str!("../../../test_data/alonzo27.block"),
        ];

        for (idx, block_str) in test_blocks.iter().enumerate() {
            println!("decoding test block {}", idx + 1);
            let bytes = hex::decode(block_str).unwrap_or_else(|_| panic!("bad block file {idx}"));

            let block: BlockWrapper = minicbor::decode(&bytes[..])
                .unwrap_or_else(|_| panic!("error decoding cbor for file {idx}"));

            let bytes2 = to_vec(block)
                .unwrap_or_else(|_| panic!("error encoding block cbor for file {idx}"));

            assert!(bytes.eq(&bytes2), "re-encoded bytes didn't match original");
        }
    }

    #[test]
    fn header_isomorphic_decoding_encoding() {
        let test_headers = [
            // peculiar alonzo header used as origin for a vasil devnet
            include_str!("../../../test_data/alonzo26.header"),
        ];

        for (idx, header_str) in test_headers.iter().enumerate() {
            println!("decoding test header {}", idx + 1);
            let bytes = hex::decode(header_str).unwrap_or_else(|_| panic!("bad header file {idx}"));

            let header: Header = minicbor::decode(&bytes[..])
                .unwrap_or_else(|_| panic!("error decoding cbor for file {idx}"));

            let bytes2 = to_vec(header)
                .unwrap_or_else(|_| panic!("error encoding header cbor for file {idx}"));

            assert!(bytes.eq(&bytes2), "re-encoded bytes didn't match original");
        }
    }

    #[test]
    fn plutus_data_isomorphic_decoding_encoding() {
        let datas = [
            // unit = Constr 0 []
            "d87980",
            // pltmap = Map [(I 1, unit), (I 2, pltlist)]
            "a201d87980029f000102ff",
            // pltlist = List [I 0, I 1, I 2]
            "9f000102ff",
            // Constr 5 [pltmap, Constr 5 [Map [(pltmap, toData True), (pltlist, pltmap), (List [], List [I 1])], unit, toData (0, 1)]]
            "d87e9fa201d87980029f000102ffd87e9fa3a201d87980029f000102ffd87a809f000102ffa201d87980029f000102ff809f01ffd87980d8799f0001ffffff",
            // Constr 5 [List [], List [I 1], Map [], Map [(I 1, unit), (I 2, Constr 2 [I 2])]]
            "d87e9f809f01ffa0a201d8798002d87b9f02ffff",
            // B (B.replicate 32 105)
            "58206969696969696969696969696969696969696969696969696969696969696969",
            // B (B.replicate 67 105)
            "5f58406969696969696969696969696969696969696969696969696969696969696969696969696969696969696969696969696969696969696969696969696969696943696969ff",
            // B B.empty
            "40",
        ];
        for data_hex in datas {
            let data_bytes = hex::decode(data_hex).unwrap();
            let data = PlutusData::decode_fragment(&data_bytes).unwrap();
            assert_eq!(data.encode_fragment().unwrap(), data_bytes);
        }
    }
}
