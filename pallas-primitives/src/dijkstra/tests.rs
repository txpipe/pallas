use super::{
    AccountBalanceInterval, AccountBalanceIntervals, AuxiliaryData, Block, BlockTransaction,
    Certificate, CostModels, DRep, GovAction, Guards, Header, MempoolTransaction, NativeScript,
    NonEmptySet, ProposalProcedure, Set, SetArm, StakeCredential, TransactionBody,
    TransactionOutput, Value, WitnessSet,
};
use pallas_codec::minicbor;
use pallas_codec::utils::{KeepRaw, Nullable};

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
];

/// Blocks from before the fork. Their bodies are the Conway five element
/// shape and their header bodies are already the twelve field Dijkstra one,
/// so the Dijkstra block type must refuse them on the body.
const PRE_FORK_BLOCKS: &[(&str, &str)] =
    &[("conway5", include_str!("../../../test_data/conway5.block"))];

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
        (45, 45, 5),
        "the fixtures should carry forty five transactions, each with a body and a witness set, five of them with auxiliary data"
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

        let leios_tail = minicbor::to_vec(as_dijkstra.header_body.block_body_contains_leios_cert)
            .unwrap()
            .len()
            + minicbor::to_vec(&as_dijkstra.header_body.eb_announcement)
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
    assert!(!empty.header_body.block_body_contains_leios_cert);
    assert!(matches!(
        empty.header_body.eb_announcement,
        crate::Nullable::Null
    ));

    let populated: Header = header_of("dijkstra8");
    assert!(populated.header_body.block_body_contains_leios_cert);
    assert!(matches!(
        populated.header_body.eb_announcement,
        crate::Nullable::Some(_)
    ));

    let width = |h: &Header| {
        minicbor::to_vec(&h.header_body.eb_announcement)
            .unwrap()
            .len()
    };
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
    // Renamed to `eb_announcement`.
    "leios_announcement",
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
    "is_valid",
    "pledge",
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

/// True when `defs.cddl` defines `name` as a rule, not merely mentions it.
fn defines_rule(name: &str) -> bool {
    DEFS_CDDL.lines().any(|line| {
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

/// Every CDDL rule this module's doc comments cite. Both files are scanned,
/// because a doc comment in either one can cite a rule.
fn cited_rules() -> std::collections::BTreeSet<String> {
    let mut cited = std::collections::BTreeSet::new();

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
                cited.insert(name);
                cited.extend(snake_words(body));
            }
        }
    }

    cited
}

/// Every citation has to land in exactly one of three places: a rule the
/// vendored file defines, a rule the ledger removed, or a name that is not one.
#[test]
fn every_cited_rule_matches_the_vendored_cddl() {
    let cited = cited_rules();

    // Every assertion below is satisfied by a scan that returns nothing.
    for expected in [
        "block",
        "block_body",
        "block_transaction",
        "certificate",
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
            !defines_rule(name) && !ABSENT_RULES.contains(name) && !NOT_RULES.contains(name)
        })
        .collect();
    assert!(
        unaccounted.is_empty(),
        "a doc comment cites {unaccounted:?}, which defs.cddl does not define and neither list accounts for"
    );

    let resurrected: Vec<&str> = ABSENT_RULES
        .iter()
        .copied()
        .filter(|name| defines_rule(name))
        .collect();
    assert!(
        resurrected.is_empty(),
        "defs.cddl defines {resurrected:?}, which this module models as removed or renamed"
    );

    let parked: Vec<&str> = NOT_RULES
        .iter()
        .copied()
        .filter(|name| defines_rule(name))
        .collect();
    assert!(
        parked.is_empty(),
        "defs.cddl defines {parked:?}, so they are rules and do not belong on the not-a-rule list"
    );

    let stale: Vec<&str> = NOT_RULES
        .iter()
        .copied()
        .filter(|name| !cited.contains(*name))
        .collect();
    assert!(
        stale.is_empty(),
        "no doc comment names {stale:?} any more, so the not-a-rule list has entries nothing needs"
    );

    // No list above can pass by the predicate always agreeing.
    assert!(defines_rule("block_body"));
    assert!(!defines_rule("block_bod"));
    assert!(!defines_rule("no_such_rule_exists"));
}

#[test]
fn conway_header_is_refused_as_dijkstra() {
    let conway = hex::decode(include_str!("../../../test_data/conway1.block")).unwrap();
    let (_, block): (u16, crate::conway::Block) = minicbor::decode(&conway).unwrap();
    let raw = block.header.raw_cbor();

    let decoded: Result<KeepRaw<'_, Header>, _> = minicbor::decode(raw);
    assert!(
        decoded.is_err(),
        "a ten field Conway header must not decode as a Dijkstra header"
    );
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
        header.header_body.block_body_contains_leios_cert,
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

    let crate::Nullable::Some(announcement) = &header.header_body.eb_announcement else {
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

    assert!(header.header_body.block_body_contains_leios_cert);
    assert!(
        matches!(header.header_body.eb_announcement, crate::Nullable::Null),
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
    assert!(matches!(
        header_of("dijkstra11").header_body.eb_announcement,
        crate::Nullable::Null
    ));
    assert!(
        !header_of("dijkstra11")
            .header_body
            .block_body_contains_leios_cert
    );
}

/// `eb_announcement = [eb_hash : hash32, eb_size : uint .size 4]` (`defs.cddl`).
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

        let crate::Nullable::Some(announcement) = &header.header_body.eb_announcement else {
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
        (38, 7),
        "the fixtures carry thirty eight tagged input sets and seven bare ones"
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
