use pallas_codec::{
    minicbor::{self, Decode, Encode},
    // utils::AnyCbor,
};
use std::fmt::Debug;

pub type Slot = u64;

// TODO: Add `pallas_primitives::babbage::Header`
pub type Header = Vec<u8>;

pub type Hash = Vec<u8>;

pub type VoteIssuerId = Vec<u8>;

pub type BlockOffer = (Slot, Hash);

pub type BlockTxsOffer = (Slot, Hash);

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum State {
    Idle,
    Busy,
    Done,
}

#[derive(Debug, Encode, Decode)]
#[cbor(flat)]
pub enum Message {
    #[n(0)]
    RequestNext,
    #[n(1)]
    BlockAnnouncement(#[n(0)] Header),
    #[n(2)]
    BlockOffer(#[n(0)] Slot, #[n(1)] Hash),
    #[n(3)]
    BlockTxsOffer(#[n(0)] Slot, #[n(1)] Hash),
    #[n(4)]
    VoteOffer(#[n(0)] Vec<(Slot, VoteIssuerId)>),
    #[n(5)]
    Done,
}
