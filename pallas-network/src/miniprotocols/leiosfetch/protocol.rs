pub use pallas_codec::{
    minicbor::{self, Decode, Encode},
    utils::AnyCbor,
};

pub use crate::miniprotocols::leiosnotify::{
    BlockOffer, BlockTxsOffer, Hash, Header, Slot, VoteIssuerId,
};
use std::{collections::BTreeMap, fmt::Debug};

pub use super::primitives::LeiosVote;

pub type Tx = AnyCbor; // Mock Txs
pub type TxReference = Hash;
pub type EndorserBlock = Vec<TxReference>;
pub type BitMap = Vec<(u16, u64)>;
pub type TxMap = BTreeMap<u16, BitMap>;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum State {
    Idle,
    Block,
    BlockTxs,
    Votes,
    BlockRange,
    Done,
}

#[derive(Debug, Encode, Decode)]
#[cbor(flat)]
pub enum Message {
    #[n(0)]
    BlockRequest(#[n(0)] Slot, #[n(1)] Hash),
    #[n(1)]
    Block(#[n(0)] EndorserBlock),
    #[n(2)]
    BlockTxsRequest(#[n(0)] Slot, #[n(1)] Hash, #[n(2)] TxMap),
    #[n(3)]
    BlockTxs(#[n(0)] Vec<AnyCbor>),
    #[n(4)]
    VoteRequest(#[n(0)] Vec<(Slot, VoteIssuerId)>),
    #[n(5)]
    VoteDelivery(#[n(0)] Vec<LeiosVote>),
    #[n(6)]
    RangeRequest {
        #[n(0)]
        first: (Slot, Hash),
        #[n(1)]
        last: (Slot, Hash),
    },
    #[n(7)]
    NextBlockAndTxs(#[n(0)] EndorserBlock, #[n(1)] Vec<AnyCbor>),
    #[n(8)]
    LastBlockAndTxs(#[n(0)] EndorserBlock, #[n(1)] Vec<AnyCbor>),
    #[n(9)]
    Done,
}
