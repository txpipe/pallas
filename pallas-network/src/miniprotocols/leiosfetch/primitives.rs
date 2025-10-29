// Material brought from `pallas-primitives`
// TODO: Refactor in order to avoid repetition.

use pallas_codec::{
    minicbor::{self, Decode, Encode},
    utils::Bytes,
};

use crate::miniprotocols::{leiosnotify::Hash, localtxsubmission::primitives::PoolKeyhash};

pub type BlsSignature = Bytes; // 48 bytes

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[cbor(flat)]
pub enum LeiosVote {
    #[n(0)]
    Persistent {
        #[n(0)]
        election_id: Bytes,

        #[n(1)]
        persistent_voter_id: Bytes,

        #[n(2)]
        endorser_block_hash: Hash,

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
        endorser_block_hash: Hash,

        #[n(5)]
        vote_signature: BlsSignature,
    },
}
