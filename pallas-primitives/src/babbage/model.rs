//! Ledger primitives and cbor codec for the Alonzo era
//!
//! Handcrafted, idiomatic rust artifacts based on based on the [Babbage CDDL](https://github.com/input-output-hk/cardano-ledger/blob/master/eras/babbage/test-suite/cddl-files/babbage.cddl) file in IOHK repo.

use pallas_codec::minicbor::{bytes::ByteVec, Decode, Encode};
use pallas_crypto::hash::Hash;

use pallas_codec::utils::{CborWrap, KeepRaw, KeyValuePairs, MaybeIndefArray, Nullable};

// required for derive attrs to work
use pallas_codec::minicbor;

pub use crate::alonzo::VrfCert;

#[derive(Encode, Decode, Debug, PartialEq, Clone)]
pub struct HeaderBody {
    #[n(0)]
    pub block_number: u64,

    #[n(1)]
    pub slot: u64,

    #[n(2)]
    pub prev_hash: Option<Hash<32>>,

    #[n(3)]
    pub issuer_vkey: ByteVec,

    #[n(4)]
    pub vrf_vkey: ByteVec,

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

#[derive(Encode, Decode, Debug, Clone, PartialEq, PartialOrd)]
pub struct OperationalCert {
    #[n(0)]
    pub operational_cert_hot_vkey: ByteVec,

    #[n(1)]
    pub operational_cert_sequence_number: u64,

    #[n(2)]
    pub operational_cert_kes_period: u64,

    #[n(3)]
    pub operational_cert_sigma: ByteVec,
}

pub use crate::alonzo::ProtocolVersion;

pub use crate::alonzo::KesSignature;

#[derive(Encode, Decode, Debug, PartialEq, Clone)]
pub struct Header {
    #[n(0)]
    pub header_body: HeaderBody,

    #[n(1)]
    pub body_signature: ByteVec,
}

pub use crate::alonzo::TransactionInput;

pub use crate::alonzo::NonceVariant;

pub use crate::alonzo::Nonce;

pub use crate::alonzo::ScriptHash;

pub use crate::alonzo::PolicyId;

pub use crate::alonzo::AssetName;

pub use crate::alonzo::Multiasset;

pub use crate::alonzo::Mint;

pub use crate::alonzo::Coin;

pub use crate::alonzo::Value;

pub use crate::alonzo::TransactionOutput as LegacyTransactionOutput;

pub use crate::alonzo::PoolKeyhash;

pub use crate::alonzo::Epoch;

pub use crate::alonzo::Genesishash;

pub use crate::alonzo::GenesisDelegateHash;

pub use crate::alonzo::VrfKeyhash;

pub use crate::alonzo::InstantaneousRewardSource;

pub use crate::alonzo::InstantaneousRewardTarget;

pub use crate::alonzo::MoveInstantaneousReward;

pub use crate::alonzo::RewardAccount;

pub type Withdrawals = KeyValuePairs<RewardAccount, Coin>;

pub type RequiredSigners = MaybeIndefArray<AddrKeyhash>;

pub use crate::alonzo::Port;

pub use crate::alonzo::IPv4;

pub use crate::alonzo::IPv6;

pub use crate::alonzo::DnsName;

pub use crate::alonzo::Relay;

pub use crate::alonzo::PoolMetadataHash;

pub use crate::alonzo::PoolMetadata;

pub use crate::alonzo::AddrKeyhash;

pub use crate::alonzo::Scripthash;

pub use crate::alonzo::RationalNumber;

pub use crate::alonzo::UnitInterval;

pub use crate::alonzo::PositiveInterval;

pub use crate::alonzo::StakeCredential;

pub use crate::alonzo::Certificate;

pub use crate::alonzo::NetworkId;

#[derive(Encode, Decode, Debug, PartialEq, Clone)]
#[cbor(index_only)]
pub enum Language {
    #[n(0)]
    PlutusV1,

    #[n(1)]
    PlutusV2,
}

pub use crate::alonzo::CostModel;

#[derive(Encode, Decode, Debug, PartialEq, Clone)]
#[cbor(map)]
pub struct CostMdls {
    #[n(0)]
    pub plutus_v1: Option<CostModel>,

    #[n(1)]
    pub plutus_v2: Option<CostModel>,
}

#[derive(Encode, Decode, Debug, PartialEq, Clone)]
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
    pub cost_models_for_script_languages: Option<CostMdls>,
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

#[derive(Encode, Decode, Debug, PartialEq, Clone)]
pub struct Update {
    #[n(0)]
    pub proposed_protocol_parameter_updates: KeyValuePairs<Genesishash, ProtocolParamUpdate>,

    #[n(1)]
    pub epoch: Epoch,
}

#[derive(Encode, Decode, Debug, PartialEq, Clone)]
#[cbor(map)]
pub struct TransactionBody {
    #[n(0)]
    pub inputs: MaybeIndefArray<TransactionInput>,

    #[n(1)]
    pub outputs: MaybeIndefArray<TransactionOutput>,

    #[n(2)]
    pub fee: u64,

    #[n(3)]
    pub ttl: Option<u64>,

    #[n(4)]
    pub certificates: Option<MaybeIndefArray<Certificate>>,

    #[n(5)]
    pub withdrawals: Option<KeyValuePairs<RewardAccount, Coin>>,

    #[n(6)]
    pub update: Option<Update>,

    #[n(7)]
    pub auxiliary_data_hash: Option<ByteVec>,

    #[n(8)]
    pub validity_interval_start: Option<u64>,

    #[n(9)]
    pub mint: Option<Multiasset<i64>>,

    #[n(11)]
    pub script_data_hash: Option<Hash<32>>,

    #[n(13)]
    pub collateral: Option<MaybeIndefArray<TransactionInput>>,

    #[n(14)]
    pub required_signers: Option<MaybeIndefArray<AddrKeyhash>>,

    #[n(15)]
    pub network_id: Option<NetworkId>,

    #[n(16)]
    pub collateral_return: Option<TransactionOutput>,

    #[n(17)]
    pub total_collateral: Option<Coin>,

    #[n(18)]
    pub reference_inputs: Option<MaybeIndefArray<TransactionInput>>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum TransactionOutput {
    Legacy(LegacyTransactionOutput),
    PostAlonzo(PostAlonzoTransactionOutput),
}

impl<'b, C> minicbor::Decode<'b, C> for TransactionOutput {
    fn decode(
        d: &mut minicbor::Decoder<'b>,
        _ctx: &mut C,
    ) -> Result<Self, minicbor::decode::Error> {
        match d.datatype()? {
            minicbor::data::Type::Array | minicbor::data::Type::ArrayIndef => {
                Ok(TransactionOutput::Legacy(d.decode()?))
            }
            minicbor::data::Type::Map | minicbor::data::Type::MapIndef => {
                Ok(TransactionOutput::PostAlonzo(d.decode()?))
            }
            _ => Err(minicbor::decode::Error::message(
                "invalid type for transaction output struct",
            )),
        }
    }
}

impl<C> minicbor::Encode<C> for TransactionOutput {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            TransactionOutput::Legacy(x) => x.encode(e, ctx),
            TransactionOutput::PostAlonzo(x) => x.encode(e, ctx),
        }
    }
}

#[derive(Encode, Decode, Debug, PartialEq, Clone)]
#[cbor(map)]
pub struct PostAlonzoTransactionOutput {
    #[n(0)]
    pub address: ByteVec,

    #[n(1)]
    pub value: Value,

    #[n(2)]
    pub datum_option: Option<DatumOption>,

    #[n(3)]
    pub script_ref: Option<ScriptRef>,
}

pub use crate::alonzo::VKeyWitness;

pub use crate::alonzo::NativeScript;

pub use crate::alonzo::PlutusScript as PlutusV1Script;

#[derive(Encode, Decode, Debug, PartialEq, Clone)]
#[cbor(transparent)]
pub struct PlutusV2Script(#[n(0)] pub ByteVec);

impl AsRef<[u8]> for PlutusV2Script {
    fn as_ref(&self) -> &[u8] {
        self.0.as_slice()
    }
}

pub use crate::alonzo::BigInt;

pub use crate::alonzo::PlutusData;

pub use crate::alonzo::Constr;

pub use crate::alonzo::ExUnits;

pub use crate::alonzo::ExUnitPrices;

pub use crate::alonzo::RedeemerTag;

pub use crate::alonzo::Redeemer;

pub use crate::alonzo::BootstrapWitness;

#[derive(Encode, Decode, Debug, PartialEq, Clone)]
#[cbor(map)]
pub struct TransactionWitnessSet {
    #[n(0)]
    pub vkeywitness: Option<MaybeIndefArray<VKeyWitness>>,

    #[n(1)]
    pub native_script: Option<MaybeIndefArray<NativeScript>>,

    #[n(2)]
    pub bootstrap_witness: Option<MaybeIndefArray<BootstrapWitness>>,

    #[n(3)]
    pub plutus_v1_script: Option<MaybeIndefArray<PlutusV1Script>>,

    #[n(4)]
    pub plutus_data: Option<MaybeIndefArray<PlutusData>>,

    #[n(5)]
    pub redeemer: Option<MaybeIndefArray<Redeemer>>,

    #[n(6)]
    pub plutus_v2_script: Option<MaybeIndefArray<PlutusV2Script>>,
}

#[derive(Encode, Decode, Debug, PartialEq, Clone)]
#[cbor(map)]
pub struct PostAlonzoAuxiliaryData {
    #[n(0)]
    pub metadata: Option<Metadata>,

    #[n(1)]
    pub native_scripts: Option<MaybeIndefArray<NativeScript>>,

    #[n(2)]
    pub plutus_v1_scripts: Option<MaybeIndefArray<PlutusV1Script>>,

    #[n(3)]
    pub plutus_v2_scripts: Option<MaybeIndefArray<PlutusV2Script>>,
}

pub type DatumHash = Hash<32>;

pub type Data = CborWrap<PlutusData>;

// datum_option = [ 0, $hash32 // 1, data ]
#[derive(Debug, PartialEq, Clone)]
pub enum DatumOption {
    Hash(Hash<32>),
    Data(Data),
}

impl<'b, C> minicbor::Decode<'b, C> for DatumOption {
    fn decode(
        d: &mut minicbor::Decoder<'b>,
        _ctx: &mut C,
    ) -> Result<Self, minicbor::decode::Error> {
        d.array()?;

        match d.u8()? {
            0 => Ok(Self::Hash(d.decode()?)),
            1 => Ok(Self::Data(d.decode()?)),
            _ => Err(minicbor::decode::Error::message(
                "invalid variant for datum option enum",
            )),
        }
    }
}

impl<C> minicbor::Encode<C> for DatumOption {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            Self::Hash(x) => e.encode_with((0, x), ctx)?,
            Self::Data(x) => e.encode_with((1, x), ctx)?,
        };

        Ok(())
    }
}

// script_ref = #6.24(bytes .cbor script)
pub type ScriptRef = CborWrap<Script>;

// script = [ 0, native_script // 1, plutus_v1_script // 2, plutus_v2_script ]
#[derive(Debug, PartialEq, Clone)]
pub enum Script {
    NativeScript(NativeScript),
    PlutusV1Script(PlutusV1Script),
    PlutusV2Script(PlutusV2Script),
}

impl<'b, C> minicbor::Decode<'b, C> for Script {
    fn decode(
        d: &mut minicbor::Decoder<'b>,
        _ctx: &mut C,
    ) -> Result<Self, minicbor::decode::Error> {
        d.array()?;

        match d.u8()? {
            0 => Ok(Self::NativeScript(d.decode()?)),
            1 => Ok(Self::PlutusV1Script(d.decode()?)),
            2 => Ok(Self::PlutusV2Script(d.decode()?)),
            _ => Err(minicbor::decode::Error::message(
                "invalid variant for script enum",
            )),
        }
    }
}

impl<C> minicbor::Encode<C> for Script {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            Self::NativeScript(x) => e.encode_with((0, x), ctx)?,
            Self::PlutusV1Script(x) => e.encode_with((1, x), ctx)?,
            Self::PlutusV2Script(x) => e.encode_with((2, x), ctx)?,
        };

        Ok(())
    }
}

pub use crate::alonzo::Metadatum;

pub use crate::alonzo::MetadatumLabel;

pub use crate::alonzo::Metadata;

pub use crate::alonzo::AuxiliaryData;

pub use crate::alonzo::TransactionIndex;

#[derive(Encode, Decode, Debug, PartialEq, Clone)]
pub struct Block {
    #[n(0)]
    pub header: Header,

    #[b(1)]
    pub transaction_bodies: MaybeIndefArray<TransactionBody>,

    #[n(2)]
    pub transaction_witness_sets: MaybeIndefArray<TransactionWitnessSet>,

    #[n(3)]
    pub auxiliary_data_set: KeyValuePairs<TransactionIndex, AuxiliaryData>,

    #[n(4)]
    pub invalid_transactions: Option<MaybeIndefArray<TransactionIndex>>,
}

/// A memory representation of an already minted block
///
/// This structure is analogous to [Block], but it allows to retrieve the
/// original CBOR bytes for each structure that might require hashing. In this
/// way, we make sure that the resulting hash matches what exists on-chain.
#[derive(Encode, Decode, Debug, PartialEq, Clone)]
pub struct MintedBlock<'b> {
    #[n(0)]
    pub header: KeepRaw<'b, Header>,

    #[b(1)]
    pub transaction_bodies: MaybeIndefArray<KeepRaw<'b, TransactionBody>>,

    #[n(2)]
    pub transaction_witness_sets: MaybeIndefArray<KeepRaw<'b, TransactionWitnessSet>>,

    #[n(3)]
    pub auxiliary_data_set: KeyValuePairs<TransactionIndex, KeepRaw<'b, AuxiliaryData>>,

    #[n(4)]
    pub invalid_transactions: Option<MaybeIndefArray<TransactionIndex>>,
}

#[derive(Encode, Decode, Debug)]
pub struct Tx {
    #[n(0)]
    pub transaction_body: TransactionBody,

    #[n(1)]
    pub transaction_witness_set: TransactionWitnessSet,

    #[n(2)]
    pub success: bool,

    #[n(3)]
    pub auxiliary_data: Nullable<AuxiliaryData>,
}

#[derive(Encode, Decode, Debug, Clone)]
pub struct MintedTx<'b> {
    #[b(0)]
    pub transaction_body: KeepRaw<'b, TransactionBody>,

    #[n(1)]
    pub transaction_witness_set: KeepRaw<'b, TransactionWitnessSet>,

    #[n(2)]
    pub success: bool,

    #[n(3)]
    pub auxiliary_data: Nullable<KeepRaw<'b, AuxiliaryData>>,
}

#[cfg(test)]
mod tests {
    use pallas_codec::minicbor;

    use super::MintedBlock;

    type BlockWrapper<'b> = (u16, MintedBlock<'b>);

    #[test]
    fn block_isomorphic_decoding_encoding() {
        let test_blocks = vec![
            include_str!("../../../test_data/babbage1.block"),
            include_str!("../../../test_data/babbage2.block"),
            include_str!("../../../test_data/babbage3.block"),
            // peculiar block with single plutus cost model
            include_str!("../../../test_data/babbage4.block"),
        ];

        for (idx, block_str) in test_blocks.iter().enumerate() {
            println!("decoding test block {}", idx + 1);
            let bytes = hex::decode(block_str).expect(&format!("bad block file {}", idx));

            let block: BlockWrapper = minicbor::decode(&bytes[..])
                .expect(&format!("error decoding cbor for file {}", idx));

            let bytes2 = minicbor::to_vec(block)
                .expect(&format!("error encoding block cbor for file {}", idx));

            assert!(bytes.eq(&bytes2), "re-encoded bytes didn't match original");
        }
    }
}
