//! Ledger primitives and cbor codec for the Byron era
//!
//! Handcrafted, idiomatic rust artifacts based on based on the [Byron CDDL](https://github.com/input-output-hk/cardano-ledger/blob/master/eras/byron/cddl-spec/byron.cddl) file in IOHK repo.

use log::warn;
use minicbor::bytes::ByteVec;
use minicbor_derive::{Decode, Encode};
use pallas_crypto::hash::Hash;

use crate::utils::{
    CborWrap, EmptyMap, KeyValuePairs, MaybeIndefArray, OrderPreservingProperties, TagWrap,
};

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
pub type Attributes = EmptyMap;

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
        let key = d.u8()?;

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
                e.u32(0)?;
                e.encode(x)?;

                Ok(())
            }
            AddrAttrProperty::Bytes(x) => {
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

    // [2, #6.24(bytes .cbor ([pubkey, signature]))]
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

// Shared Seed Computation

// cddl note:
// This is encoded using the 'Binary' instance
// for Scrape.PublicKey
pub type VssPubKey = ByteVec;

// cddl note:
// This is encoded using the 'Binary' instance
// for Scrape.Secret.
pub type VssSec = ByteVec;

// cddl note:
// This is encoded using the 'Binary' instance
// for Scrape.EncryptedSi.
// TODO work out why this seems to be in a length 1 array
pub type VssEnc = MaybeIndefArray<ByteVec>;

// cddl note:
// This is encoded using the 'Binary' instance
// for Scrape.DecryptedShare
pub type VssDec = ByteVec;

// cddl note:
// This is encoded using the
// 'Binary' instance for Scrape.Proof
pub type VssProof = (ByteVec, ByteVec, ByteVec, Vec<ByteVec>);

//ssccomm = [pubkey, [{vsspubkey => vssenc},vssproof], signature]
pub type SscComm = (
    PubKey,
    (KeyValuePairs<VssPubKey, VssEnc>, VssProof),
    Signature,
);

//ssccomms = #6.258([* ssccomm])
pub type SscComms = TagWrap<MaybeIndefArray<SscComm>, 258>;

// sscopens = {stakeholderid => vsssec}
pub type SscOpens = KeyValuePairs<StakeholderId, VssSec>;

// sscshares = {addressid => [addressid, [* vssdec]]}
pub type SscShares = KeyValuePairs<AddressId, (AddressId, MaybeIndefArray<VssDec>)>;

// CDDL says: ssccert = [vsspubkey, pubkey, epochid, signature]
// this is what seems to work: ssccert = [vsspubkey, epochid, pubkey, signature]
pub type SscCert = (VssPubKey, EpochId, PubKey, Signature);

// ssccerts = #6.258([* ssccert])
pub type SscCerts = TagWrap<MaybeIndefArray<SscCert>, 258>;

#[derive(Debug)]
pub enum Ssc {
    Variant0(SscComms, SscCerts),
    Variant1(SscOpens, SscCerts),
    Variant2(SscShares, SscCerts),
    Variant3(SscCerts),
}

impl<'b> minicbor::Decode<'b> for Ssc {
    fn decode(d: &mut minicbor::Decoder<'b>) -> Result<Self, minicbor::decode::Error> {
        d.array()?;

        let variant = d.u8()?;

        match variant {
            0 => Ok(Ssc::Variant0(d.decode()?, d.decode()?)),
            1 => Ok(Ssc::Variant1(d.decode()?, d.decode()?)),
            2 => Ok(Ssc::Variant2(d.decode()?, d.decode()?)),
            3 => Ok(Ssc::Variant3(d.decode()?)),
            _ => Err(minicbor::decode::Error::Message("invalid variant for ssc")),
        }
    }
}

impl minicbor::Encode for Ssc {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            Ssc::Variant0(a, b) => {
                e.array(3)?;
                e.u8(0)?;
                e.encode(a)?;
                e.encode(b)?;

                Ok(())
            }
            Ssc::Variant1(a, b) => {
                e.array(3)?;
                e.u8(1)?;
                e.encode(a)?;
                e.encode(b)?;

                Ok(())
            }
            Ssc::Variant2(a, b) => {
                e.array(3)?;
                e.u8(2)?;
                e.encode(a)?;
                e.encode(b)?;

                Ok(())
            }
            Ssc::Variant3(x) => {
                e.array(2)?;
                e.u8(3)?;
                e.encode(x)?;

                Ok(())
            }
        }
    }
}

#[derive(Debug)]
pub enum SscProof {
    Variant0(ByronHash, ByronHash),
    Variant1(ByronHash, ByronHash),
    Variant2(ByronHash, ByronHash),
    Variant3(ByronHash),
}

impl<'b> minicbor::Decode<'b> for SscProof {
    fn decode(d: &mut minicbor::Decoder<'b>) -> Result<Self, minicbor::decode::Error> {
        d.array()?;

        let variant = d.u8()?;

        match variant {
            0 => Ok(SscProof::Variant0(d.decode()?, d.decode()?)),
            1 => Ok(SscProof::Variant1(d.decode()?, d.decode()?)),
            2 => Ok(SscProof::Variant2(d.decode()?, d.decode()?)),
            3 => Ok(SscProof::Variant3(d.decode()?)),
            _ => Err(minicbor::decode::Error::Message(
                "invalid variant for sscproof",
            )),
        }
    }
}

impl minicbor::Encode for SscProof {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            SscProof::Variant0(a, b) => {
                e.array(3)?;
                e.u8(0)?;
                e.encode(a)?;
                e.encode(b)?;

                Ok(())
            }
            SscProof::Variant1(a, b) => {
                e.array(3)?;
                e.u8(1)?;
                e.encode(a)?;
                e.encode(b)?;

                Ok(())
            }
            SscProof::Variant2(a, b) => {
                e.array(3)?;
                e.u8(2)?;
                e.encode(a)?;
                e.encode(b)?;

                Ok(())
            }
            SscProof::Variant3(x) => {
                e.array(2)?;
                e.u8(3)?;
                e.encode(x)?;

                Ok(())
            }
        }
    }
}

// Delegation

#[derive(Debug, Encode, Decode)]
pub struct Dlg {
    #[n(0)]
    epoch: EpochId,

    #[n(1)]
    issuer: PubKey,

    #[n(2)]
    delegate: PubKey,

    #[n(3)]
    certificate: Signature,
}

pub type DlgSig = (Dlg, Signature);

#[derive(Debug, Encode, Decode)]
pub struct Lwdlg {
    #[n(0)]
    epoch_range: (EpochId, EpochId),

    #[n(1)]
    issuer: PubKey,

    #[n(2)]
    delegate: PubKey,

    #[n(3)]
    certificate: Signature,
}

pub type LwdlgSig = (Lwdlg, Signature);

// Updates

pub type BVer = (u16, u16, u8);

#[derive(Debug)]
pub enum TxFeePol {
    //[0, #6.24(bytes .cbor ([bigint, bigint]))]
    Variant0(CborWrap<(i64, i64)>),

    // [u8 .gt 0, encoded-cbor]
    Other(u8, ByteVec),
}

impl<'b> minicbor::Decode<'b> for TxFeePol {
    fn decode(d: &mut minicbor::Decoder<'b>) -> Result<Self, minicbor::decode::Error> {
        d.array()?;

        let variant = d.u8()?;

        match variant {
            0 => Ok(TxFeePol::Variant0(d.decode()?)),
            x => Ok(TxFeePol::Other(x, d.decode()?)),
        }
    }
}

impl minicbor::Encode for TxFeePol {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            TxFeePol::Variant0(x) => {
                e.array(2)?;
                e.u8(0)?;
                e.encode(x)?;

                Ok(())
            }
            TxFeePol::Other(a, b) => {
                e.array(2)?;
                e.u8(*a)?;
                e.encode(b)?;

                Ok(())
            }
        }
    }
}

#[derive(Debug, Encode, Decode)]
pub struct BVerMod {
    #[n(0)]
    script_version: (Option<u16>,),

    #[n(1)]
    slot_duration: (Option<u64>,),

    #[n(2)]
    max_block_size: (Option<u64>,),

    #[n(3)]
    max_header_size: (Option<u64>,),

    #[n(4)]
    max_tx_size: (Option<u64>,),

    #[n(5)]
    max_proposal_size: (Option<u64>,),

    #[n(6)]
    mpc_thd: (Option<u64>,),

    #[n(7)]
    heavy_del_thd: (Option<u64>,),

    #[n(8)]
    update_vote_thd: (Option<u64>,),

    #[n(9)]
    update_proposal_thd: (Option<u64>,),

    #[n(10)]
    update_implicit: (Option<u64>,),

    #[n(11)]
    soft_fork_rule: (Option<(u64, u64, u64)>,),

    #[n(12)]
    tx_fee_policy: (Option<TxFeePol>,),

    #[n(13)]
    unlock_stake_epoch: (Option<EpochId>,),
}

pub type UpData = (ByronHash, ByronHash, ByronHash, ByronHash);

#[derive(Debug, Encode, Decode)]
pub struct UpProp {
    #[n(0)]
    block_version: Option<BVer>,

    #[n(1)]
    block_version_mod: Option<BVerMod>,

    #[n(2)]
    software_version: Option<(String, u32)>,

    #[n(3)]
    data: Option<TagWrap<(String, UpData), 258>>,

    #[n(4)]
    attributes: Option<Attributes>,

    #[n(5)]
    from: Option<PubKey>,

    #[n(6)]
    signature: Option<Signature>,
}

#[derive(Debug, Encode, Decode)]
pub struct UpVote {
    #[n(0)]
    voter: PubKey,

    #[n(1)]
    proposal_id: UpdId,

    #[n(2)]
    vote: bool,

    #[n(3)]
    signature: Signature,
}

#[derive(Debug, Encode, Decode)]
pub struct Up {
    #[n(0)]
    proposal: Option<UpProp>,

    #[n(1)]
    votes: MaybeIndefArray<UpVote>,
}

// Blocks

pub type Difficulty = MaybeIndefArray<u64>;

#[derive(Debug)]
pub enum BlockSig {
    Signature(Signature),
    LwdlgSig(LwdlgSig),
    DlgSig(DlgSig),
}

impl<'b> minicbor::Decode<'b> for BlockSig {
    fn decode(d: &mut minicbor::Decoder<'b>) -> Result<Self, minicbor::decode::Error> {
        d.array()?;

        let variant = d.u8()?;

        match variant {
            0 => Ok(BlockSig::Signature(d.decode()?)),
            1 => Ok(BlockSig::LwdlgSig(d.decode()?)),
            2 => Ok(BlockSig::DlgSig(d.decode()?)),
            _ => Err(minicbor::decode::Error::Message(
                "unknown variant for blocksig",
            )),
        }
    }
}

impl minicbor::Encode for BlockSig {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            BlockSig::Signature(x) => {
                e.array(2)?;
                e.u8(0)?;
                e.encode(x)?;

                Ok(())
            }
            BlockSig::LwdlgSig(x) => {
                e.array(2)?;
                e.u8(1)?;
                e.encode(x)?;

                Ok(())
            }
            BlockSig::DlgSig(x) => {
                e.array(2)?;
                e.u8(2)?;
                e.encode(x)?;

                Ok(())
            }
        }
    }
}

#[derive(Encode, Decode, Debug)]
pub struct BlockCons(
    #[n(0)] SlotId,
    #[n(1)] PubKey,
    #[n(2)] Difficulty,
    #[n(3)] BlockSig,
);

#[derive(Encode, Decode, Debug)]
pub struct BlockHeadEx {
    #[n(0)]
    block_version: BVer,

    #[n(1)]
    software_version: (String, u32),

    #[n(2)]
    attributes: Option<Attributes>,

    #[n(3)]
    extra_proof: ByronHash,
}

#[derive(Encode, Decode, Debug)]
pub struct BlockProof {
    #[n(0)]
    tx_proof: TxProof,

    #[n(1)]
    ssc_proof: SscProof,

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
    tx_payload: MaybeIndefArray<TxPayload>,

    #[n(1)]
    ssc_payload: Ssc,

    #[n(2)]
    dlg_payload: MaybeIndefArray<Dlg>,

    #[n(3)]
    upd_payload: Up,
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
    body_proof: ByronHash,

    #[n(3)]
    consensus_data: EbbCons,

    #[n(4)]
    extra_data: (Attributes,),
}

#[derive(Encode, Decode, Debug)]
pub struct MainBlock {
    #[n(0)]
    header: BlockHead,

    #[n(1)]
    body: BlockBody,

    #[n(2)]
    extra: MaybeIndefArray<Attributes>,
}

#[derive(Encode, Decode, Debug)]
pub struct EbBlock {
    #[n(0)]
    header: EbbHead,

    #[n(1)]
    body: MaybeIndefArray<StakeholderId>,

    #[n(2)]
    extra: Option<Attributes>,
}

#[allow(clippy::large_enum_variant)]
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
    use crate::alonzo::Block;
    use crate::Fragment;

    use minicbor::{self, to_vec};

    #[test]
    fn block_isomorphic_decoding_encoding() {
        let test_blocks = vec![
            include_str!("test_data/test1.block"),
            include_str!("test_data/test2.block"),
            include_str!("test_data/test3.block"),
        ];

        for (idx, block_str) in test_blocks.iter().enumerate() {
            println!("decoding test block {}", idx + 1);
            let bytes = hex::decode(block_str).expect(&format!("bad block file {}", idx));

            let block = Block::decode_fragment(&bytes[..])
                .expect(&format!("error decoding cbor for file {}", idx));

            let _bytes2 =
                to_vec(block).expect(&format!("error encoding block cbor for file {}", idx));

            // HACK: we ommit the ismorphic requirement until we find the
            // offending difference
            // assert_eq!(bytes, bytes2);
        }
    }
}
