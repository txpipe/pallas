//! Ledger primitives and cbor codec for the Byron era
//!
//! Handcrafted, idiomatic rust artifacts based on based on the [Byron CDDL](https://github.com/input-output-hk/cardano-ledger/blob/master/eras/byron/cddl-spec/byron.cddl) file in IOHK repo.

use log::warn;
use minicbor::{bytes::ByteVec, data::Tag};
use minicbor_derive::{Decode, Encode};
use pallas_crypto::hash::Hash;
use std::{collections::BTreeMap, iter::Skip, ops::Deref};

use crate::utils::{CborWrap, KeyValuePairs, MaybeIndefArray, OrderPreservingProperties};

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord)]
pub struct SkipCbor<const N: usize> {}

impl<'b, const N: usize> minicbor::Decode<'b> for SkipCbor<N> {
    fn decode(d: &mut minicbor::Decoder<'b>) -> Result<Self, minicbor::decode::Error> {
        {
            let probe = d.probe();
            warn!("skipped cbor value {}: {:?}", N, probe.datatype()?);
            println!("skipped cbor value {}: {:?}", N, probe.datatype()?);
        }

        d.skip()?;
        Ok(SkipCbor {})
    }
}

impl<const N: usize> minicbor::Encode for SkipCbor<N> {
    fn encode<W: minicbor::encode::Write>(
        &self,
        _e: &mut minicbor::Encoder<W>,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        todo!()
    }
}

// Basic Cardano Types

pub type Blake2b256 = Hash<32>;

pub type TxId = Blake2b256;
pub type BlockId = Blake2b256;
pub type UpdId = Blake2b256;
pub type ByronHash = Blake2b256;

pub type Blake2b224 = Hash<28>;

pub type AddressId = Blake2b224;
pub type StakeholderId = Blake2b224;

pub type EpochId = u64;

#[derive(Encode, Decode, Debug)]
pub struct SlotId {
    #[n(0)]
    epoch: EpochId,

    #[n(1)]
    slot: u64,
}

pub type PubKey = ByteVec;
pub type Signature = ByteVec;

// Attributes

// quote from the CDDL file: at the moment we do not bother deserialising these,
// since they don't contain anything

// attributes = {* any => any}
pub type Attributes = Vec<SkipCbor<0>>;

// Addresses

#[derive(Debug)]
pub enum AddrDistr {
    Variant0(StakeholderId),
    Variant1,
}

impl<'b> minicbor::Decode<'b> for AddrDistr {
    fn decode(d: &mut minicbor::Decoder<'b>) -> Result<Self, minicbor::decode::Error> {
        d.array()?;
        let variant = d.u32()?;

        match variant {
            0 => Ok(AddrDistr::Variant0(d.decode()?)),
            1 => Ok(AddrDistr::Variant1),
            _ => Err(minicbor::decode::Error::Message(
                "invalid variant for addrdstr",
            )),
        }
    }
}

impl minicbor::Encode for AddrDistr {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            AddrDistr::Variant0(x) => {
                e.array(2)?;
                e.u32(0)?;
                e.encode(x)?;

                Ok(())
            }
            AddrDistr::Variant1 => {
                e.array(1)?;
                e.u32(1)?;

                Ok(())
            }
        }
    }
}

#[derive(Debug)]
pub enum AddrType {
    PubKey,
    Script,
    Redeem,
    Other(u64),
}

impl<'b> minicbor::Decode<'b> for AddrType {
    fn decode(d: &mut minicbor::Decoder<'b>) -> Result<Self, minicbor::decode::Error> {
        let variant = d.u64()?;

        match variant {
            0 => Ok(AddrType::PubKey),
            1 => Ok(AddrType::Script),
            2 => Ok(AddrType::Redeem),
            x => Ok(AddrType::Other(x)),
        }
    }
}

impl minicbor::Encode for AddrType {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            AddrType::PubKey => e.u64(0)?,
            AddrType::Script => e.u64(1)?,
            AddrType::Redeem => e.u64(2)?,
            AddrType::Other(x) => e.u64(*x)?,
        };

        Ok(())
    }
}

#[derive(Debug)]
pub enum AddrAttrProperty {
    AddrDistr(AddrDistr),
    Bytes(ByteVec),
}

impl<'b> minicbor::Decode<'b> for AddrAttrProperty {
    fn decode(d: &mut minicbor::Decoder<'b>) -> Result<Self, minicbor::decode::Error> {
        d.array()?;

        let key = d.u32()?;

        match key {
            0 => Ok(AddrAttrProperty::AddrDistr(d.decode()?)),
            1 => Ok(AddrAttrProperty::Bytes(d.decode()?)),
            _ => Err(minicbor::decode::Error::Message(
                "unknown variant for addrattr property",
            )),
        }
    }
}

impl minicbor::Encode for AddrAttrProperty {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            AddrAttrProperty::AddrDistr(x) => {
                e.array(2)?;
                e.u32(0)?;
                e.encode(x)?;

                Ok(())
            }
            AddrAttrProperty::Bytes(x) => {
                e.array(2)?;
                e.u32(1)?;
                e.encode(x)?;

                Ok(())
            }
        }
    }
}

pub type AddrAttr = OrderPreservingProperties<AddrAttrProperty>;

// address = [ #6.24(bytes .cbor ([addressid, addrattr, addrtype])), u64 ]
pub type Address = (CborWrap<(AddressId, AddrAttr, AddrType)>, u64);

// Transactions

// txout = [address, u64]
pub type TxOut = (Address, u64);

#[derive(Debug)]
pub enum TxIn {
    // [0, #6.24(bytes .cbor ([txid, u32]))]
    Variant0(CborWrap<(TxId, u32)>),

    // [u8 .ne 0, encoded-cbor]
    Other(u8, ByteVec),
}

impl<'b> minicbor::Decode<'b> for TxIn {
    fn decode(d: &mut minicbor::Decoder<'b>) -> Result<Self, minicbor::decode::Error> {
        d.array()?;

        let variant = d.u8()?;

        match variant {
            0 => Ok(TxIn::Variant0(d.decode()?)),
            x => Ok(TxIn::Other(x, d.decode()?)),
        }
    }
}

impl minicbor::Encode for TxIn {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            TxIn::Variant0(x) => {
                e.array(2)?;
                e.u8(0)?;
                e.encode(x)?;

                Ok(())
            }
            TxIn::Other(a, b) => {
                e.array(2)?;
                e.u8(*a)?;
                e.encode(b)?;

                Ok(())
            }
        }
    }
}

// tx = [[+ txin], [+ txout], attributes]
pub type Tx = (Vec<TxIn>, Vec<TxOut>, Attributes);

// txproof = [u32, hash, hash]
pub type TxProof = (u32, ByronHash, ByronHash);

#[derive(Debug)]
pub enum Twit {
    // [0, #6.24(bytes .cbor ([pubkey, signature]))]
    Variant0(CborWrap<(PubKey, Signature)>),

    // [1, #6.24(bytes .cbor ([[u16, bytes], [u16, bytes]]))]
    Variant1(CborWrap<((u16, ByteVec), (u16, ByteVec))>),

    //[2, #6.24(bytes .cbor ([pubkey, signature]))]
    Variant2(CborWrap<(PubKey, Signature)>),

    // [u8 .gt 2, encoded-cbor]
    Other(u8, ByteVec),
}

impl<'b> minicbor::Decode<'b> for Twit {
    fn decode(d: &mut minicbor::Decoder<'b>) -> Result<Self, minicbor::decode::Error> {
        d.array()?;

        let variant = d.u8()?;

        match variant {
            0 => Ok(Twit::Variant0(d.decode()?)),
            1 => Ok(Twit::Variant1(d.decode()?)),
            2 => Ok(Twit::Variant2(d.decode()?)),
            x => Ok(Twit::Other(x, d.decode()?)),
        }
    }
}

impl minicbor::Encode for Twit {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            Twit::Variant0(x) => {
                e.array(2)?;
                e.u8(0)?;
                e.encode(x)?;

                Ok(())
            }
            Twit::Variant1(x) => {
                e.array(2)?;
                e.u8(1)?;
                e.encode(x)?;

                Ok(())
            }
            Twit::Variant2(x) => {
                e.array(2)?;
                e.u8(2)?;
                e.encode(x)?;

                Ok(())
            }
            Twit::Other(a, b) => {
                e.array(2)?;
                e.u8(*a)?;
                e.encode(b)?;

                Ok(())
            }
        }
    }
}

// Blocks

pub type Difficulty = Vec<u64>;

pub enum BlockSig {
    Signature(SkipCbor<66>),
    LwdlgSig(SkipCbor<66>),
    DlgSig(SkipCbor<66>),
}

#[derive(Encode, Decode, Debug)]
pub struct BlockCons(
    #[n(0)] SlotId,
    #[n(1)] PubKey,
    #[n(2)] Difficulty,
    #[n(3)] SkipCbor<55>, //BlockSig,
);

#[derive(Encode, Decode, Debug)]
pub struct BlockHeadEx {
    #[n(0)]
    block_version: SkipCbor<77>, // bver

    #[n(1)]
    software_version: (String, u32),

    #[n(2)]
    attributes: SkipCbor<77>, //attributes

    #[n(3)]
    extra_proof: ByronHash,
}

#[derive(Encode, Decode, Debug)]
pub struct BlockProof {
    #[n(0)]
    tx_proof: SkipCbor<44>, // txproof,

    #[n(1)]
    ssc_proof: SkipCbor<44>, // sscproof,

    #[n(2)]
    dlg_proof: ByronHash,

    #[n(3)]
    upd_proof: ByronHash,
}

#[derive(Encode, Decode, Debug)]
pub struct BlockHead {
    #[n(0)]
    protocol_magic: u32,

    #[n(1)]
    prev_block: BlockId,

    #[n(2)]
    body_proof: BlockProof,

    #[n(3)]
    consensus_data: BlockCons,

    #[n(4)]
    extra_data: BlockHeadEx,
}

// [tx, [* twit]]
pub type TxPayload = (Tx, Vec<Twit>);

#[derive(Encode, Decode, Debug)]
pub struct BlockBody {
    #[n(0)]
    tx_payload: Vec<TxPayload>,

    #[n(1)]
    ssc_payload: SkipCbor<99>, // ssc

    #[n(2)]
    dlg_payload: SkipCbor<99>, // [* dlg]

    #[n(3)]
    upd_payload: SkipCbor<99>, // up
}

// Epoch Boundary Blocks

#[derive(Encode, Decode, Debug)]
pub struct EbbCons {
    #[n(0)]
    epoch_id: EpochId,

    #[n(1)]
    difficulty: Difficulty,
}

#[derive(Encode, Decode, Debug)]
pub struct EbbHead {
    #[n(0)]
    protocol_magic: u32,

    #[n(1)]
    prev_block: BlockId,

    #[n(2)]
    body_proof: SkipCbor<12>,

    #[n(3)]
    consensus_data: EbbCons,

    #[n(4)]
    extra_data: SkipCbor<14>, // [attributes]
}

#[derive(Encode, Decode, Debug)]
pub struct MainBlock {
    #[n(0)]
    header: BlockHead,

    #[n(1)]
    body: BlockBody,

    #[n(2)]
    extra: SkipCbor<12>, // Vec<Attributes>,
}

#[derive(Encode, Decode, Debug)]
pub struct EbBlock {
    #[n(0)]
    header: EbbHead,

    #[n(1)]
    body: Vec<StakeholderId>,

    #[n(2)]
    extra: SkipCbor<2>, // Vec<Attributes>,
}

#[derive(Debug)]
pub enum Block {
    MainBlock(MainBlock),
    EbBlock(EbBlock),
}

impl<'b> minicbor::Decode<'b> for Block {
    fn decode(d: &mut minicbor::Decoder<'b>) -> Result<Self, minicbor::decode::Error> {
        d.array()?;

        let variant = d.u32()?;

        match variant {
            0 => Ok(Block::EbBlock(d.decode()?)),
            1 => Ok(Block::MainBlock(d.decode()?)),
            _ => Err(minicbor::decode::Error::Message(
                "unknown variant for block",
            )),
        }
    }
}

impl minicbor::Encode for Block {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            Block::EbBlock(x) => {
                e.array(2)?;
                e.encode(0)?;
                e.encode(x)?;

                Ok(())
            }
            Block::MainBlock(x) => {
                e.array(2)?;
                e.encode(1)?;
                e.encode(x)?;

                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{Block, Fragment};
    use minicbor::{self, to_vec};

    #[test]
    fn block_isomorphic_decoding_encoding() {
        let test_blocks = vec![include_str!("test_data/test1.block")];

        for (idx, block_str) in test_blocks.iter().enumerate() {
            println!("decoding test block {}", idx + 1);
            let bytes = hex::decode(block_str).expect(&format!("bad block file {}", idx));

            let block = Block::decode_fragment(&bytes[..])
                .expect(&format!("error decoding cbor for file {}", idx));

            println!("{:?}", block);

            let bytes2 =
                to_vec(block).expect(&format!("error encoding block cbor for file {}", idx));

            assert_eq!(bytes, bytes2);
        }
    }
}
