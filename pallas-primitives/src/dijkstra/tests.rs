use super::{
    AccountBalanceInterval, AccountBalanceIntervals, AuxiliaryData, Block, BlockTransaction,
    Certificate, CostModels, DRep, DatumOption, EndorserBlock, GovAction, Guards, Header,
    HeaderBody, LeiosHeaderBody, MempoolTransaction, NativeScript, NonEmptySet, PlutusData,
    ProposalProcedure, ProtocolParamUpdate, RationalNumber, Relay, ScriptRef, Set, SetArm,
    StakeCredential, SubTransactionBody, TransactionBody, TransactionOutput, Value, Voter,
    WitnessSet,
};
use crate::flat_enum::{assert_reads_either_length_form, decode_all};
use pallas_codec::minicbor;
use pallas_codec::utils::{CborWrap, KeepRaw, Nullable};
use pallas_crypto::hash::Hasher;

type BlockWrapper<'b> = (u16, Block<'b>);

/// Provenance for these fixtures is in `test_data/dijkstra-fixtures.md`.
const TEST_BLOCKS: &[(&str, &str)] = &[
    (
        "dijkstra1",
        include_str!("../../../test_data/dijkstra1.block"),
    ),
    (
        "dijkstra2",
        include_str!("../../../test_data/dijkstra2.block"),
    ),
    (
        "dijkstra3",
        include_str!("../../../test_data/dijkstra3.block"),
    ),
    (
        "dijkstra4",
        include_str!("../../../test_data/dijkstra4.block"),
    ),
    (
        "dijkstra5",
        include_str!("../../../test_data/dijkstra5.block"),
    ),
    (
        "dijkstra6",
        include_str!("../../../test_data/dijkstra6.block"),
    ),
    (
        "dijkstra7",
        include_str!("../../../test_data/dijkstra7.block"),
    ),
    (
        "dijkstra8",
        include_str!("../../../test_data/dijkstra8.block"),
    ),
    (
        "dijkstra9",
        include_str!("../../../test_data/dijkstra9.block"),
    ),
    (
        "dijkstra10",
        include_str!("../../../test_data/dijkstra10.block"),
    ),
    (
        "dijkstra11",
        include_str!("../../../test_data/dijkstra11.block"),
    ),
    (
        "dijkstra12",
        include_str!("../../../test_data/dijkstra12.block"),
    ),
    (
        "dijkstra13",
        include_str!("../../../test_data/dijkstra13.block"),
    ),
    (
        "dijkstra14",
        include_str!("../../../test_data/dijkstra14.block"),
    ),
    (
        "dijkstra15",
        include_str!("../../../test_data/dijkstra15.block"),
    ),
    (
        "dijkstra-rc-2026w42-7",
        include_str!("../../../test_data/dijkstra-rc-2026w42-7.block"),
    ),
    (
        "dijkstra-rc-2026w42-11",
        include_str!("../../../test_data/dijkstra-rc-2026w42-11.block"),
    ),
];

/// Blocks from before the fork. Their bodies are the Conway five element
/// shape and their header bodies are already the twelve field Dijkstra one,
/// so the Dijkstra block type must refuse them on the body.
const PRE_FORK_BLOCKS: &[(&str, &str)] =
    &[("conway5", include_str!("../../../test_data/conway5.block"))];

/// Endorser block bodies and the number of transactions each names.
const ENDORSER_BLOCKS: &[(&str, &str, usize)] = &[
    (
        "dijkstra-eb1",
        include_str!("../../../test_data/dijkstra-eb1.ebbody"),
        1,
    ),
    (
        "dijkstra-eb2",
        include_str!("../../../test_data/dijkstra-eb2.ebbody"),
        30,
    ),
    (
        "dijkstra-17402",
        include_str!("../../../test_data/dijkstra-17402.ebbody"),
        244,
    ),
];

const WITH_TRANSACTIONS: usize = 1;

fn block_named(name: &str) -> &'static str {
    TEST_BLOCKS
        .iter()
        .find(|(n, _)| *n == name)
        .unwrap_or_else(|| panic!("no fixture named {name}"))
        .1
}

fn block_of(name: &str) -> Block<'static> {
    let bytes: &'static [u8] =
        Box::leak(hex::decode(block_named(name)).unwrap().into_boxed_slice());
    let (_, block): BlockWrapper<'static> = minicbor::decode(bytes)
        .unwrap_or_else(|e| panic!("{name} should decode as a Dijkstra block: {e:?}"));
    block
}

fn header_of(name: &str) -> Header {
    let block = block_of(name);
    minicbor::decode(block.header.raw_cbor()).unwrap()
}

fn leios(header: &Header) -> &LeiosHeaderBody {
    match &header.header_body {
        HeaderBody::Leios(body) => body,
        HeaderBody::Praos(_) => panic!("expected the twelve field Leios header body"),
    }
}

#[test]
fn block_isomorphic_decoding_encoding() {
    for (name, block_str) in TEST_BLOCKS.iter() {
        let bytes = hex::decode(block_str).unwrap_or_else(|_| panic!("bad block file {name}"));

        let block: BlockWrapper = minicbor::decode(&bytes)
            .unwrap_or_else(|e| panic!("error decoding cbor for file {name}: {e:?}"));

        let bytes2 = minicbor::to_vec(block)
            .unwrap_or_else(|e| panic!("error encoding block cbor for file {name}: {e:?}"));

        let first_diff = bytes
            .iter()
            .zip(bytes2.iter())
            .position(|(a, b)| a != b)
            .unwrap_or(bytes.len().min(bytes2.len()));

        assert!(
            bytes.eq(&bytes2),
            "{name}: re-encoded bytes didn't match original, in {} out {}, first difference at byte {first_diff}\n  orig {}\n  ours {}",
            bytes.len(),
            bytes2.len(),
            hex::encode(&bytes[first_diff.saturating_sub(8)..(first_diff + 24).min(bytes.len())]),
            hex::encode(&bytes2[first_diff.saturating_sub(8)..(first_diff + 24).min(bytes2.len())]),
        );
    }
}

#[test]
fn endorser_block_isomorphic_decoding_encoding() {
    for (name, body_str, transactions) in ENDORSER_BLOCKS.iter() {
        let bytes = hex::decode(body_str).unwrap_or_else(|_| panic!("bad body file {name}"));

        let block: EndorserBlock = minicbor::decode(&bytes)
            .unwrap_or_else(|e| panic!("error decoding cbor for file {name}: {e:?}"));
        assert_eq!(block.len(), *transactions, "{name}: transaction count");

        assert_eq!(
            hex::encode(minicbor::to_vec(&block).unwrap()),
            hex::encode(&bytes),
            "{name}: re-encoded bytes didn't match original"
        );
    }
}

/// `Block::header` is a `KeepRaw` and re-emits the bytes it was decoded from,
/// so the block round trip cannot catch a wrong header. This re-encodes the
/// header through its own type, which is what exercises the field count.
#[test]
fn header_isomorphic_decoding_encoding() {
    for (name, block_str) in TEST_BLOCKS.iter() {
        let bytes = hex::decode(block_str).unwrap();
        let (_, block): BlockWrapper = minicbor::decode(&bytes).unwrap();

        let raw = block.header.raw_cbor();
        let reencoded = minicbor::to_vec(block.header.clone().unwrap()).unwrap();

        assert_eq!(
            hex::encode(raw),
            hex::encode(&reencoded),
            "{name}: header re-encoded through its own type did not match the wire bytes"
        );
    }
}

/// The transaction level twin of the header test. A `KeepRaw` replays the
/// bytes it read, so a map key the model has no field for is skipped in and
/// replayed out, and only re-encoding through each type catches that.
#[test]
fn transaction_payloads_reencode_to_their_wire_bytes() {
    let mut bodies = 0usize;
    let mut witness_sets = 0usize;
    let mut auxiliary_data = 0usize;

    for (name, block_str) in TEST_BLOCKS.iter() {
        let bytes = hex::decode(block_str).unwrap();
        let (_, block): BlockWrapper = minicbor::decode(&bytes).unwrap();

        for (i, tx) in block.block_body.transactions.iter().enumerate() {
            assert_eq!(
                hex::encode(minicbor::to_vec(tx.transaction_body.clone().unwrap()).unwrap()),
                hex::encode(tx.transaction_body.raw_cbor()),
                "{name} transaction {i}: the body re-encoded through its own type did not match the wire bytes"
            );
            bodies += 1;

            assert_eq!(
                hex::encode(minicbor::to_vec(tx.transaction_witness_set.clone().unwrap()).unwrap()),
                hex::encode(tx.transaction_witness_set.raw_cbor()),
                "{name} transaction {i}: the witness set re-encoded through its own type did not match the wire bytes"
            );
            witness_sets += 1;

            if let crate::Nullable::Some(aux) = &tx.auxiliary_data {
                assert_eq!(
                    hex::encode(minicbor::to_vec(aux.clone().unwrap()).unwrap()),
                    hex::encode(aux.raw_cbor()),
                    "{name} transaction {i}: the auxiliary data re-encoded through its own type did not match the wire bytes"
                );
                auxiliary_data += 1;
            }
        }
    }

    assert_eq!(
        (bodies, witness_sets, auxiliary_data),
        (46, 46, 5),
        "the fixtures should carry forty six transactions, each with a body and a witness set, five of them with auxiliary data"
    );
}

const INDEFINITE_BYTES_TXS: &[(&str, &str, &str, usize, &str, &str)] = &[
    (
        "dijkstra-indef-bytes1",
        include_str!("../../../test_data/dijkstra-indef-bytes1.tx"),
        "943a88c97f86522b2cda39d1d6bd076ea51a696e5ad41c41e4c6d38ce8f7dd50",
        11,
        "e040142528145d15aeea4834e948b87030a94ed33cdf3f7fff0364cf84a2408a",
        "00b44447f25f59bf23280d29fdea97f4e341e0bcd1d407ce80a8f11ede1ec80930158084eb5c735fd9bfb1ec53b8b5001c4b39d32c116b4921",
    ),
    (
        "dijkstra-indef-bytes2",
        include_str!("../../../test_data/dijkstra-indef-bytes2.tx"),
        "b026c42a9cf28aff7ce8624487f4d2b8f45df93454ac0ea2842408c1673cce5c",
        7,
        "943a88c97f86522b2cda39d1d6bd076ea51a696e5ad41c41e4c6d38ce8f7dd50",
        "00b44447f25f59bf23280d29fdea97f4e341e0bcd1d407ce80a8f11ede1ec80930158084eb5c735fd9bfb1ec53b8b5001c4b39d32c116b4921",
    ),
];

#[test]
fn transactions_with_indefinite_length_byte_strings_decode_and_keep_their_ids() {
    for (name, tx_hex, id, indefinite, input_id, address) in INDEFINITE_BYTES_TXS {
        let bytes = hex::decode(tx_hex).unwrap();

        let tx: MempoolTransaction = minicbor::decode(&bytes)
            .unwrap_or_else(|e| panic!("{name} should decode as a mempool transaction: {e:?}"));

        let input = tx.transaction_body.inputs.first().unwrap();
        assert_eq!(
            hex::encode(input.transaction_id),
            *input_id,
            "{name}: the first input id should be its chunks joined in order"
        );

        let Some(TransactionOutput::Legacy(output)) = tx.transaction_body.outputs.first() else {
            panic!("{name}: the first output should be in the array form");
        };
        assert_eq!(
            hex::encode(&*output.address),
            *address,
            "{name}: the first output address should be its chunks joined in order"
        );

        let body = tx.transaction_body.raw_cbor();
        let begins = minicbor::decode::Tokenizer::new(body)
            .filter(|token| matches!(token, Ok(minicbor::data::Token::BeginBytes)))
            .count();
        assert_eq!(
            begins, *indefinite,
            "{name}: the body should hold {indefinite} indefinite length byte strings"
        );

        assert_eq!(
            hex::encode(pallas_crypto::hash::Hasher::<256>::hash(body)),
            *id,
            "{name}: the id is the hash of the body bytes as read"
        );

        assert_eq!(
            hex::encode(minicbor::to_vec(&tx).unwrap()),
            *tx_hex,
            "{name}: the transaction should re-encode to the bytes it was read from"
        );
    }
}

/// Dijkstranet blocks with their header hash and transaction ids, provenance in `test_data/dijkstra-fixtures.md`.
const DIJKSTRANET_BLOCKS: &[(&str, &str, &str, &[&str])] = &[
    (
        "dijkstranet-66086",
        include_str!("../../../test_data/dijkstranet-66086.block"),
        "3ca025672aea5bbb68cc38eedb4c55a640c0e723b3c9eaf927bcd911b50fefd7",
        &["d43fe27b4a7343dc74708d8249721feac1fca1715403abd37b6b47f4a4f70514"],
    ),
    (
        "dijkstranet-66639",
        include_str!("../../../test_data/dijkstranet-66639.block"),
        "c3e155c67cd0bb2830e24818d5f4caa1e0385706d7f2eeccea6dccede162a6b6",
        &["1a0f3695f4a2eb8ccc81cef4907fb2fb417656353afce622a9c4366eff8e109c"],
    ),
    (
        "dijkstranet-67128",
        include_str!("../../../test_data/dijkstranet-67128.block"),
        "998efaf41f50d590f398192e6743e087acd644c5f26dca294a09bc6ae7836766",
        &["44c8e0fb792791b5fc62c23ec1e0628d7ebe4bc9a405010297cb9110498683f1"],
    ),
    (
        "dijkstranet-68330",
        include_str!("../../../test_data/dijkstranet-68330.block"),
        "b4ec9af31fcd284f3af54c18020ba8227bd81c0cbf36badc248d539a57b76d32",
        &["395c71f98f21dffd5483e952e35bef29262f51c95e85eb03508fbc8e86cba346"],
    ),
    (
        "dijkstranet-83548",
        include_str!("../../../test_data/dijkstranet-83548.block"),
        "330f81d235c2a5ea7d2a681f8b0b015bac8e1b1779906725f2939769cd365dd2",
        &["31afb7ab7cda6aa98bebf8b253c8fa490d6ddf61b77517c2b2d40527ee4d75c0"],
    ),
    (
        "dijkstranet-100887",
        include_str!("../../../test_data/dijkstranet-100887.block"),
        "0a4cee04726609c227aa3eecca86262b002229d332d7ae1238ac0cad3b8ddd63",
        &["0af0fec1ce41f40f4547492ff2e2abbfcca0a80b2a4cd77a60184005af8aa5d4"],
    ),
    (
        "dijkstranet-100930",
        include_str!("../../../test_data/dijkstranet-100930.block"),
        "71048c26f77c44dd7edc5344364ee75fbab6cce3c4481646e9f5ae36d085d532",
        &["0266f3298ea1db321eb5972d1579ebff9fc7be489300bd8cf7e399d659d5133e"],
    ),
    (
        "dijkstranet-100933",
        include_str!("../../../test_data/dijkstranet-100933.block"),
        "fb19d3b8563c307b4d9e9721f481434b7e0590a240cc0d2696953a968adc33ba",
        &["306735fd4f18141d55d609499980c2c3dc14cb3479c7c1468bab6d7961dd7bef"],
    ),
    (
        "dijkstranet-101244",
        include_str!("../../../test_data/dijkstranet-101244.block"),
        "6cdc9f838a7b666a49ee2d7a775afe93a081ae0578577714f7fc6fee28bbf474",
        &["f86e2ad6b52d61a4e88cf0547a1d1af59a19c278f56af50593090becb86fb1ed"],
    ),
    (
        "dijkstranet-101801",
        include_str!("../../../test_data/dijkstranet-101801.block"),
        "18d6c6cacb5d6d482926b13045e6b9ebf7a1fc7399a69c9583e733c7e3105146",
        &["1777608f1e9ce0d46c5edef8ab86d44aa1cefdaf1657f6a699808c69e216569d"],
    ),
    (
        "dijkstranet-101807",
        include_str!("../../../test_data/dijkstranet-101807.block"),
        "e34b2cffd4d15d8582fc2924fcc8369918da19d4c730b169ae79a0c45e91831a",
        &["53d78059eb3ef662f533d373b16b7f54d9c5cb67d31efed3636564cf05c3faa7"],
    ),
    (
        "dijkstranet-101810",
        include_str!("../../../test_data/dijkstranet-101810.block"),
        "be5e06fe75dae3392e6a9c1d25cda513b1b91ea4241e4486f6171f7df7ad7074",
        &["ddb3a4f0227b1e8fbdc17f269ef4b347aa367381660bdf17318fc99ecc4e188e"],
    ),
    (
        "dijkstranet-101816",
        include_str!("../../../test_data/dijkstranet-101816.block"),
        "d7c8f5fab5bd38c16f36e704901795c93bf4bf34e6fedf1a7b5870e2c98cf68f",
        &["d47a1a3065882d0a61d19e71d937eb9a45ece43ae59f14ec594d4fc23782695b"],
    ),
    (
        "dijkstranet-101821",
        include_str!("../../../test_data/dijkstranet-101821.block"),
        "4c77c2ffa373c77e4e9330e86b1e0d3d76de2137b8ed35d5952740c01d0a84e1",
        &["c651a23cb67c2434d1768c6e0e457b4e88f121637c17d082c01827adaf3ee0d8"],
    ),
    (
        "dijkstranet-104912",
        include_str!("../../../test_data/dijkstranet-104912.block"),
        "0a6c6ddcc753782a7a1ee49e21fb744a86f6b552a662043ad06241dd76681634",
        &["3ea15dba4b39188058f449e4b02c8d034fc04754a7c45690a0e5c603cda63998"],
    ),
    (
        "dijkstranet-105251",
        include_str!("../../../test_data/dijkstranet-105251.block"),
        "fcaedf7e2c6950a54a8d4a3857ec018414c37f8870c9d5ebc8c2522c7e0a369f",
        &["509983f270ef14f28779045a5a62e64507ba97e1eda371a82a475dce85919b5b"],
    ),
    (
        "dijkstranet-108317",
        include_str!("../../../test_data/dijkstranet-108317.block"),
        "a446ce788b8af3428cf4d2f23f3461a21c65bcb8780cc467fae56a385fd5eeb5",
        &["34f7777ec0618f637bf31c34d1a74288f83452b57fd86edeef8469aea11a9bda"],
    ),
    (
        "dijkstranet-126727",
        include_str!("../../../test_data/dijkstranet-126727.block"),
        "6f89073e92c8703ebc7267660172ffe5c9399f53bd0b81ac6174f1a7cbce9a6d",
        &[
            "5aded5626162eeba270690e57a4741c1be9a37e62d9c7af938f5c5a2d6b7f4ad",
            "c8ef3e0d778120a05b2ec806bd10a6127bb12e00db9cdd84b48e563a0cf4e6d9",
        ],
    ),
    (
        "dijkstranet-153913",
        include_str!("../../../test_data/dijkstranet-153913.block"),
        "3765872ae453542afb13ae96c731c6838edfbe8ae75fe313c762abaefc8ce211",
        &["94ac9a72722a64b9f96e14051c3ec528548aa166cc8277ffd33d1c9aa03ae7c8"],
    ),
];

#[test]
fn dijkstranet_blocks_decode_with_their_ids_and_reencode() {
    for (name, block_hex, header_hash, tx_ids) in DIJKSTRANET_BLOCKS {
        let bytes = hex::decode(block_hex).unwrap();

        let (era, block): BlockWrapper = minicbor::decode(&bytes)
            .unwrap_or_else(|e| panic!("{name} should decode as a Dijkstra block: {e}"));
        assert_eq!(era, 8, "{name}: block wrapper tag");

        assert_eq!(
            hex::encode(Hasher::<256>::hash(block.header.raw_cbor())),
            *header_hash,
            "{name}: header hash"
        );

        let HeaderBody::Praos(body) = &block.header.header_body else {
            panic!("{name}: a Dijkstranet header body has the ten field Praos form");
        };
        assert_eq!(
            Some(body.block_number.to_string().as_str()),
            name.strip_prefix("dijkstranet-"),
            "{name}: block number"
        );
        let minor = if *name == "dijkstranet-101801" { 69 } else { 0 };
        assert_eq!(
            body.protocol_version,
            (12, minor),
            "{name}: protocol version"
        );

        let ids: Vec<String> = block
            .block_body
            .transactions
            .iter()
            .map(|tx| hex::encode(Hasher::<256>::hash(tx.transaction_body.raw_cbor())))
            .collect();
        assert_eq!(ids, *tx_ids, "{name}: transaction ids");

        assert_eq!(
            hex::encode(minicbor::to_vec((era, &block)).unwrap()),
            *block_hex,
            "{name}: the block should re-encode to the bytes it was read from"
        );
    }
}

/// Transactions holding a PlutusV4 witness script, in their own witness set or a sub transaction's.
const V4_WITNESS_TXS: &[(&str, &str)] = &[
    (
        "dijkstra-v4-witness",
        include_str!("../../../test_data/dijkstra-v4-witness.tx"),
    ),
    (
        "dijkstra-v4-sub-witness",
        include_str!("../../../test_data/dijkstra-v4-sub-witness.tx"),
    ),
];

/// The keys of a CBOR map, in ascending order.
fn map_keys(cbor: &[u8]) -> Vec<u64> {
    let mut d = minicbor::Decoder::new(cbor);
    let n = d.map().unwrap().unwrap();
    let mut keys: Vec<u64> = (0..n)
        .map(|_| {
            let key = d.u64().unwrap();
            d.skip().unwrap();
            key
        })
        .collect();
    keys.sort();
    keys
}

/// The node writes key 8 ahead of key 5, so the test compares keys.
#[test]
fn a_witness_set_holding_a_plutus_v4_script_keeps_every_key_on_reencode() {
    let mut witness_sets = 0usize;
    let mut mismatched = Vec::new();

    for (name, tx_str) in V4_WITNESS_TXS {
        let bytes = hex::decode(tx_str).unwrap();
        let tx: BlockTransaction = minicbor::decode(&bytes).unwrap();
        let subs = tx
            .transaction_body
            .sub_transactions
            .iter()
            .flat_map(|x| x.iter())
            .map(|x| &x.transaction_witness_set);

        for (i, wits) in std::iter::once(&tx.transaction_witness_set)
            .chain(subs)
            .enumerate()
        {
            let wire = map_keys(wits.raw_cbor());
            let encoded = map_keys(&minicbor::to_vec(wits.clone().unwrap()).unwrap());
            if encoded != wire {
                mismatched.push((*name, i, wire, encoded));
            }
            witness_sets += 1;
        }
    }

    assert_eq!(witness_sets, 4, "two transactions and two sub transactions");
    assert_eq!(
        mismatched,
        vec![],
        "these witness sets lost or gained a key on re-encode"
    );
}

/// Run over an empty tail and a populated one, because an empty tail is two
/// bytes and a length check could match that by accident.
#[test]
fn dijkstra_header_is_not_a_babbage_header() {
    for name in ["dijkstra1", "dijkstra8"] {
        let bytes = hex::decode(block_named(name)).unwrap();
        let (_, block): BlockWrapper = minicbor::decode(&bytes).unwrap();
        let raw = block.header.raw_cbor();

        let as_babbage: crate::babbage::Header =
            minicbor::decode(raw).expect("babbage decoder accepts a dijkstra header");
        let babbage_bytes = minicbor::to_vec(&as_babbage).unwrap();
        assert!(
            babbage_bytes.len() < raw.len(),
            "{name}: expected the babbage header type to silently drop the leios fields"
        );

        let as_dijkstra: Header = minicbor::decode(raw).unwrap();

        let leios_tail = minicbor::to_vec(leios(&as_dijkstra).block_body_contains_leios_cert)
            .unwrap()
            .len()
            + minicbor::to_vec(&leios(&as_dijkstra).eb_announcement)
                .unwrap()
                .len();
        assert_eq!(
            raw.len() - babbage_bytes.len(),
            leios_tail,
            "{name}: the dropped tail should be exactly block_body_contains_leios_cert plus eb_announcement"
        );

        assert_eq!(minicbor::to_vec(&as_dijkstra).unwrap(), raw);
    }

    let empty: Header = header_of("dijkstra1");
    assert!(!leios(&empty).block_body_contains_leios_cert);
    assert!(matches!(
        leios(&empty).eb_announcement,
        crate::Nullable::Null
    ));

    let populated: Header = header_of("dijkstra8");
    assert!(leios(&populated).block_body_contains_leios_cert);
    assert!(matches!(
        leios(&populated).eb_announcement,
        crate::Nullable::Some(_)
    ));

    let width = |h: &Header| minicbor::to_vec(&leios(h).eb_announcement).unwrap().len();
    assert_eq!(width(&empty), 1, "a nil announcement is one byte");
    assert!(
        width(&populated) > 32,
        "an announcement carrying a 32 byte hash cannot fit in {} bytes",
        width(&populated)
    );
}

/// The refusal is the block body's, not the header's, so both halves are
/// asserted: a refusal that fired at the header would be a different fact.
#[test]
fn pre_fork_blocks_are_refused_as_dijkstra_blocks() {
    for (name, block_str) in PRE_FORK_BLOCKS.iter() {
        let bytes = hex::decode(block_str).unwrap();

        let decoded: Result<BlockWrapper, _> = minicbor::decode(&bytes);
        assert!(
            decoded.is_err(),
            "{name} is a pre fork block and must not decode as a Dijkstra one"
        );

        // Conway's type reads it, so the refusal above is about the era and not a damaged fixture.
        let (_, conway): (u16, crate::conway::Block) = minicbor::decode(&bytes)
            .unwrap_or_else(|e| panic!("{name} should decode as a Conway block: {e:?}"));
        let raw_header = conway.header.raw_cbor();

        let through_conway = minicbor::to_vec(conway.header.clone().unwrap()).unwrap();
        assert!(
            through_conway.len() < raw_header.len(),
            "{name}: Conway's ten field header body should drop the Leios tail, raw {} re-encoded {}",
            raw_header.len(),
            through_conway.len()
        );

        let as_dijkstra: Header = minicbor::decode(raw_header).unwrap_or_else(|e| {
            panic!("{name}: a pre fork header on this chain is already twelve fields: {e:?}")
        });
        assert_eq!(
            minicbor::to_vec(&as_dijkstra).unwrap(),
            raw_header,
            "{name}: the Dijkstra header type should give every byte back"
        );
    }
}

/// Neither form may decode as the other, or a closure would carry a verdict nobody issued.
#[test]
fn a_block_transaction_is_not_a_mempool_transaction() {
    let bytes = hex::decode(TEST_BLOCKS[WITH_TRANSACTIONS].1).unwrap();
    let (_, block): BlockWrapper = minicbor::decode(&bytes).unwrap();

    let tx = block
        .block_body
        .transactions
        .first()
        .expect("this fixture must carry at least one transaction");

    let four = minicbor::to_vec(tx).unwrap();
    let back: BlockTransaction = minicbor::decode(&four).unwrap();
    assert_eq!(&back, tx);

    let as_mempool: Result<MempoolTransaction, _> = minicbor::decode(&four);
    assert!(
        as_mempool.is_err(),
        "a block transaction must not decode as a mempool transaction"
    );

    let three = minicbor::to_vec(tx.to_mempool_transaction()).unwrap();
    let round: MempoolTransaction = minicbor::decode(&three).unwrap();
    assert!(!round.is_valid_supplied);
    assert_eq!(
        four.len() - three.len(),
        minicbor::to_vec(tx.success).unwrap().len(),
        "the two forms should differ by the flag and nothing else"
    );

    let as_block: Result<BlockTransaction, _> = minicbor::decode(&three);
    assert!(
        as_block.is_err(),
        "a mempool transaction must not decode as a block transaction"
    );
}

/// A valid block transaction comes back byte for byte from a round trip through
/// the mempool form.
#[test]
fn a_mempool_transaction_becomes_a_valid_block_transaction() {
    let bytes = hex::decode(TEST_BLOCKS[WITH_TRANSACTIONS].1).unwrap();
    let (_, block): BlockWrapper = minicbor::decode(&bytes).unwrap();

    let tx = block
        .block_body
        .transactions
        .first()
        .expect("this fixture must carry at least one transaction");

    let three = minicbor::to_vec(tx.to_mempool_transaction()).unwrap();
    let mempool: MempoolTransaction = minicbor::decode(&three).unwrap();

    let rebuilt = BlockTransaction::from(mempool.clone());
    assert!(rebuilt.success, "the mempool form admits no other verdict");

    let spelled_out = MempoolTransaction {
        is_valid_supplied: true,
        ..mempool
    };
    assert_eq!(
        BlockTransaction::from(spelled_out),
        rebuilt,
        "a `true` written out means what the three element form means"
    );
    assert_eq!(&rebuilt, tx, "nothing but the flag was restored");
    assert_eq!(
        minicbor::to_vec(&rebuilt).unwrap(),
        minicbor::to_vec(tx).unwrap(),
        "the round trip is byte for byte"
    );

    let four = minicbor::to_vec(&rebuilt).unwrap();
    let round: Result<MempoolTransaction, _> = minicbor::decode(&four);
    assert!(
        round.is_err(),
        "the rebuilt transaction must no longer read as a mempool one"
    );
}

/// Heap addresses of an input, a vkey witness and a metadata label, which a
/// move leaves unchanged.
fn heap_addresses(
    body: &TransactionBody,
    witnesses: &WitnessSet,
    auxiliary_data: &Nullable<KeepRaw<AuxiliaryData>>,
) -> [usize; 3] {
    let input = body.inputs.first().expect("a transaction spends an input");
    let vkey = witnesses
        .vkeywitness
        .as_ref()
        .and_then(|w| w.first())
        .expect("the fixture transaction is signed");
    let label = match auxiliary_data {
        Nullable::Some(aux) => match &**aux {
            AuxiliaryData::PostAlonzo(aux) => aux.metadata.as_ref().and_then(|m| m.keys().next()),
            _ => None,
        },
        _ => None,
    }
    .expect("the fixture transaction carries metadata");

    [
        input as *const _ as usize,
        vkey as *const _ as usize,
        label as *const _ as usize,
    ]
}

#[test]
fn a_mempool_transaction_is_moved_into_its_block_form() {
    let bytes = hex::decode(block_named("dijkstra4")).unwrap();
    let (_, block): BlockWrapper = minicbor::decode(&bytes).unwrap();

    let tx = block
        .block_body
        .transactions
        .first()
        .expect("dijkstra4 carries a transaction");

    let three = minicbor::to_vec(tx.to_mempool_transaction()).unwrap();
    let mempool: MempoolTransaction = minicbor::decode(&three).unwrap();

    let before = heap_addresses(
        &mempool.transaction_body,
        &mempool.transaction_witness_set,
        &mempool.auxiliary_data,
    );
    let rebuilt = BlockTransaction::from(mempool);
    let after = heap_addresses(
        &rebuilt.transaction_body,
        &rebuilt.transaction_witness_set,
        &rebuilt.auxiliary_data,
    );

    assert_eq!(
        after, before,
        "the body, witness set and auxiliary data must keep their allocations"
    );
}

const DEFS_CDDL: &str = include_str!("defs.cddl");
const MODEL_RS: &str = include_str!("model.rs");
const TESTS_RS: &str = include_str!("tests.rs");

/// The leios-fetch messages of the vendored cardano-blueprint, the second source this module cites.
#[cfg(feature = "blueprint")]
const LEIOS_FETCH_CDDL: &str =
    include_str!("../../../cardano-blueprint/src/network/node-to-node/leios-fetch/messages.cddl");

/// Rules this module cites that the leios-fetch messages define.
const LEIOS_FETCH_RULES: &[&str] = &["endorser_block"];

/// Rules the vendored revision does not define, because the ledger deleted or
/// renamed each one. A resync that brings one back fails the test below.
const ABSENT_RULES: &[&str] = &[
    // Deleted outright along with the block body's index set.
    "invalid_transactions",
    "transaction_index",
    // Renamed to `block_transaction`.
    "transaction",
    // Renamed to `mempool_transaction`.
    "transaction_mempool",
    // Renamed to `eb_announcement`, then to `eb_references_announcement`.
    "leios_announcement",
    "eb_announcement",
    // Renamed to `bls_key`.
    "leios_key",
    // Deleted with `redeemers`' array arm.
    "redeemer",
    // Widened into `guards` at the same key.
    "required_signers",
    // Dropped certificate variants 0 and 1.
    "account_registration_cert",
    "account_unregistration_cert",
    // Renamed to `guardrails_script_hash`.
    "policy_hash",
];

/// Backticked names in this module's doc comments that are not CDDL rules:
/// field names inside a quoted rule, prelude types, and CBOR literals.
const NOT_RULES: &[&str] = &[
    // Prelude types and control operators.
    "bool",
    "bytes",
    "cbor",
    "nil",
    "size",
    "uint",
    // The generic parameter of the three set rules, not a rule itself.
    "a0",
    // Field names inside a quoted rule body.
    "block_body_contains_leios_cert",
    "bls_possession_proof",
    "bls_pubkey",
    "body_signature",
    "deposit",
    "eb_hash",
    "eb_size",
    "highest_supported_major_version",
    "is_valid",
    "pledge",
    "self_reported_software_tag",
    "signers",
    "success",
    "ttl",
    // Values rather than rules.
    "false",
    "guarding",
    "true",
    // Parameters of the scan below, named in their own doc comments.
    "name",
    "text",
];

/// True when the CDDL `text` defines `name` as a rule, not merely mentions it.
fn defines_rule(text: &str, name: &str) -> bool {
    text.lines().any(|line| {
        line.strip_prefix(name).is_some_and(|rest| {
            let rest = rest.trim_start();
            // A plain rule is `name =`, a generic one is `name<a0> =`.
            rest.starts_with('=') || rest.starts_with('<')
        })
    })
}

/// Every snake case word in `text`. A word with an upper case letter is a
/// Rust name and never a CDDL rule.
fn snake_words(text: &str) -> Vec<String> {
    text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .filter(|word| {
            word.starts_with(|c: char| c.is_ascii_lowercase())
                && word
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        })
        .map(str::to_string)
        .collect()
}

/// The rule a backticked span cites: a bare rule name, or a quoted
/// definition `name = ...`. Anything else in backticks is prose.
fn rule_citation(span: &str) -> Option<(String, &str)> {
    let name: String = span
        .chars()
        .take_while(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '_')
        .collect();
    if !name.starts_with(|c: char| c.is_ascii_lowercase()) {
        return None;
    }

    let mut rest = span[name.len()..].trim_start();
    if let Some(after) = rest.strip_prefix('<') {
        rest = after.split_once('>')?.1.trim_start();
    }

    match rest.strip_prefix('=') {
        Some(body) => Some((name, body)),
        None if rest.is_empty() => Some((name, "")),
        None => None,
    }
}

/// Every rule citation in this module's doc comments, as the rule name and
/// the quoted text after its `=`. Both files are scanned, because a doc
/// comment in either one can cite a rule.
fn citations() -> Vec<(String, String)> {
    let mut found = Vec::new();

    for line in MODEL_RS.lines().chain(TESTS_RS.lines()) {
        let line = line.trim_start();
        if !(line.starts_with("///") || line.starts_with("//!")) {
            continue;
        }
        assert!(
            line.matches('`').count() % 2 == 0,
            "a doc comment opens a backtick it does not close, so the scan below cannot tell quoted text from prose: {line}"
        );

        for (i, span) in line.split('`').enumerate() {
            // Even indices are the prose between the quoted spans.
            if i % 2 == 0 {
                continue;
            }
            if let Some((name, body)) = rule_citation(span) {
                found.push((name, body.to_string()));
            }
        }
    }

    found
}

/// Every CDDL rule this module's doc comments cite.
fn cited_rules() -> std::collections::BTreeSet<String> {
    let mut cited = std::collections::BTreeSet::new();

    for (name, body) in citations() {
        // A leios-fetch rule's body names leios-fetch rules, so only the rule's name is added.
        if !LEIOS_FETCH_RULES.contains(&name.as_str()) {
            cited.extend(snake_words(&body));
        }
        cited.insert(name);
    }

    cited
}

/// Every citation has to land in exactly one of four places: a rule the
/// vendored file defines, a rule on the leios-fetch list, a rule the ledger
/// removed, or a name that is not one.
#[test]
fn every_cited_rule_matches_the_vendored_cddl() {
    let cited = cited_rules();

    // Every assertion below is satisfied by a scan that returns nothing.
    for expected in [
        "block",
        "block_body",
        "block_transaction",
        "certificate",
        "endorser_block",
        "guards",
        "header_body",
        "protocol_param_update",
        "transaction_body",
        "transaction_witness_set",
    ] {
        assert!(
            cited.contains(expected),
            "the citation scan did not find `{expected}`, which this module models and cites"
        );
    }
    assert!(
        cited.len() >= 60,
        "the citation scan found only {} names, far fewer than the module cites",
        cited.len()
    );

    let unaccounted: Vec<&str> = cited
        .iter()
        .map(String::as_str)
        .filter(|name| {
            !defines_rule(DEFS_CDDL, name)
                && !LEIOS_FETCH_RULES.contains(name)
                && !ABSENT_RULES.contains(name)
                && !NOT_RULES.contains(name)
        })
        .collect();
    assert!(
        unaccounted.is_empty(),
        "a doc comment cites {unaccounted:?}, which defs.cddl does not define and no list accounts for"
    );

    let doubled: Vec<&str> = LEIOS_FETCH_RULES
        .iter()
        .copied()
        .filter(|name| defines_rule(DEFS_CDDL, name))
        .collect();
    assert!(
        doubled.is_empty(),
        "defs.cddl defines {doubled:?}, so they belong to it rather than to the leios-fetch list"
    );

    let resurrected: Vec<&str> = ABSENT_RULES
        .iter()
        .copied()
        .filter(|name| defines_rule(DEFS_CDDL, name))
        .collect();
    assert!(
        resurrected.is_empty(),
        "defs.cddl defines {resurrected:?}, which this module models as removed or renamed"
    );

    let parked: Vec<&str> = NOT_RULES
        .iter()
        .copied()
        .filter(|name| defines_rule(DEFS_CDDL, name))
        .collect();
    assert!(
        parked.is_empty(),
        "defs.cddl defines {parked:?}, so they are rules and do not belong on the not-a-rule list"
    );

    let stale: Vec<&str> = NOT_RULES
        .iter()
        .chain(LEIOS_FETCH_RULES)
        .copied()
        .filter(|name| !cited.contains(*name))
        .collect();
    assert!(
        stale.is_empty(),
        "no doc comment names {stale:?} any more, so a list has entries nothing needs"
    );

    // No list above can pass by the predicate always agreeing.
    assert!(defines_rule(DEFS_CDDL, "block_body"));
    assert!(!defines_rule(DEFS_CDDL, "block_bod"));
    assert!(!defines_rule(DEFS_CDDL, "no_such_rule_exists"));
}

#[cfg(feature = "blueprint")]
#[test]
fn every_leios_fetch_citation_matches_the_leios_fetch_messages() {
    let unsourced: Vec<&str> = LEIOS_FETCH_RULES
        .iter()
        .copied()
        .filter(|name| !defines_rule(LEIOS_FETCH_CDDL, name))
        .collect();
    assert!(
        unsourced.is_empty(),
        "the leios-fetch messages do not define {unsourced:?}, so they do not belong on its list"
    );

    let words = |text: &str| text.split_whitespace().collect::<Vec<_>>().join(" ");
    let quoted: Vec<String> = citations()
        .into_iter()
        .filter(|(name, body)| LEIOS_FETCH_RULES.contains(&name.as_str()) && !body.is_empty())
        .map(|(name, body)| words(&format!("{name} ={body}")))
        .collect();
    for rule in LEIOS_FETCH_RULES {
        assert!(
            quoted
                .iter()
                .any(|quote| quote.starts_with(&format!("{rule} ="))),
            "no doc comment quotes `{rule}`, so nothing checks its text against the leios-fetch messages"
        );
    }
    for quote in &quoted {
        assert!(
            LEIOS_FETCH_CDDL.lines().any(|line| words(line) == *quote),
            "a doc comment quotes `{quote}`, which is no line of the leios-fetch messages"
        );
    }

    // The list cannot pass by the predicate always agreeing.
    assert!(defines_rule(LEIOS_FETCH_CDDL, "endorser_block"));
    assert!(!defines_rule(LEIOS_FETCH_CDDL, "block_body"));
}

#[test]
fn an_eleven_field_header_body_is_refused() {
    let header = header_of("dijkstra1");
    let body = leios(&header);

    let mut e = minicbor::Encoder::new(Vec::new());
    e.array(2).unwrap();
    e.array(11).unwrap();
    e.encode(body.block_number).unwrap();
    e.encode(body.slot).unwrap();
    e.encode(body.prev_hash).unwrap();
    e.encode(&body.issuer_vkey).unwrap();
    e.encode(&body.vrf_vkey).unwrap();
    e.encode(&body.vrf_result).unwrap();
    e.encode(body.block_body_size).unwrap();
    e.encode(body.block_body_hash).unwrap();
    e.encode(&body.operational_cert).unwrap();
    e.encode(&body.header_version_info).unwrap();
    e.encode(body.block_body_contains_leios_cert).unwrap();
    e.encode(&header.body_signature).unwrap();
    let raw = e.into_writer();

    let mut full = minicbor::Encoder::new(Vec::new());
    full.array(2).unwrap();
    full.array(12).unwrap();
    full.encode(body.block_number).unwrap();
    full.encode(body.slot).unwrap();
    full.encode(body.prev_hash).unwrap();
    full.encode(&body.issuer_vkey).unwrap();
    full.encode(&body.vrf_vkey).unwrap();
    full.encode(&body.vrf_result).unwrap();
    full.encode(body.block_body_size).unwrap();
    full.encode(body.block_body_hash).unwrap();
    full.encode(&body.operational_cert).unwrap();
    full.encode(&body.header_version_info).unwrap();
    full.encode(body.block_body_contains_leios_cert).unwrap();
    full.encode(&body.eb_announcement).unwrap();
    full.encode(&header.body_signature).unwrap();
    assert_eq!(
        full.into_writer(),
        minicbor::to_vec(&header).unwrap(),
        "the hand built twelve field header should equal the fixture's header"
    );

    let decoded: Result<Header, _> = minicbor::decode(&raw);
    let err = decoded.expect_err("an eleven field header body must not decode");
    assert!(
        err.to_string().contains("missing value at index 11"),
        "refused for another reason: {err}"
    );
}

/// Rewrites a header's body array with an indefinite length head and a break.
fn with_indefinite_header_body(raw: &[u8]) -> Vec<u8> {
    let mut d = minicbor::Decoder::new(raw);
    d.array().unwrap();
    let start = d.position();
    let fields = d.array().unwrap().unwrap();
    let contents = d.position();
    for _ in 0..fields {
        d.skip().unwrap();
    }
    let end = d.position();

    let mut out = raw[..start].to_vec();
    out.push(0x9f);
    out.extend_from_slice(&raw[contents..end]);
    out.push(0xff);
    out.extend_from_slice(&raw[end..]);
    out
}

#[test]
fn a_header_body_in_an_indefinite_array_reads_by_its_element_count() {
    let conway = hex::decode(include_str!("../../../test_data/conway1.block")).unwrap();
    let (_, block): (u16, crate::conway::Block) = minicbor::decode(&conway).unwrap();
    let praos = with_indefinite_header_body(block.header.raw_cbor());
    let header: Header = minicbor::decode(&praos).unwrap();
    let HeaderBody::Praos(body) = &header.header_body else {
        panic!("ten fields to a break should read as the Praos form");
    };
    assert_eq!(body, &block.header.header_body);

    let dijkstra = block_of("dijkstra1");
    let leios_raw = with_indefinite_header_body(dijkstra.header.raw_cbor());
    let header: Header = minicbor::decode(&leios_raw).unwrap();
    let HeaderBody::Leios(body) = &header.header_body else {
        panic!("twelve fields to a break should read as the Leios form");
    };
    assert_eq!(body, leios(&header_of("dijkstra1")));
}

#[test]
fn a_ten_field_header_body_reads_as_the_praos_form() {
    let conway = hex::decode(include_str!("../../../test_data/conway1.block")).unwrap();
    let (_, block): (u16, crate::conway::Block) = minicbor::decode(&conway).unwrap();
    let raw = block.header.raw_cbor();

    let header: Header = minicbor::decode(raw).unwrap();
    let HeaderBody::Praos(body) = &header.header_body else {
        panic!("a ten field header body should read as the Praos form");
    };
    assert_eq!(body, &block.header.header_body);
    assert_eq!(minicbor::to_vec(&header).unwrap(), raw);
}

/// A `proposal_procedure` whose `gov_action` is a `parameter_change_action`
/// setting one key. Hand built, because no fixture carries a proposal.
fn proposal_setting(key: u64) -> Vec<u8> {
    let hex_str = match key {
        0 => include_str!("../../../test_data/proposal-param-change-key0.hex"),
        48 => include_str!("../../../test_data/proposal-param-change-key48.hex"),
        other => panic!("no proposal fixture sets key {other}"),
    };

    hex::decode(hex_str).expect("invalid hex")
}

/// A short type would turn a key 48 change into an update that changes nothing.
#[test]
fn a_proposal_keeps_the_dijkstra_parameter_keys() {
    // Key 0 exists in every era, so a failure here is about the proposal shape.
    let shared_key = proposal_setting(0);
    let decoded: ProposalProcedure =
        minicbor::decode(&shared_key).expect("a proposal setting key 0 must decode");
    assert_eq!(
        hex::encode(minicbor::to_vec(&decoded).unwrap()),
        hex::encode(&shared_key),
        "a proposal setting key 0 lost bytes on re-encode"
    );

    let dijkstra_key = proposal_setting(48);
    let decoded: ProposalProcedure =
        minicbor::decode(&dijkstra_key).expect("a proposal setting key 48 must decode");
    assert_eq!(
        hex::encode(minicbor::to_vec(&decoded).unwrap()),
        hex::encode(&dijkstra_key),
        "a proposal setting key 48 lost it on re-encode, so the update type this era's gov_action carries stops short of 48"
    );

    match decoded.gov_action {
        GovAction::ParameterChange(_, update, _) => assert_eq!(
            update.max_ref_script_size_per_endorser_block,
            Some(20_000),
            "key 48 decoded into no field"
        ),
        other => panic!("expected a parameter change action, found {other:?}"),
    }
}

#[test]
fn a_parameter_update_re_encodes_every_key_and_each_nil() {
    let cases = [
        // Key 38 set to a value, the shape every earlier key has.
        ("38 set", "a11826d81e820301"),
        // Keys 49 to 56, with 50 and 54 as tag 30 rationals.
        (
            "49 to 56",
            "a8183118711832d81e8207031833187218341873183518741836d81e8201091837187518381876",
        ),
        // Keys 38 and 53 set to nil, which the CDDL admits for those two keys alone.
        ("38 and 53 nil", "a21826f61835f6"),
    ];

    let mut lost = Vec::new();
    for (label, hex_str) in cases {
        let bytes = hex::decode(hex_str).unwrap();
        let update: ProtocolParamUpdate = minicbor::decode(&bytes)
            .unwrap_or_else(|e| panic!("{label}: the update did not decode: {e:?}"));
        let encoded = hex::encode(minicbor::to_vec(&update).unwrap());
        if encoded != hex_str {
            lost.push((label, encoded));
        }
    }

    assert_eq!(lost, vec![], "these updates lost a key on re-encode");
}

/// `pool_params` carries `? bls_key : bls_key/ nil` (`defs.cddl`) between
/// `vrf_keyhash` and `pledge`. Every fixture certificate reaches the wire
/// inside a `KeepRaw` body, so without this the length choice runs nowhere.
#[test]
fn a_pool_registration_re_encodes_each_bls_key_state() {
    // A real registration off the chain, so the fields around the slot are the node's.
    let mut populated = None;
    for (_, block_str) in TEST_BLOCKS.iter() {
        let bytes = hex::decode(block_str).unwrap();
        let (_, block): BlockWrapper = minicbor::decode(&bytes).unwrap();
        for tx in block.block_body.transactions.iter() {
            let body = tx.transaction_body.clone().unwrap();
            let Some(certificates) = body.certificates else {
                continue;
            };
            for certificate in certificates.iter() {
                if matches!(certificate, Certificate::PoolRegistration { .. }) {
                    populated = Some(certificate.clone());
                }
            }
        }
    }
    let populated = populated.expect("the fixtures must carry a pool registration");

    let Certificate::PoolRegistration { ref bls_key, .. } = populated else {
        unreachable!()
    };
    assert!(
        matches!(bls_key, Some(crate::Nullable::Some(_))),
        "the chain carries this slot populated, which is the state the other two are built from"
    );

    let in_state = |slot| {
        let mut certificate = populated.clone();
        if let Certificate::PoolRegistration { bls_key, .. } = &mut certificate {
            *bls_key = slot;
        }
        certificate
    };

    let states = [
        ("populated", populated.clone(), 0x8b),
        ("nil", in_state(Some(crate::Nullable::Null)), 0x8b),
        ("absent", in_state(None), 0x8a),
    ];

    let mut encodings = Vec::new();
    for (label, certificate, array_header) in states.iter() {
        let bytes = minicbor::to_vec(certificate).unwrap();
        assert_eq!(
            bytes[0], *array_header,
            "{label}: the slot decides the array length"
        );

        let back: Certificate = minicbor::decode(&bytes)
            .unwrap_or_else(|e| panic!("{label}: a pool registration did not decode: {e:?}"));
        assert_eq!(&back, certificate, "{label}");

        encodings.push((label, bytes));
    }

    assert_ne!(
        encodings[0].1, encodings[1].1,
        "a populated slot and a nil slot must not encode alike"
    );
    assert_ne!(
        encodings[1].1, encodings[2].1,
        "a nil slot and an absent slot must not encode alike"
    );
}

/// No fixture carries either shape, so without this the encoder's four
/// element branch and the decoder's refusal of `false` run nowhere.
#[test]
fn a_mempool_transaction_re_encodes_both_accepted_shapes() {
    let bytes = hex::decode(TEST_BLOCKS[WITH_TRANSACTIONS].1).unwrap();
    let (_, block): BlockWrapper = minicbor::decode(&bytes).unwrap();
    let tx = block
        .block_body
        .transactions
        .first()
        .expect("this fixture must carry at least one transaction");

    let three = tx.to_mempool_transaction();
    let three_bytes = minicbor::to_vec(&three).unwrap();
    assert_eq!(
        three_bytes[0], 0x83,
        "a mempool transaction is three elements"
    );
    let back: MempoolTransaction = minicbor::decode(&three_bytes).unwrap();
    assert_eq!(back, three);

    let mut four = three.clone();
    four.is_valid_supplied = true;
    let four_bytes = minicbor::to_vec(&four).unwrap();
    assert_eq!(four_bytes[0], 0x84, "the tolerated form is four elements");
    let back: MempoolTransaction = minicbor::decode(&four_bytes).unwrap();
    assert_eq!(back, four);
    assert!(back.is_valid_supplied);

    // The body and the witness set replay their own bytes, so the flag's position
    // is computable. Checking the byte stops the refusal below firing elsewhere.
    let flag_at =
        1 + tx.transaction_body.raw_cbor().len() + tx.transaction_witness_set.raw_cbor().len();
    assert_eq!(four_bytes[flag_at], 0xf5, "the flag should be `true` here");

    let mut with_false = four_bytes.clone();
    with_false[flag_at] = 0xf4;
    let refused: Result<MempoolTransaction, _> = minicbor::decode(&with_false);
    assert!(
        refused.is_err(),
        "`false` is not an allowed value for the flag"
    );
}

/// An empty array has no first element to decide the arm on, so it is refused.
#[test]
fn an_empty_guards_array_is_refused() {
    let keyhash = [0x11u8; 28];

    let mut empty = vec![0x80];
    empty.push(0x58);
    empty.push(0x1c);
    empty.extend_from_slice(&keyhash);

    // The byte string after the empty array is not part of it.
    let decoded: Result<Guards, _> = minicbor::decode(&empty);
    assert!(
        decoded.is_err(),
        "an empty guards array must be refused, decoded as {:?}",
        decoded.ok()
    );

    // The same shape with one element decodes, so the refusal is about emptiness.
    let mut one_keyhash = vec![0x81, 0x58, 0x1c];
    one_keyhash.extend_from_slice(&keyhash);
    let decoded: Guards =
        minicbor::decode(&one_keyhash).expect("a one element addr_keyhash guards must decode");
    assert!(matches!(decoded, Guards::AddrKeyhashes(_)));

    let mut one_credential = vec![0x81, 0x82, 0x00, 0x58, 0x1c];
    one_credential.extend_from_slice(&keyhash);
    let decoded: Guards =
        minicbor::decode(&one_credential).expect("a one element credential guards must decode");
    assert!(matches!(decoded, Guards::Credentials(_)));
}

/// The bytes of a body with one input and no outputs, ending with the given hex keys.
fn body_ending(top: bool, rest: &str) -> &'static [u8] {
    let (header, fee) = if top { ("a4", "0200") } else { ("a3", "") };
    let input = format!("d9010281825820{}00", "03".repeat(32));
    generated(&format!("{header}00{input}0180{fee}{rest}"))
}

#[test]
fn an_empty_guards_set_in_either_body_reads_as_no_guards() {
    for empty in ["d9010280", "80", "9fff"] {
        let top: TransactionBody = decode_all(body_ending(true, &format!("0e{empty}")));
        assert_eq!(top.guards, None, "{empty}");
        let sub: SubTransactionBody = decode_all(body_ending(false, &format!("0e{empty}")));
        assert_eq!(sub.guards, None, "{empty}");
    }
}

#[test]
fn a_guards_set_with_a_key_in_either_body_reads_as_that_key() {
    let one = format!("0ed9010281581c{}", "06".repeat(28));
    let top: TransactionBody = decode_all(body_ending(true, &one));
    assert!(matches!(top.guards, Some(Guards::AddrKeyhashes(_))));
    let sub: SubTransactionBody = decode_all(body_ending(false, &one));
    assert!(matches!(sub.guards, Some(Guards::AddrKeyhashes(_))));
}

#[test]
fn a_guards_value_that_is_not_a_set_is_refused_in_either_body() {
    let top: Result<TransactionBody, _> = minicbor::decode(body_ending(true, "0e01"));
    assert!(top.is_err());
    let sub: Result<SubTransactionBody, _> = minicbor::decode(body_ending(false, "0e01"));
    assert!(sub.is_err());
}

/// A type reaching Alonzo's native script from here would refuse a whole
/// block over a guard clause in a transaction that used the array form.
#[test]
fn the_array_form_of_auxiliary_data_carries_this_eras_native_scripts() {
    let keyhash = [0x7a; 28];

    // `[{}, [[6, [0, h'..']]]]`, one guard clause in the array form.
    let mut aux = vec![0x82, 0xa0, 0x81, 0x82, 0x06, 0x82, 0x00, 0x58, 0x1c];
    aux.extend_from_slice(&keyhash);

    let decoded: AuxiliaryData =
        minicbor::decode(&aux).expect("the array form carrying a guard clause must decode");

    let AuxiliaryData::ShelleyMa(inner) = &decoded else {
        panic!("an array must decode as the ShelleyMa arm, found {decoded:?}");
    };
    let scripts = inner
        .auxiliary_scripts
        .as_ref()
        .expect("a guard clause must not make the whole auxiliary scripts field read as absent");
    assert_eq!(scripts.len(), 1);
    assert!(
        matches!(scripts[0], NativeScript::ScriptRequireGuard(_)),
        "the guard clause must survive, found {:?}",
        scripts[0]
    );

    assert_eq!(
        hex::encode(minicbor::to_vec(&decoded).unwrap()),
        hex::encode(&aux),
        "the array form must re-encode to the bytes it was read from"
    );
}

/// `script_n_of_k`'s threshold is `int64` here just as in alonzo (`defs.cddl`
/// carries the note in both); the extremes are where an unsigned regression
/// would fail first.
#[test]
fn n_of_k_threshold_is_signed_at_the_extremes() {
    for n in [i64::MIN, -1, 0, i64::MAX] {
        let script = NativeScript::ScriptNOfK(n, vec![]);
        let bytes = minicbor::to_vec(&script).unwrap();
        let decoded: NativeScript = minicbor::decode(&bytes).unwrap();
        assert_eq!(decoded, script, "round trip for n={n}");

        let alonzo_bytes =
            minicbor::to_vec(crate::alonzo::NativeScript::ScriptNOfK(n, vec![])).unwrap();
        assert_eq!(
            bytes, alonzo_bytes,
            "wire bytes must match alonzo's for n={n}"
        );
    }
}

/// The header hash each fixture was cut against, from `test_data/dijkstra-fixtures.md`.
const HEADER_HASHES: &[(&str, &str)] = &[
    (
        "dijkstra1",
        "d0c2a26a0192baf397b75cd38137987d82036c269089362842888279f3e19daf",
    ),
    (
        "dijkstra2",
        "adb23531ebb61891912e6a4bdabcbaaa053223d2de342eedbaa9b6af4fb526f3",
    ),
    (
        "dijkstra3",
        "294b3df1e6758e6f17f2b5a09ed469c6ec37c2db4d274264c8ee5edabe31229a",
    ),
    (
        "dijkstra4",
        "f8926da4333a3ce5fdb7b60a00d80eb23da0823c6962f06abfdee149b59dae41",
    ),
    (
        "dijkstra5",
        "7b7c9f48ac331106e9f9ef03856090bc6275f09fca5bb4c789068d04c54b079a",
    ),
    (
        "dijkstra6",
        "dec1d7087ff0191191fd3bac1559ae1b9c40b93ad33ec9cfba1f2e4722019a23",
    ),
    (
        "dijkstra7",
        "920a4883bf663cd3640af8ee87292edd391ff9b99debef2ba556f2f8e9d5761d",
    ),
    (
        "dijkstra8",
        "33a48eee693522320891dd4d1da8ee33c288c854eb25337802dbc4c912571d07",
    ),
    (
        "dijkstra9",
        "112495349409ccaa7a57e810aa59b014337612438a3e07cd5ce1bb3126dfbbea",
    ),
    (
        "dijkstra10",
        "917c72dcd2d2222df5cc82fcebea55c4f9135e5f1491afd66d79d48d52e42f60",
    ),
    (
        "dijkstra11",
        "e45c1dc810ddfb36ffb9647eaf08861b4611fb4e872a227c00337dddf2680b8c",
    ),
    (
        "dijkstra12",
        "3e0e56a9af0cb26e0641747ca835874135d02d35a29820a5e4de6beb37c17914",
    ),
    (
        "dijkstra13",
        "cf522686b27e452b3e261904058c7e323f3723e2f5c629e5a7542579b59474b4",
    ),
    (
        "dijkstra14",
        "9b481f4b4fa46de9a1bde085570b5fc9d90f161f99bde7f63bfe4ed20dbc37bf",
    ),
    (
        "dijkstra15",
        "0db84efa0259153a240cecacd0f9e52f942d40f96b132ebd0d5b3526e19b3a7b",
    ),
    (
        "dijkstra-rc-2026w42-7",
        "0d88c2182f7b6f1c2796677109d918c430538bf6b4df11c1ebf29a94243a3cd6",
    ),
    (
        "dijkstra-rc-2026w42-11",
        "f2fccd6e970425d1403101bc6553a4fb6a548a27d465fd29ae4c2ab11992b204",
    ),
];

/// The hash is over the header CBOR span, so it also pins where the body begins.
#[test]
fn every_fixture_hashes_to_its_recorded_header_hash() {
    assert_eq!(
        HEADER_HASHES.len(),
        TEST_BLOCKS.len(),
        "every fixture needs a recorded hash, and no row may name a fixture that is gone"
    );

    for (name, expected) in HEADER_HASHES.iter() {
        let block = block_of(name);
        let hashed = pallas_crypto::hash::Hasher::<256>::hash(block.header.raw_cbor());
        assert_eq!(
            hex::encode(hashed),
            *expected,
            "{name}: the header does not hash to the value it was cut against"
        );
    }

    let first = block_of("dijkstra1");
    let mut damaged = first.header.raw_cbor().to_vec();
    let last = damaged.len() - 1;
    damaged[last] ^= 0x01;
    assert_ne!(
        hex::encode(pallas_crypto::hash::Hasher::<256>::hash(&damaged)),
        HEADER_HASHES[0].1,
        "one flipped bit must change the header hash"
    );
}

/// Every value is read out of the block, because asking only whether the slot
/// is non nil would pass on a certificate decoded into the wrong two fields.
#[test]
fn a_block_carrying_both_leios_fields_reads_both() {
    let block = block_of("dijkstra8");
    let header = header_of("dijkstra8");

    // The header's flag and the body's slot are independent, and they agree.
    assert!(
        leios(&header).block_body_contains_leios_cert,
        "dijkstra8 is a block whose header says it carries a certificate"
    );

    let crate::Nullable::Some(certificate) = &block.block_body.leios_certificate else {
        panic!("dijkstra8 must carry a leios certificate in its block body");
    };

    assert_eq!(
        hex::encode(certificate.signers.as_slice()),
        "f218a17300000e08200200"
    );
    assert_eq!(certificate.signers.len(), 11);
    assert_eq!(
        hex::encode(certificate.signature.as_slice()),
        "b033236ab20c0f6aeec3a32c78500aa532c23c442f914e4688e831a7972524955e2ceaa29a898ec813f7993d08bd6d18"
    );
    assert_eq!(
        certificate.signature.len(),
        48,
        "a BLS12-381 signature is 48 bytes"
    );

    let crate::Nullable::Some(announcement) = &leios(&header).eb_announcement else {
        panic!("dijkstra8 must carry an endorser block announcement");
    };
    assert_eq!(
        hex::encode(announcement.eb_hash),
        "de5f4b812d0e6dc3129510c6663de4ea99bbd6bf019ec2d541853242926fd446"
    );
    assert_eq!(announcement.eb_hash.as_ref().len(), 32);
    assert_eq!(announcement.eb_size, 39495);

    // Peras is reserved and unused, so this slot is the empty answer.
    assert!(matches!(
        block.block_body.peras_certificate,
        crate::Nullable::Null
    ));
}

/// A model that read the certificate out of the announcement, or set the flag
/// from either, would pass on every other block in the set.
#[test]
fn a_certificate_travels_without_an_announcement() {
    let block = block_of("dijkstra9");
    let header = header_of("dijkstra9");

    assert!(leios(&header).block_body_contains_leios_cert);
    assert!(
        matches!(leios(&header).eb_announcement, crate::Nullable::Null),
        "dijkstra9 is the block that certifies without announcing"
    );

    let crate::Nullable::Some(certificate) = &block.block_body.leios_certificate else {
        panic!("dijkstra9 must carry a leios certificate");
    };
    assert_eq!(
        hex::encode(certificate.signers.as_slice()),
        "fb38a17604001208208000"
    );
    assert_eq!(
        hex::encode(certificate.signature.as_slice()),
        "aaffc43163076f2f58434d4b4d68c34afe09b45bc6da439cdef3d09ecba6b395a55335ba4a2f1feb6eba3096b0abe250"
    );
    assert_eq!(certificate.signature.len(), 48);

    // The two certificates differ, so neither test reads a decoder constant.
    let other = block_of("dijkstra8");
    let crate::Nullable::Some(first) = &other.block_body.leios_certificate else {
        unreachable!()
    };
    assert_ne!(first.signers, certificate.signers);
    assert_ne!(first.signature, certificate.signature);

    // And the other direction of the pair: a block with neither field.
    let plain = block_of("dijkstra11");
    assert!(matches!(
        plain.block_body.leios_certificate,
        crate::Nullable::Null
    ));
    let plain_header = header_of("dijkstra11");
    assert!(matches!(
        leios(&plain_header).eb_announcement,
        crate::Nullable::Null
    ));
    assert!(!leios(&plain_header).block_body_contains_leios_cert);
}

/// `eb_references_announcement = [eb_hash : hash32, eb_size : uint .size 4]` (`defs.cddl`).
/// dijkstra15 announces 71103 bytes, past what two bytes hold, so the size is
/// written as a five byte uint where dijkstra8's 39495 is a three byte one.
#[test]
fn an_announced_size_reaches_the_five_byte_uint() {
    assert!(
        71103 > u32::from(u16::MAX),
        "the wide case has to be past what a two byte uint holds"
    );

    for (name, size, head, payload) in [
        ("dijkstra8", 39495u32, 0x19u8, 2usize),
        ("dijkstra15", 71103, 0x1a, 4),
    ] {
        let block = block_of(name);
        let header = header_of(name);

        let crate::Nullable::Some(announcement) = &leios(&header).eb_announcement else {
            panic!("{name} must carry an endorser block announcement");
        };
        assert_eq!(announcement.eb_size, size, "{name}: the announced size");
        assert_eq!(
            announcement.eb_hash.as_ref().len(),
            32,
            "{name}: the announced hash"
        );

        let encoded = minicbor::to_vec(announcement).unwrap();
        assert_eq!(
            encoded[35], head,
            "{name}: the CBOR head byte the size re-encodes to"
        );
        assert_eq!(
            encoded.len(),
            36 + payload,
            "{name}: the size head byte is followed by {payload} payload bytes"
        );

        let raw = block.header.raw_cbor();
        assert!(
            raw.windows(encoded.len()).any(|w| w == encoded),
            "{name}: the announcement did not re-encode to a span of the header it was read from"
        );
    }
}

/// Transaction body key 3 is `ttl`, and dijkstra10 carries both output forms
/// at once, so one block reads the map form and the legacy array form.
#[test]
fn a_ttl_and_both_output_forms_are_read_from_one_block() {
    let block = block_of("dijkstra10");

    let mut ttls = Vec::new();
    let mut map_outputs = 0usize;
    let mut array_outputs = 0usize;

    for tx in block.block_body.transactions.iter() {
        let body = tx.transaction_body.clone().unwrap();
        if let Some(ttl) = body.ttl {
            ttls.push(ttl);
        }
        for output in body.outputs.iter() {
            match output {
                TransactionOutput::PostAlonzo(_) => map_outputs += 1,
                TransactionOutput::Legacy(_) => array_outputs += 1,
            }
        }
    }

    // A model with no field for key 3 would skip it in and replay it out, so
    // the round trip cannot see it and this is what does.
    assert_eq!(
        ttls,
        vec![468_996],
        "dijkstra10 sets ttl on exactly one of its seven transactions"
    );

    assert_eq!(
        (map_outputs, array_outputs),
        (5, 2),
        "dijkstra10 carries five map form outputs and two legacy array ones"
    );

    let first = block.block_body.transactions.first().unwrap();
    let body = first.transaction_body.clone().unwrap();
    let TransactionOutput::PostAlonzo(output) = body.outputs.first().unwrap() else {
        panic!("dijkstra10's first transaction pays to a map form output");
    };
    assert!(
        !output.address.is_empty(),
        "a map form output carries an address at key 0"
    );
    assert!(
        matches!(output.value, Value::Coin(_)),
        "this output pays plain ada"
    );
    assert!(
        output.datum_option.is_none(),
        "no datum option exists anywhere on this chain"
    );
    assert!(
        output.script_ref.is_none(),
        "no script reference exists anywhere on this chain"
    );
}

/// `delegation_to_drep_cert = (9, stake_credential, drep)` (`defs.cddl`).
/// dijkstra13 is the only fixture that reaches the tag 9 arm.
#[test]
fn a_vote_delegation_certificate_is_read_from_the_chain() {
    let block = block_of("dijkstra13");

    let mut certificates = Vec::new();
    for tx in block.block_body.transactions.iter() {
        let body = tx.transaction_body.clone().unwrap();
        if let Some(found) = body.certificates {
            certificates.extend(found.iter().cloned());
        }
    }

    assert_eq!(
        certificates.len(),
        2,
        "dijkstra13 carries a registration and a vote delegation"
    );

    let delegation = certificates
        .iter()
        .find(|c| matches!(c, Certificate::VoteDeleg(..)))
        .expect("dijkstra13 must carry a vote delegation certificate");
    let Certificate::VoteDeleg(credential, drep) = delegation else {
        unreachable!()
    };
    assert!(
        matches!(credential, StakeCredential::AddrKeyhash(_)),
        "this delegation is by key hash, found {credential:?}"
    );
    assert!(
        matches!(drep, DRep::Abstain),
        "this delegation is to the predefined abstain drep, found {drep:?}"
    );

    // A different arm, so the find above chose rather than took the only one.
    assert!(
        certificates
            .iter()
            .any(|c| matches!(c, Certificate::Reg(..))),
        "the block's other certificate is a registration"
    );

    let round: Certificate = minicbor::decode(&minicbor::to_vec(delegation).unwrap()).unwrap();
    assert_eq!(&round, delegation);
}

/// The fixture set holds both arms. If the bare arm ever left it, the round
/// trip would go on passing with the arm exercised by nothing.
#[test]
fn a_set_re_encodes_the_arm_it_was_read_from() {
    let mut tagged = 0usize;
    let mut bare = 0usize;

    for (name, block_str) in TEST_BLOCKS.iter() {
        let bytes = hex::decode(block_str).unwrap();
        let (_, block): BlockWrapper = minicbor::decode(&bytes).unwrap();

        for (i, tx) in block.block_body.transactions.iter().enumerate() {
            let body = tx.transaction_body.clone().unwrap();
            match body.inputs.arm() {
                SetArm::Tagged => tagged += 1,
                SetArm::Bare => bare += 1,
            }

            // Key 0 is the body map's first key, so the inputs span starts two
            // bytes in. Both of those bytes are checked rather than assumed.
            let raw = tx.transaction_body.raw_cbor();
            assert!(
                (0xa0..=0xb7).contains(&raw[0]),
                "{name} transaction {i}: a body is a map of at most 23 keys"
            );
            assert_eq!(
                raw[1], 0x00,
                "{name} transaction {i}: inputs are the body's first key"
            );
            assert!(
                raw[2..].starts_with(&minicbor::to_vec(&body.inputs).unwrap()),
                "{name} transaction {i}: the inputs did not re-encode to the span they were read from"
            );
        }
    }

    assert_eq!(
        (tagged, bare),
        (39, 7),
        "the fixtures carry thirty nine tagged input sets and seven bare ones"
    );

    // A type that normalised the arms would satisfy everything above and fail here.
    let built: Set<u8> = vec![1u8, 2].into();
    assert_eq!(
        built.arm(),
        SetArm::Tagged,
        "a set that was built rather than decoded writes the tagged arm"
    );
    assert_ne!(
        hex::encode(minicbor::to_vec(&built).unwrap()),
        hex::encode(minicbor::to_vec(built.clone().with_arm(SetArm::Bare)).unwrap()),
        "the two arms must not encode alike"
    );
}

/// `certificates : nonempty_set<certificate>` (`defs.cddl`). Every other
/// fixture writes that set under tag 258, and dijkstra14 writes it bare, so
/// both arms of the body's key 4 are read from real bytes rather than one.
#[test]
fn a_certificate_set_is_read_on_both_arms_from_the_chain() {
    let mut tagged = Vec::new();
    let mut bare = Vec::new();

    for (name, _) in TEST_BLOCKS.iter() {
        let block = block_of(name);

        for tx in block.block_body.transactions.iter() {
            let body = tx.transaction_body.clone().unwrap();
            let Some(certificates) = body.certificates else {
                continue;
            };

            let encoded = minicbor::to_vec(&certificates).unwrap();
            match certificates.arm() {
                SetArm::Tagged => {
                    assert_eq!(
                        encoded[0], 0xd9,
                        "{name}: a tagged certificate set re-encodes under a two byte tag"
                    );
                    tagged.push(*name);
                }
                SetArm::Bare => {
                    assert_eq!(
                        encoded[0] >> 5,
                        4,
                        "{name}: a bare certificate set re-encodes as a plain array"
                    );
                    bare.push(*name);
                }
            }
        }
    }

    assert_eq!(
        bare,
        vec!["dijkstra14"],
        "dijkstra14 is the fixture whose transaction body writes its certificates bare"
    );
    assert_eq!(
        tagged.len(),
        10,
        "every other certificate set in the fixtures is tagged, and found {tagged:?}"
    );

    let block = block_of("dijkstra14");
    let tx = block
        .block_body
        .transactions
        .iter()
        .next()
        .expect("dijkstra14 carries a transaction");
    let body = tx.transaction_body.clone().unwrap();
    let certificates = body
        .certificates
        .expect("dijkstra14 carries the certificates key");

    assert_eq!(certificates.len(), 1);
    assert!(
        matches!(certificates[0], Certificate::PoolRegistration { .. }),
        "dijkstra14 registers a pool, found {:?}",
        certificates[0]
    );
    assert_eq!(
        body.inputs.arm(),
        SetArm::Bare,
        "dijkstra14 writes its input set bare as well"
    );
}

/// So the test above is about the guard clause and not about the array form.
#[test]
fn the_array_form_still_carries_a_clause_every_era_has() {
    let keyhash = [0x5c; 28];

    let mut aux = vec![0x82, 0xa0, 0x81, 0x82, 0x00, 0x58, 0x1c];
    aux.extend_from_slice(&keyhash);

    let decoded: AuxiliaryData = minicbor::decode(&aux).expect("must decode");

    let AuxiliaryData::ShelleyMa(inner) = &decoded else {
        panic!("an array must decode as the ShelleyMa arm")
    };
    let scripts = inner.auxiliary_scripts.as_ref().expect("readable");
    assert_eq!(scripts.len(), 1);
    assert!(matches!(scripts[0], NativeScript::ScriptPubkey(_)));

    assert_eq!(
        hex::encode(minicbor::to_vec(&decoded).unwrap()),
        hex::encode(&aux)
    );
}

#[test]
fn a_set_of_one_element_reads_on_both_arms() {
    let tagged = [0xd9, 0x01, 0x02, 0x81, 0x01];
    let bare = [0x81, 0x01];

    let decoded: NonEmptySet<u64> =
        minicbor::decode(&tagged).expect("a tagged one element nonempty set must decode");
    assert_eq!(decoded.len(), 1);
    assert_eq!(decoded.arm(), SetArm::Tagged);

    let decoded: NonEmptySet<u64> =
        minicbor::decode(&bare).expect("a bare one element nonempty set must decode");
    assert_eq!(decoded.len(), 1);
    assert_eq!(decoded.arm(), SetArm::Bare);

    let empty: Set<u64> = minicbor::decode(&[0x80]).expect("an empty set must decode");
    assert!(empty.is_empty());
    let empty: Set<u64> =
        minicbor::decode(&[0xd9, 0x01, 0x02, 0x80]).expect("an empty tagged set must decode");
    assert!(empty.is_empty());
}

#[test]
fn an_empty_array_is_not_a_nonempty_set() {
    let cases: &[(&str, &[u8])] = &[
        ("tagged", &[0xd9, 0x01, 0x02, 0x80]),
        ("bare", &[0x80]),
        ("tagged indefinite", &[0xd9, 0x01, 0x02, 0x9f, 0xff]),
        ("bare indefinite", &[0x9f, 0xff]),
    ];

    for (arm, bytes) in cases {
        let err = minicbor::decode::<NonEmptySet<u64>>(bytes)
            .expect_err("an empty nonempty_set must not decode");
        assert!(
            err.to_string().contains("a nonempty set"),
            "the {arm} empty array was refused for some other reason: {err}"
        );
    }
}

#[test]
fn a_body_field_typed_nonempty_refuses_an_empty_array() {
    let top: &[u8] = &[0xa3, 0x00, 0x80, 0x01, 0x80, 0x02, 0x00];
    let sub: &[u8] = &[0xa2, 0x00, 0x80, 0x01, 0x80];
    minicbor::decode::<TransactionBody>(top).expect("a body without the field must decode");
    minicbor::decode::<SubTransactionBody>(sub).expect("a sub body without the field must decode");

    let with_empty = |body: &[u8], key: u8| {
        let mut bytes = vec![body[0] + 1];
        bytes.extend_from_slice(&body[1..]);
        bytes.extend_from_slice(&[key, 0x80]);
        bytes
    };
    for key in [4, 13, 18, 20, 23] {
        let err = minicbor::decode::<TransactionBody>(&with_empty(top, key))
            .expect_err("an empty nonempty field must not decode");
        assert!(
            err.to_string().contains("a nonempty set"),
            "body key {key} was refused for some other reason: {err}"
        );
    }
    for key in [4, 18, 20] {
        let err = minicbor::decode::<SubTransactionBody>(&with_empty(sub, key))
            .expect_err("an empty nonempty field must not decode");
        assert!(
            err.to_string().contains("a nonempty set"),
            "sub body key {key} was refused for some other reason: {err}"
        );
    }
}

#[test]
fn an_empty_script_set_in_a_witness_set_is_refused() {
    let err = minicbor::decode::<WitnessSet>(&[0xa1, 0x06, 0xd9, 0x01, 0x02, 0x80])
        .expect_err("an empty plutus_v2_script set must not decode");
    assert!(
        err.to_string().contains("a nonempty set"),
        "the empty script set was refused for some other reason: {err}"
    );
}

#[test]
fn an_empty_bootstrap_witness_set_is_read_as_written() {
    let bytes = [0xa1, 0x02, 0xd9, 0x01, 0x02, 0x80];
    let witnesses: WitnessSet = decode_all(&bytes);
    assert_eq!(
        witnesses.bootstrap_witness.as_deref().map(Vec::len),
        Some(0)
    );
    assert_eq!(minicbor::to_vec(&witnesses).unwrap(), bytes);
}

#[test]
fn cost_models_keeps_a_key_the_named_fields_do_not_cover() {
    // a2                      map of two keys
    //   00  82 01 02          key 0, the PlutusV1 model [1, 2]
    //   04  81 03             key 4, inside the CDDL's 4 .. 255 wildcard
    let with_wildcard_key = [0xa2, 0x00, 0x82, 0x01, 0x02, 0x04, 0x81, 0x03];

    let decoded: CostModels =
        minicbor::decode(&with_wildcard_key).expect("a cost models map with key 4 must decode");
    assert_eq!(decoded.plutus_v1, Some(vec![1, 2]));
    assert_eq!(decoded.unknown.get(&4), Some(&vec![3]));
    assert_eq!(
        hex::encode(minicbor::to_vec(&decoded).unwrap()),
        hex::encode(with_wildcard_key),
        "the cost model under key 4 was dropped on re-encode"
    );

    let named_only = [0xa1, 0x00, 0x82, 0x01, 0x02];
    let decoded: CostModels = minicbor::decode(&named_only).expect("a named key must decode");
    assert!(decoded.unknown.is_empty());
    assert_eq!(
        hex::encode(minicbor::to_vec(&decoded).unwrap()),
        hex::encode(named_only)
    );
}

#[test]
fn the_legal_account_balance_interval_forms_still_read() {
    let cases: &[(&str, &[u8], AccountBalanceInterval)] = &[
        (
            "bounded",
            &[0x82, 0x01, 0x02],
            AccountBalanceInterval::Bounded(1, 2),
        ),
        (
            "lower",
            &[0x82, 0x01, 0xf6],
            AccountBalanceInterval::LowerBound(1),
        ),
        (
            "upper",
            &[0x82, 0xf6, 0x02],
            AccountBalanceInterval::UpperBound(2),
        ),
        ("bare coin", &[0x0a], AccountBalanceInterval::Exact(10)),
        (
            "indefinite",
            &[0x9f, 0x01, 0x02, 0xff],
            AccountBalanceInterval::Bounded(1, 2),
        ),
    ];

    for (name, bytes, want) in cases {
        let decoded: AccountBalanceInterval =
            minicbor::decode(bytes).unwrap_or_else(|e| panic!("the {name} form must decode: {e}"));
        assert_eq!(&decoded, want, "the {name} form read as something else");
    }

    let definite_in_a_map = [
        0xa2, // map of two reward accounts
        0x41, 0x01, 0x82, 0x01, 0x02, // the first carries a bounded interval
        0x41, 0x02, 0x0a, // the second carries a bare coin
    ];
    let decoded: AccountBalanceIntervals =
        minicbor::decode(&definite_in_a_map).expect("a map of definite intervals must decode");
    assert_eq!(decoded.len(), 2);

    let indefinite_in_a_map = [
        0xa2, 0x41, 0x01, 0x9f, 0x01, 0x02, 0xff, // an indefinite interval
        0x41, 0x02, 0x0a,
    ];
    let decoded: AccountBalanceIntervals =
        minicbor::decode(&indefinite_in_a_map).expect("a map of indefinite intervals must decode");
    assert_eq!(decoded.len(), 2);
}

/// The bytes of a map form output with an address and a coin, ending with the given hex field.
fn output_ending_in(field: &str) -> Vec<u8> {
    hex::decode(format!("a300581d61{}0100{field}", "11".repeat(28))).unwrap()
}

#[test]
fn a_script_reference_in_tag_24_bytes_in_chunks_reads_as_the_joined_chunks() {
    let bytes = output_ending_in("03d8185f42820043820180ff");
    let TransactionOutput::PostAlonzo(output) = decode_all(&bytes) else {
        panic!("the output is a map");
    };
    let Some(CborWrap(ScriptRef::NativeScript(script))) = &output.script_ref else {
        panic!("the output holds a native script reference");
    };
    assert_eq!(**script, NativeScript::ScriptAll(vec![]));
    assert_eq!(script.raw_cbor(), [0x82, 0x01, 0x80]);
}

#[test]
fn an_inline_datum_in_tag_24_bytes_in_chunks_reads_as_the_joined_chunks() {
    let bytes = output_ending_in("028201d8185f41184118ff");
    let TransactionOutput::PostAlonzo(output) = decode_all(&bytes) else {
        panic!("the output is a map");
    };
    let Some(DatumOption::Data(CborWrap(data))) = output.datum_option.as_deref() else {
        panic!("the output holds an inline datum");
    };
    assert_eq!(data.raw_cbor(), [0x18, 0x18]);
}

#[test]
fn values_ending_in_an_absent_value_reencode_as_the_ledger_wrote_them() {
    use super::TransactionBody;

    for hex in [
        include_str!("../../../test_data/dijkstra-generated/body-certificates-ending-in-null.hex"),
        include_str!("../../../test_data/dijkstra-generated/body-governance-ending-in-null.hex"),
    ] {
        let bytes = hex::decode(hex).unwrap();
        let body: TransactionBody = minicbor::decode(&bytes).unwrap();
        assert_eq!(hex::encode(minicbor::to_vec(&body).unwrap()), hex);
    }
}

#[test]
fn an_account_balance_interval_is_two_elements_or_none() {
    let cases: &[(&str, &[u8])] = &[
        ("three element", &[0x83, 0x01, 0x02, 0x03]),
        ("one element", &[0x81, 0x01]),
        ("empty", &[0x80]),
        ("three element indefinite", &[0x9f, 0x01, 0x02, 0x03, 0xff]),
    ];

    for (name, bytes) in cases {
        let err = minicbor::decode::<AccountBalanceInterval>(bytes)
            .expect_err("an array that is not two elements must not decode");
        assert!(
            err.to_string().contains("account_balance_interval"),
            "the {name} array was refused for some other reason: {err}"
        );
    }
}

/// The bytes of one vector from `test_data/dijkstra-generated`.
fn generated(hex: &str) -> &'static [u8] {
    Box::leak(hex::decode(hex).unwrap().into_boxed_slice())
}

/// Whether a rational number equals `numerator / denominator` as a ratio.
fn same_ratio(found: &RationalNumber, numerator: u64, denominator: u64) -> bool {
    found.numerator as u128 * denominator as u128 == found.denominator as u128 * numerator as u128
}

#[test]
fn a_generated_parameter_update_keeps_the_keys_after_an_indefinite_rational() {
    let bytes = generated(include_str!(
        "../../../test_data/dijkstra-generated/ppu-rational-indefinite.hex"
    ));
    let update: ProtocolParamUpdate = decode_all(bytes);

    assert_eq!(update.minfee_a, Some(0x30417a8beab11095));
    assert_eq!(update.minfee_b, Some(0x93d15ea0c1acc926));
    assert_eq!(update.max_block_body_size, Some(0x327f8f25));
    assert_eq!(update.max_transaction_size, Some(0x7f16cbc8));
    assert_eq!(update.max_block_header_size, Some(0x4a9f));
    assert_eq!(update.key_deposit, Some(0xd78ece0e55ae6876));
    assert_eq!(update.pool_deposit, Some(0xf20ce478efceddf0));
    assert_eq!(update.maximum_epoch, Some(0x13adde3c));
    assert_eq!(update.desired_number_of_stake_pools, Some(0x890a));
    assert!(same_ratio(
        update.pool_pledge_influence.as_ref().unwrap(),
        0x6e24f5aeed36f7f5,
        0xd81f3b4fc11e15ae
    ));
    assert!(same_ratio(
        update.expansion_rate.as_ref().unwrap(),
        251,
        832
    ));
    assert!(same_ratio(
        update.treasury_growth_rate.as_ref().unwrap(),
        0x555555555555540a,
        0x555555555555543d
    ));
    assert_eq!(update.ada_per_utxo_byte, Some(0xe242b5b210626137));
}

#[test]
fn a_generated_relay_in_an_indefinite_array_reads_to_its_break() {
    let bytes = generated(include_str!(
        "../../../test_data/dijkstra-generated/relay-indefinite.hex"
    ));
    let relay: Relay = decode_all(bytes);

    assert_eq!(
        relay,
        Relay::MultiHostName("\u{241a}\u{3bdc1}\u{d0}".to_string())
    );
}

#[test]
fn a_generated_certificate_in_an_indefinite_array_reads_to_its_break() {
    let bytes = generated(include_str!(
        "../../../test_data/dijkstra-generated/body-certificate-indefinite.hex"
    ));
    let body: TransactionBody = decode_all(bytes);

    let certificates: Vec<_> = body.certificates.unwrap().to_vec();
    assert_eq!(
        certificates,
        vec![Certificate::UpdateDRepCert(
            StakeCredential::ScriptHash(
                "931c0e475a84cfe718b21f9ae8224c5077671633d9f8cacff61e009b"
                    .parse()
                    .unwrap()
            ),
            None
        )]
    );
}

#[test]
fn a_generated_value_in_an_indefinite_array_reads_to_its_break() {
    let bytes = generated(include_str!(
        "../../../test_data/dijkstra-generated/body-value-indefinite.hex"
    ));
    let body: TransactionBody = decode_all(bytes);

    let [TransactionOutput::Legacy(output)] = &body.outputs[..] else {
        panic!("the body holds one array form output");
    };
    let crate::alonzo::Value::Multiasset(coin, assets) = &output.amount else {
        panic!("the output value is a coin and assets");
    };
    assert_eq!(*coin, 18052182784416840275);
    let policy: crate::PolicyId = "a05a12f1f644247a57a10eec17c3f9752b30f44fbe49736dbc92c083"
        .parse()
        .unwrap();
    let names: Vec<_> = assets[&policy].iter().collect();
    assert_eq!(names.len(), 1);
    assert!(names[0].0.is_empty());
    assert_eq!(*names[0].1, 5424943601111550909);
}

#[test]
fn a_generated_transaction_in_an_indefinite_array_reads_to_its_break() {
    let bytes = generated(include_str!(
        "../../../test_data/dijkstra-generated/tx-indefinite.hex"
    ));
    let tx: MempoolTransaction = decode_all(bytes);

    assert!(tx.is_valid_supplied);
    assert_eq!(tx.transaction_body.fee, 10309690930332875869);
    let [TransactionOutput::Legacy(output)] = &tx.transaction_body.outputs[..] else {
        panic!("the body holds one array form output");
    };
    let coin = match &output.amount {
        crate::alonzo::Value::Coin(coin) | crate::alonzo::Value::Multiasset(coin, _) => *coin,
    };
    assert_eq!(coin, 13818883891935063992);
    assert_eq!(
        tx.transaction_witness_set
            .vkeywitness
            .as_ref()
            .map(|w| w.len()),
        Some(2)
    );
    assert_eq!(
        tx.transaction_witness_set
            .plutus_v1_script
            .as_ref()
            .map(|s| s.len()),
        Some(1)
    );
    assert!(matches!(tx.auxiliary_data, crate::Nullable::Null));
}

#[test]
fn a_generated_constructor_102_in_an_indefinite_array_hashes_as_the_ledger_does() {
    let bytes = generated(include_str!(
        "../../../test_data/dijkstra-generated/wits-constr-102-indefinite.hex"
    ));
    let witnesses: WitnessSet = decode_all(bytes);

    let datums = witnesses.plutus_data.unwrap();
    let hashes: Vec<String> = datums
        .iter()
        .map(|datum| Hasher::<256>::hash(datum.raw_cbor()).to_string())
        .collect();
    assert_eq!(
        hashes,
        [
            "f929da3fb67b303d9fd90a2fce58316023844d6b1f5d49fb399480f0f8e2f73e",
            "ee155ace9c40292074cb6aff8c9ccdd273c81648ff1149ef36bcea6ebb8a3e25",
        ]
    );
    let PlutusData::Constr(constr) = &*datums[0] else {
        panic!("the first datum is a constructor");
    };
    assert_eq!(constr.constr_index(), 1000);
    assert!(constr.fields.is_empty());
}

#[test]
fn a_generated_hard_fork_version_in_an_indefinite_array_decodes() {
    let bytes = generated(include_str!(
        "../../../test_data/dijkstra-generated/proposal-hard-fork-indefinite-version.hex"
    ));
    let body: TransactionBody = decode_all(bytes);

    let proposals: Vec<_> = body.proposal_procedures.unwrap().to_vec();
    assert_eq!(proposals.len(), 1);
    assert_eq!(proposals[0].deposit, 15225768051747963167);
    assert_eq!(
        proposals[0].gov_action,
        GovAction::HardForkInitiation(None, (3, 2317795968))
    );
}

/// Provenance for this fixture is in `test_data/dijkstra-generated.md`.
const VERSION_INFO_BLOCK: &str =
    include_str!("../../../test_data/dijkstra-generated/block-header-version-info.hex");

/// The version info of the generated block's header, as hex.
const VERSION_INFO: &str = "821a3be658c11aeb03cba5";

fn generated_header_hex() -> String {
    let block = hex::decode(VERSION_INFO_BLOCK.trim()).unwrap();
    let mut d = minicbor::Decoder::new(&block);
    d.array().unwrap();
    let start = d.position();
    d.skip().unwrap();
    hex::encode(&block[start..d.position()])
}

#[test]
fn a_version_info_in_an_indefinite_array_reads_to_its_break() {
    let header = generated_header_hex();
    let bytes = hex::decode(header.replace(VERSION_INFO, "9f1a3be658c11aeb03cba5ff")).unwrap();
    let mut d = minicbor::Decoder::new(&bytes);
    let header: Header = d.decode().expect("an indefinite version info must decode");
    assert_eq!(d.position(), bytes.len());
    let info = &leios(&header).header_version_info;
    assert_eq!(info.highest_supported_major_version, 1004951745);
    assert_eq!(info.self_reported_software_tag, 3942894501);
}

#[test]
fn a_version_info_field_past_four_bytes_is_refused() {
    let header = generated_header_hex();
    assert_eq!(header.matches(VERSION_INFO).count(), 1);
    let at = header.find(VERSION_INFO).unwrap() / 2;

    let widest = hex::decode(header.replace(VERSION_INFO, "821affffffff1affffffff")).unwrap();
    minicbor::decode::<Header>(&widest).expect("both fields at u32::MAX must decode");

    for (field, info, position) in [
        (
            "highest_supported_major_version",
            "821b00000001000000001aeb03cba5",
            at + 1,
        ),
        (
            "self_reported_software_tag",
            "821a3be658c11b0000000100000000",
            at + 6,
        ),
    ] {
        let mutated = hex::decode(header.replace(VERSION_INFO, info)).unwrap();
        let err =
            minicbor::decode::<Header>(&mutated).expect_err("a field of 2^32 must not decode");
        assert_eq!(
            err.position(),
            Some(position),
            "{field} was refused somewhere else: {err}"
        );
    }
}

const HASH: &str = "581c11111111111111111111111111111111111111111111111111111111";
const HASH32: &str = "58201111111111111111111111111111111111111111111111111111111111111111";
const REWARD: &str = "581de111111111111111111111111111111111111111111111111111111111";

#[test]
fn a_gov_action_reads_either_length_form() {
    let id = format!("82{HASH32}00");
    let credential = format!("8200{HASH}");
    assert_reads_either_length_form::<GovAction>(&[
        format!("8400{id}a10001{HASH}"),
        format!("8301{id}820a00"),
        format!("8302a1{REWARD}01{HASH}"),
        format!("8203{id}"),
        format!("8504{id}81{credential}a1{credential}01d81e820102"),
        format!("8305{id}828260{HASH32}{HASH}"),
        "8106".to_string(),
    ]);
}

#[test]
fn a_script_reference_reads_either_length_form() {
    assert_reads_either_length_form::<ScriptRef>(&[
        format!("82008200{HASH}"),
        "82014100".to_string(),
        "82024100".to_string(),
        "82034100".to_string(),
        "82044100".to_string(),
    ]);
}

#[test]
fn a_native_script_reads_either_length_form() {
    let script = format!("8200{HASH}");
    assert_reads_either_length_form::<NativeScript>(&[
        script.clone(),
        format!("820181{script}"),
        "820280".to_string(),
        "83030180".to_string(),
        "820401".to_string(),
        "820501".to_string(),
        format!("82068201{HASH}"),
    ]);
}

#[test]
fn a_generated_credential_in_an_indefinite_array_decodes() {
    let bytes = generated(include_str!(
        "../../../test_data/dijkstra-generated/body-credential-indefinite.hex"
    ));
    let body: TransactionBody = decode_all(bytes);

    assert_eq!(
        body.certificates.unwrap().to_vec(),
        vec![Certificate::StakeDelegation(
            StakeCredential::ScriptHash(
                "17ab434af5e4d2fcc4815897879f52d2068af2dc2630974ac0333a2b"
                    .parse()
                    .unwrap()
            ),
            "bc7069ba4ab9909c4b1517f3bd9ccbb45b075569c163820971dbe63e"
                .parse()
                .unwrap(),
        )]
    );
}

#[test]
fn generated_dreps_in_indefinite_arrays_decode() {
    let bytes = generated(include_str!(
        "../../../test_data/dijkstra-generated/body-drep-indefinite.hex"
    ));
    let body: TransactionBody = decode_all(bytes);

    assert_eq!(
        body.certificates.unwrap().to_vec(),
        vec![
            Certificate::VoteRegDeleg(
                StakeCredential::AddrKeyhash(
                    "098118293e4e9f9862dd7a9e876d00ca97e3b98ead92b6a2e7c30dc9"
                        .parse()
                        .unwrap()
                ),
                DRep::NoConfidence,
                0x97a8a827c45150b1
            ),
            Certificate::VoteRegDeleg(
                StakeCredential::AddrKeyhash(
                    "b8f80a08bf5b6bad733522cd91768136b5aad289bff7a51d7d620508"
                        .parse()
                        .unwrap()
                ),
                DRep::Abstain,
                0x41a4ecaab467a454
            ),
        ]
    );
}

#[test]
fn generated_datum_options_in_indefinite_arrays_decode() {
    let bytes = generated(include_str!(
        "../../../test_data/dijkstra-generated/body-datum-option-indefinite.hex"
    ));
    let body: TransactionBody = decode_all(bytes);

    let [
        TransactionOutput::PostAlonzo(hashed),
        TransactionOutput::PostAlonzo(inline),
    ] = &body.outputs[..]
    else {
        panic!("the body holds two map form outputs");
    };
    assert!(matches!(
        hashed.datum_option.as_deref(),
        Some(DatumOption::Hash(hash))
            if hash.to_string() == "f76479ff10b1f158e4b6c98049836b3bd0abe3fcceab1a5d646fdaa8f0d312fb"
    ));
    assert!(matches!(
        inline.datum_option.as_deref(),
        Some(DatumOption::Data(CborWrap(data))) if data.raw_cbor() == [0x80]
    ));
}

#[test]
fn a_generated_gov_action_in_an_indefinite_array_decodes() {
    let bytes = generated(include_str!(
        "../../../test_data/dijkstra-generated/body-gov-action-indefinite.hex"
    ));
    let body: TransactionBody = decode_all(bytes);

    let proposals: Vec<_> = body.proposal_procedures.unwrap().to_vec();
    assert_eq!(proposals.len(), 1);
    assert_eq!(proposals[0].deposit, 0x3092f0c34ad35adb);
    let account =
        hex::decode("f100020200000101010201000101010000010100000001010201000101").unwrap();
    assert_eq!(
        proposals[0].gov_action,
        GovAction::TreasuryWithdrawals([(account.into(), 0xa1d446b07e0bbe31)].into(), None)
    );
}

#[test]
fn a_generated_voter_in_an_indefinite_array_decodes() {
    let bytes = generated(include_str!(
        "../../../test_data/dijkstra-generated/body-voter-indefinite.hex"
    ));
    let body: TransactionBody = decode_all(bytes);

    let voters: Vec<_> = body.voting_procedures.unwrap().into_keys().collect();
    assert_eq!(
        voters,
        vec![Voter::ConstitutionalCommitteeKey(
            "7ff244d677b8920e733c5117a58478eb164d06298ca4bde583ac1e76"
                .parse()
                .unwrap()
        )]
    );
}

#[test]
fn a_generated_guard_credential_in_an_indefinite_array_decodes() {
    let bytes = generated(include_str!(
        "../../../test_data/dijkstra-generated/body-guard-credential-indefinite.hex"
    ));
    let body: TransactionBody = decode_all(bytes);

    let Some(Guards::Credentials(credentials)) = body.guards else {
        panic!("the body holds credential guards");
    };
    assert_eq!(
        credentials.to_vec(),
        vec![StakeCredential::AddrKeyhash(
            "b7fa435c83b33b9504b0aa55247460636800452cd4ed29cedd431dfe"
                .parse()
                .unwrap()
        )]
    );
}

#[test]
fn a_generated_script_reference_in_an_indefinite_array_decodes() {
    let bytes = generated(include_str!(
        "../../../test_data/dijkstra-generated/body-script-ref-indefinite.hex"
    ));
    let body: TransactionBody = decode_all(bytes);

    let [TransactionOutput::PostAlonzo(output)] = &body.outputs[..] else {
        panic!("the body holds one map form output");
    };
    let Some(CborWrap(ScriptRef::NativeScript(script))) = &output.script_ref else {
        panic!("the output holds a native script reference");
    };
    assert_eq!(**script, NativeScript::InvalidHereafter(0x73e96a1328c7873b));
}

#[test]
fn generated_native_scripts_in_indefinite_arrays_decode() {
    let bytes = generated(include_str!(
        "../../../test_data/dijkstra-generated/wits-native-scripts-indefinite.hex"
    ));
    let witnesses: WitnessSet = decode_all(bytes);

    let scripts: Vec<NativeScript> = witnesses
        .native_script
        .unwrap()
        .to_vec()
        .into_iter()
        .map(KeepRaw::unwrap)
        .collect();
    assert_eq!(
        scripts,
        vec![
            NativeScript::InvalidHereafter(0x73e96a1328c7873b),
            NativeScript::ScriptAny(vec![
                NativeScript::InvalidHereafter(0x7b16eeee03a344c6),
                NativeScript::InvalidBefore(0xdc760102a6a9e550),
            ]),
            NativeScript::ScriptRequireGuard(StakeCredential::ScriptHash(
                "35315820fb38df0f2034fa91fed7a4199d1d4b2d1b0d2a165345e349"
                    .parse()
                    .unwrap()
            )),
        ]
    );
}
