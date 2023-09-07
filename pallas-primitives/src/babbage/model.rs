//! Ledger primitives and cbor codec for the Alonzo era
//!
//! Handcrafted, idiomatic rust artifacts based on based on the [Babbage CDDL](https://github.com/input-output-hk/cardano-ledger/blob/master/eras/babbage/test-suite/cddl-files/babbage.cddl) file in IOHK repo.

use serde::{Deserialize, Serialize};

use pallas_codec::minicbor::{Decode, Encode};
use pallas_crypto::hash::Hash;

use pallas_codec::utils::{Bytes, CborWrap, KeepRaw, KeyValuePairs, MaybeIndefArray, Nullable};

// required for derive attrs to work
use pallas_codec::minicbor;

pub use crate::alonzo::VrfCert;

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

pub use crate::alonzo::ProtocolVersion;

pub use crate::alonzo::KesSignature;

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub struct Header {
    #[n(0)]
    pub header_body: HeaderBody,

    #[n(1)]
    pub body_signature: Bytes,
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

pub type RequiredSigners = Vec<AddrKeyhash>;

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

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[cbor(index_only)]
pub enum Language {
    #[n(0)]
    PlutusV1,

    #[n(1)]
    PlutusV2,
}

pub use crate::alonzo::CostModel;

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[cbor(map)]
pub struct CostMdls {
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

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub struct Update {
    #[n(0)]
    pub proposed_protocol_parameter_updates: KeyValuePairs<Genesishash, ProtocolParamUpdate>,

    #[n(1)]
    pub epoch: Epoch,
}

#[derive(Encode, Decode, Debug, PartialEq, Clone)]
#[cbor(map)]
pub struct PseudoTransactionBody<T1> {
    #[n(0)]
    pub inputs: Vec<TransactionInput>,

    #[n(1)]
    pub outputs: Vec<T1>,

    #[n(2)]
    pub fee: u64,

    #[n(3)]
    pub ttl: Option<u64>,

    #[n(4)]
    pub certificates: Option<Vec<Certificate>>,

    #[n(5)]
    pub withdrawals: Option<KeyValuePairs<RewardAccount, Coin>>,

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
    pub collateral_return: Option<T1>,

    #[n(17)]
    pub total_collateral: Option<Coin>,

    #[n(18)]
    pub reference_inputs: Option<Vec<TransactionInput>>,
}

pub type TransactionBody = PseudoTransactionBody<TransactionOutput>;

pub type MintedTransactionBody<'a> = PseudoTransactionBody<MintedTransactionOutput<'a>>;

impl<'a> From<MintedTransactionBody<'a>> for TransactionBody {
    fn from(value: MintedTransactionBody<'a>) -> Self {
        Self {
            inputs: value.inputs,
            outputs: value.outputs.into_iter().map(|x| x.into()).collect(),
            fee: value.fee,
            ttl: value.ttl,
            certificates: value.certificates,
            withdrawals: value.withdrawals,
            update: value.update,
            auxiliary_data_hash: value.auxiliary_data_hash,
            validity_interval_start: value.validity_interval_start,
            mint: value.mint,
            script_data_hash: value.script_data_hash,
            collateral: value.collateral,
            required_signers: value.required_signers,
            network_id: value.network_id,
            collateral_return: value.collateral_return.map(|x| x.into()),
            total_collateral: value.total_collateral,
            reference_inputs: value.reference_inputs,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum PseudoTransactionOutput<T> {
    Legacy(LegacyTransactionOutput),
    PostAlonzo(T),
}

impl<'b, C, T> minicbor::Decode<'b, C> for PseudoTransactionOutput<T>
where
    T: minicbor::Decode<'b, C>,
{
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        match d.datatype()? {
            minicbor::data::Type::Array | minicbor::data::Type::ArrayIndef => {
                Ok(PseudoTransactionOutput::Legacy(d.decode_with(ctx)?))
            }
            minicbor::data::Type::Map | minicbor::data::Type::MapIndef => {
                Ok(PseudoTransactionOutput::PostAlonzo(d.decode_with(ctx)?))
            }
            _ => Err(minicbor::decode::Error::message(
                "invalid type for transaction output struct",
            )),
        }
    }
}

impl<C, T> minicbor::Encode<C> for PseudoTransactionOutput<T>
where
    T: minicbor::Encode<C>,
{
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            PseudoTransactionOutput::Legacy(x) => x.encode(e, ctx),
            PseudoTransactionOutput::PostAlonzo(x) => x.encode(e, ctx),
        }
    }
}

pub type TransactionOutput = PseudoTransactionOutput<PostAlonzoTransactionOutput>;

pub type MintedTransactionOutput<'b> =
    PseudoTransactionOutput<MintedPostAlonzoTransactionOutput<'b>>;

impl<'b> From<MintedTransactionOutput<'b>> for TransactionOutput {
    fn from(value: MintedTransactionOutput<'b>) -> Self {
        match value {
            PseudoTransactionOutput::Legacy(x) => Self::Legacy(x),
            PseudoTransactionOutput::PostAlonzo(x) => Self::PostAlonzo(x.into()),
        }
    }
}

#[derive(Encode, Decode, Debug, PartialEq, Clone)]
#[cbor(map)]
pub struct PseudoPostAlonzoTransactionOutput<T1> {
    #[n(0)]
    pub address: Bytes,

    #[n(1)]
    pub value: Value,

    #[n(2)]
    pub datum_option: Option<T1>,

    #[n(3)]
    pub script_ref: Option<ScriptRef>,
}

pub type PostAlonzoTransactionOutput = PseudoPostAlonzoTransactionOutput<DatumOption>;

pub type MintedPostAlonzoTransactionOutput<'b> =
    PseudoPostAlonzoTransactionOutput<MintedDatumOption<'b>>;

impl<'b> From<MintedPostAlonzoTransactionOutput<'b>> for PostAlonzoTransactionOutput {
    fn from(value: MintedPostAlonzoTransactionOutput<'b>) -> Self {
        Self {
            address: value.address,
            value: value.value,
            datum_option: value.datum_option.map(|x| x.into()),
            script_ref: value.script_ref,
        }
    }
}

pub use crate::alonzo::VKeyWitness;

pub use crate::alonzo::NativeScript;

pub use crate::alonzo::PlutusScript as PlutusV1Script;

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[cbor(transparent)]
pub struct PlutusV2Script(#[n(0)] pub Bytes);

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

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Clone)]
#[cbor(map)]
pub struct WitnessSet {
    #[n(0)]
    pub vkeywitness: Option<Vec<VKeyWitness>>,

    #[n(1)]
    pub native_script: Option<Vec<NativeScript>>,

    #[n(2)]
    pub bootstrap_witness: Option<Vec<BootstrapWitness>>,

    #[n(3)]
    pub plutus_v1_script: Option<Vec<PlutusV1Script>>,

    #[n(4)]
    pub plutus_data: Option<Vec<PlutusData>>,

    #[n(5)]
    pub redeemer: Option<Vec<Redeemer>>,

    #[n(6)]
    pub plutus_v2_script: Option<Vec<PlutusV2Script>>,
}

#[derive(Encode, Decode, Debug, PartialEq, Clone)]
#[cbor(map)]
pub struct MintedWitnessSet<'b> {
    #[n(0)]
    pub vkeywitness: Option<Vec<VKeyWitness>>,

    #[n(1)]
    pub native_script: Option<Vec<NativeScript>>,

    #[n(2)]
    pub bootstrap_witness: Option<Vec<BootstrapWitness>>,

    #[n(3)]
    pub plutus_v1_script: Option<Vec<PlutusV1Script>>,

    #[b(4)]
    pub plutus_data: Option<Vec<KeepRaw<'b, PlutusData>>>,

    #[n(5)]
    pub redeemer: Option<Vec<Redeemer>>,

    #[n(6)]
    pub plutus_v2_script: Option<Vec<PlutusV2Script>>,
}

impl<'b> From<MintedWitnessSet<'b>> for WitnessSet {
    fn from(x: MintedWitnessSet<'b>) -> Self {
        WitnessSet {
            vkeywitness: x.vkeywitness,
            native_script: x.native_script,
            bootstrap_witness: x.bootstrap_witness,
            plutus_v1_script: x.plutus_v1_script,
            plutus_data: x
                .plutus_data
                .map(|x| x.into_iter().map(|x| x.unwrap()).collect()),
            redeemer: x.redeemer,
            plutus_v2_script: x.plutus_v2_script,
        }
    }
}

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Clone)]
#[cbor(map)]
pub struct PostAlonzoAuxiliaryData {
    #[n(0)]
    pub metadata: Option<Metadata>,

    #[n(1)]
    pub native_scripts: Option<Vec<NativeScript>>,

    #[n(2)]
    pub plutus_v1_scripts: Option<Vec<PlutusV1Script>>,

    #[n(3)]
    pub plutus_v2_scripts: Option<Vec<PlutusV2Script>>,
}

pub type DatumHash = Hash<32>;

//pub type Data = CborWrap<PlutusData>;

// datum_option = [ 0, $hash32 // 1, data ]
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum PseudoDatumOption<T1> {
    Hash(Hash<32>),
    Data(CborWrap<T1>),
}

impl<'b, C, T> minicbor::Decode<'b, C> for PseudoDatumOption<T>
where
    T: minicbor::Decode<'b, C>,
{
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        d.array()?;

        match d.u8()? {
            0 => Ok(Self::Hash(d.decode_with(ctx)?)),
            1 => Ok(Self::Data(d.decode_with(ctx)?)),
            _ => Err(minicbor::decode::Error::message(
                "invalid variant for datum option enum",
            )),
        }
    }
}

impl<C, T> minicbor::Encode<C> for PseudoDatumOption<T>
where
    T: minicbor::Encode<C>,
{
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

pub type DatumOption = PseudoDatumOption<PlutusData>;

pub type MintedDatumOption<'b> = PseudoDatumOption<KeepRaw<'b, PlutusData>>;

impl<'b> From<MintedDatumOption<'b>> for DatumOption {
    fn from(value: MintedDatumOption<'b>) -> Self {
        match value {
            PseudoDatumOption::Hash(x) => Self::Hash(x),
            PseudoDatumOption::Data(x) => Self::Data(CborWrap(x.unwrap().unwrap())),
        }
    }
}

// script_ref = #6.24(bytes .cbor script)
pub type ScriptRef = CborWrap<Script>;

// script = [ 0, native_script // 1, plutus_v1_script // 2, plutus_v2_script ]
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
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

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Clone)]
pub struct PseudoBlock<T1, T2, T3, T4>
where
    T4: std::clone::Clone,
{
    #[n(0)]
    pub header: T1,

    #[b(1)]
    pub transaction_bodies: MaybeIndefArray<T2>,

    #[n(2)]
    pub transaction_witness_sets: MaybeIndefArray<T3>,

    #[n(3)]
    pub auxiliary_data_set: KeyValuePairs<TransactionIndex, T4>,

    #[n(4)]
    pub invalid_transactions: Option<MaybeIndefArray<TransactionIndex>>,
}

pub type Block = PseudoBlock<Header, TransactionBody, WitnessSet, AuxiliaryData>;

/// A memory representation of an already minted block
///
/// This structure is analogous to [Block], but it allows to retrieve the
/// original CBOR bytes for each structure that might require hashing. In this
/// way, we make sure that the resulting hash matches what exists on-chain.
pub type MintedBlock<'b> = PseudoBlock<
    KeepRaw<'b, Header>,
    KeepRaw<'b, MintedTransactionBody<'b>>,
    KeepRaw<'b, MintedWitnessSet<'b>>,
    KeepRaw<'b, AuxiliaryData>,
>;

impl<'b> From<MintedBlock<'b>> for Block {
    fn from(x: MintedBlock<'b>) -> Self {
        Block {
            header: x.header.unwrap(),
            transaction_bodies: MaybeIndefArray::Def(
                x.transaction_bodies
                    .iter()
                    .cloned()
                    .map(|x| x.unwrap())
                    .map(TransactionBody::from)
                    .collect(),
            ),
            transaction_witness_sets: MaybeIndefArray::Def(
                x.transaction_witness_sets
                    .iter()
                    .cloned()
                    .map(|x| x.unwrap())
                    .map(WitnessSet::from)
                    .collect(),
            ),
            auxiliary_data_set: x
                .auxiliary_data_set
                .to_vec()
                .into_iter()
                .map(|(k, v)| (k, v.unwrap()))
                .collect::<Vec<_>>()
                .into(),
            invalid_transactions: x.invalid_transactions,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Encode, Decode, Debug)]
pub struct PseudoTx<T1, T2, T3>
where
    T1: std::clone::Clone,
    T2: std::clone::Clone,
    T3: std::clone::Clone,
{
    #[n(0)]
    pub transaction_body: T1,

    #[n(1)]
    pub transaction_witness_set: T2,

    #[n(2)]
    pub success: bool,

    #[n(3)]
    pub auxiliary_data: Nullable<T3>,
}

pub type Tx = PseudoTx<TransactionBody, WitnessSet, AuxiliaryData>;

pub type MintedTx<'b> = PseudoTx<
    KeepRaw<'b, MintedTransactionBody<'b>>,
    KeepRaw<'b, MintedWitnessSet<'b>>,
    KeepRaw<'b, AuxiliaryData>,
>;

impl<'b> From<MintedTx<'b>> for Tx {
    fn from(x: MintedTx<'b>) -> Self {
        Tx {
            transaction_body: x.transaction_body.unwrap().into(),
            transaction_witness_set: x.transaction_witness_set.unwrap().into(),
            success: x.success,
            auxiliary_data: x.auxiliary_data.map(|x| x.unwrap()),
        }
    }
}

#[cfg(test)]
mod tests {
    use pallas_codec::minicbor;

    use super::{MintedBlock, TransactionOutput};
    use crate::Fragment;

    type BlockWrapper<'b> = (u16, MintedBlock<'b>);

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
            // block with inline datum that fails hashes
            include_str!("../../../test_data/babbage9.block"),
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
