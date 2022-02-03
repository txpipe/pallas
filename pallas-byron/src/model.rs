//! Ledger primitives and cbor codec for the Byron era
//!
//! Handcrafted, idiomatic rust artifacts based on based on the [Byron CDDL](https://github.com/input-output-hk/cardano-ledger/blob/master/eras/byron/cddl-spec/byron.cddl) file in IOHK repo.

use log::warn;
use minicbor::{bytes::ByteVec, data::Tag};
use minicbor_derive::{Decode, Encode};
use pallas_crypto::hash::Hash;
use std::{collections::BTreeMap, iter::Skip, ops::Deref};

use crate::utils::{KeyValuePairs, MaybeIndefArray};

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

#[derive(Encode, Decode, Debug)]
pub struct BlockBody {
    #[n(0)]
    tx_payload: SkipCbor<99>, // [* [tx, [* twit]]]

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
    body: SkipCbor<1>, // Vec<StakeholderId>,

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
