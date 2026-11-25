//! Decoding and encoding results of the inputs whose handling the `unstable` feature changes.

use pallas_codec::minicbor::{self, Encode};
use pallas_primitives::{
    RationalNumber, Relay, StakeCredential, alonzo, babbage,
    conway::{self, Anchor, Constitution, DRep, Vote, Voter, VotingProcedure},
};

const HASH: &str = "581c11111111111111111111111111111111111111111111111111111111";
const HASH32: &str = "58201111111111111111111111111111111111111111111111111111111111111111";

/// The builds a case's expected result holds in.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Builds {
    Every,
    WithoutUnstable,
}

use Builds::{Every, WithoutUnstable};

/// The value of type `$t` decoded from `$hex` and the position it ended at, or the error.
macro_rules! decoded {
    ($t:ty, $hex:expr) => {{
        let bytes = hex::decode($hex).unwrap();
        let mut d = minicbor::Decoder::new(&bytes);
        match d.decode::<$t>() {
            Ok(value) => format!("Ok({value:?}) at {}", d.position()),
            Err(error) => format!("Err({error})"),
        }
    }};
}

fn encoded<T: Encode<()>>(value: &T) -> String {
    hex::encode(minicbor::to_vec(value).unwrap())
}

/// A Babbage header body ending in the protocol version `version`.
fn header_body(version: &str) -> String {
    format!("8a0000f6404082404000{HASH32}8440000040{version}")
}

fn anchor() -> Anchor {
    Anchor {
        url: "a".to_string(),
        content_hash: [0x11; 32].into(),
    }
}

fn cases() -> Vec<(&'static str, Builds, String)> {
    let credential = format!("8200{HASH}");
    vec![
        (
            "credential definite",
            Every,
            decoded!(StakeCredential, &credential),
        ),
        (
            "credential indefinite",
            WithoutUnstable,
            decoded!(StakeCredential, format!("9f00{HASH}ff")),
        ),
        (
            "credential with an extra element",
            WithoutUnstable,
            decoded!(StakeCredential, format!("8300{HASH}00")),
        ),
        ("relay definite", Every, decoded!(Relay, "820263616263")),
        (
            "relay indefinite",
            WithoutUnstable,
            decoded!(Relay, "9f0263616263ff"),
        ),
        (
            "relay with an extra element",
            WithoutUnstable,
            decoded!(Relay, "83026361626300"),
        ),
        (
            "rational definite",
            Every,
            decoded!(RationalNumber, "d81e820102"),
        ),
        (
            "rational indefinite",
            WithoutUnstable,
            decoded!(RationalNumber, "d81e9f0102ff"),
        ),
        (
            "rational of three numbers",
            WithoutUnstable,
            decoded!(RationalNumber, "d81e83010203"),
        ),
        (
            "native script definite",
            Every,
            decoded!(alonzo::NativeScript, "820501"),
        ),
        (
            "native script indefinite",
            WithoutUnstable,
            decoded!(alonzo::NativeScript, "9f0501ff"),
        ),
        (
            "native script list indefinite",
            WithoutUnstable,
            decoded!(alonzo::NativeScript, "9f0180ff"),
        ),
        (
            "drep definite",
            Every,
            decoded!(DRep, format!("8201{HASH}")),
        ),
        ("drep abstain definite", Every, decoded!(DRep, "8102")),
        (
            "drep indefinite",
            WithoutUnstable,
            decoded!(DRep, format!("9f01{HASH}ff")),
        ),
        (
            "drep abstain indefinite",
            WithoutUnstable,
            decoded!(DRep, "9f02ff"),
        ),
        (
            "drep with an extra element",
            WithoutUnstable,
            decoded!(DRep, "820300"),
        ),
        (
            "voter definite",
            Every,
            decoded!(Voter, format!("8204{HASH}")),
        ),
        (
            "voter indefinite",
            WithoutUnstable,
            decoded!(Voter, format!("9f04{HASH}ff")),
        ),
        (
            "voter with an extra element",
            WithoutUnstable,
            decoded!(Voter, format!("8304{HASH}00")),
        ),
        (
            "alonzo value definite",
            Every,
            decoded!(alonzo::Value, format!("8201a1{HASH}a14001")),
        ),
        ("alonzo coin", Every, decoded!(alonzo::Value, "01")),
        (
            "alonzo value indefinite",
            WithoutUnstable,
            decoded!(alonzo::Value, format!("9f01a1{HASH}a14001ff")),
        ),
        (
            "alonzo value with an extra element",
            WithoutUnstable,
            decoded!(alonzo::Value, format!("8301a1{HASH}a1400100")),
        ),
        (
            "conway value definite",
            Every,
            decoded!(conway::Value, format!("8201a1{HASH}a14001")),
        ),
        ("conway coin", Every, decoded!(conway::Value, "01")),
        (
            "conway value indefinite",
            WithoutUnstable,
            decoded!(conway::Value, format!("9f01a1{HASH}a14001ff")),
        ),
        (
            "conway value with an extra element",
            WithoutUnstable,
            decoded!(conway::Value, format!("8301a1{HASH}a1400100")),
        ),
        (
            "datum option definite",
            Every,
            decoded!(babbage::DatumOption, format!("8200{HASH32}")),
        ),
        (
            "datum option indefinite",
            WithoutUnstable,
            decoded!(babbage::DatumOption, format!("9f00{HASH32}ff")),
        ),
        (
            "datum option with an extra element",
            WithoutUnstable,
            decoded!(babbage::DatumOption, format!("8300{HASH32}00")),
        ),
        (
            "header version definite",
            Every,
            decoded!(babbage::HeaderBody, header_body("820900")),
        ),
        (
            "header version indefinite",
            WithoutUnstable,
            decoded!(babbage::HeaderBody, header_body("9f0900ff")),
        ),
        (
            "header version of three numbers",
            WithoutUnstable,
            decoded!(babbage::HeaderBody, header_body("83090000")),
        ),
        (
            "alonzo update version definite",
            Every,
            decoded!(alonzo::ProtocolParamUpdate, "a10e820600"),
        ),
        (
            "alonzo update version null",
            Every,
            decoded!(alonzo::ProtocolParamUpdate, "a10ef6"),
        ),
        (
            "alonzo update version indefinite",
            WithoutUnstable,
            decoded!(alonzo::ProtocolParamUpdate, "a10e9f0600ff"),
        ),
        (
            "alonzo update version of three numbers",
            WithoutUnstable,
            decoded!(alonzo::ProtocolParamUpdate, "a10e83060000"),
        ),
        (
            "babbage update version definite",
            Every,
            decoded!(babbage::ProtocolParamUpdate, "a10e820700"),
        ),
        (
            "babbage update version indefinite",
            WithoutUnstable,
            decoded!(babbage::ProtocolParamUpdate, "a10e9f0700ff"),
        ),
        (
            "babbage update version of three numbers",
            WithoutUnstable,
            decoded!(babbage::ProtocolParamUpdate, "a10e83070000"),
        ),
        (
            "voting procedure with an anchor",
            Every,
            encoded(&VotingProcedure {
                vote: Vote::Yes,
                anchor: Some(anchor()),
            }),
        ),
        (
            "voting procedure without an anchor",
            WithoutUnstable,
            encoded(&VotingProcedure {
                vote: Vote::Yes,
                anchor: None,
            }),
        ),
        (
            "constitution with a guardrail script",
            Every,
            encoded(&Constitution {
                anchor: anchor(),
                guardrail_script: Some([0x11; 28].into()),
            }),
        ),
        (
            "constitution without a guardrail script",
            WithoutUnstable,
            encoded(&Constitution {
                anchor: anchor(),
                guardrail_script: None,
            }),
        ),
        (
            "alonzo certificate indefinite",
            Every,
            decoded!(alonzo::Certificate, format!("9f00{credential}ff")),
        ),
        (
            "conway certificate indefinite",
            Every,
            decoded!(conway::Certificate, format!("9f00{credential}ff")),
        ),
        (
            "babbage script reference indefinite",
            Every,
            decoded!(babbage::ScriptRef, "9f014100ff"),
        ),
        (
            "conway script reference indefinite",
            Every,
            decoded!(conway::ScriptRef, "9f034100ff"),
        ),
        (
            "conway gov action indefinite",
            Every,
            decoded!(conway::GovAction, "9f06ff"),
        ),
        (
            "conway hard fork version indefinite",
            Every,
            decoded!(conway::GovAction, "8301f69f0900ff"),
        ),
        (
            "conway hard fork version of three numbers",
            Every,
            decoded!(conway::GovAction, "8301f683090000"),
        ),
    ]
}

/// Each case's result in a build without `unstable`.
const EXPECTED: &[(&str, &str)] = &[
    (
        "credential definite",
        "Ok(AddrKeyhash(Hash<28>(\"11111111111111111111111111111111111111111111111111111111\"))) at 32",
    ),
    (
        "credential indefinite",
        "Err(decode error at position 0: flat enum requires definite-length array)",
    ),
    (
        "credential with an extra element",
        "Ok(AddrKeyhash(Hash<28>(\"11111111111111111111111111111111111111111111111111111111\"))) at 33",
    ),
    ("relay definite", "Ok(MultiHostName(\"abc\")) at 6"),
    ("relay indefinite", "Ok(MultiHostName(\"abc\")) at 6"),
    (
        "relay with an extra element",
        "Ok(MultiHostName(\"abc\")) at 6",
    ),
    (
        "rational definite",
        "Ok(RationalNumber { numerator: 1, denominator: 2 }) at 5",
    ),
    (
        "rational indefinite",
        "Ok(RationalNumber { numerator: 1, denominator: 2 }) at 5",
    ),
    (
        "rational of three numbers",
        "Ok(RationalNumber { numerator: 1, denominator: 2 }) at 5",
    ),
    ("native script definite", "Ok(InvalidHereafter(1)) at 3"),
    (
        "native script indefinite",
        "Err(decode error at position 0: flat enum requires definite-length array)",
    ),
    (
        "native script list indefinite",
        "Err(decode error at position 0: flat enum requires definite-length array)",
    ),
    (
        "drep definite",
        "Ok(Script(Hash<28>(\"11111111111111111111111111111111111111111111111111111111\"))) at 32",
    ),
    ("drep abstain definite", "Ok(Abstain) at 2"),
    (
        "drep indefinite",
        "Err(decode error at position 0: flat enum requires definite-length array)",
    ),
    (
        "drep abstain indefinite",
        "Err(decode error at position 0: flat enum requires definite-length array)",
    ),
    ("drep with an extra element", "Ok(NoConfidence) at 2"),
    (
        "voter definite",
        "Ok(StakePoolKey(Hash<28>(\"11111111111111111111111111111111111111111111111111111111\"))) at 32",
    ),
    (
        "voter indefinite",
        "Err(decode error at position 0: flat enum requires definite-length array)",
    ),
    (
        "voter with an extra element",
        "Ok(StakePoolKey(Hash<28>(\"11111111111111111111111111111111111111111111111111111111\"))) at 33",
    ),
    (
        "alonzo value definite",
        "Ok(Multiasset(1, {Hash<28>(\"11111111111111111111111111111111111111111111111111111111\"): {Bytes(ByteVec([])): 1}})) at 36",
    ),
    ("alonzo coin", "Ok(Coin(1)) at 1"),
    (
        "alonzo value indefinite",
        "Err(decode error: Unknown cbor data type for this macro-defined enum.)",
    ),
    (
        "alonzo value with an extra element",
        "Ok(Multiasset(1, {Hash<28>(\"11111111111111111111111111111111111111111111111111111111\"): {Bytes(ByteVec([])): 1}})) at 36",
    ),
    (
        "conway value definite",
        "Ok(Multiasset(1, {Hash<28>(\"11111111111111111111111111111111111111111111111111111111\"): {Bytes(ByteVec([])): PositiveCoin(1)}})) at 36",
    ),
    ("conway coin", "Ok(Coin(1)) at 1"),
    (
        "conway value indefinite",
        "Err(decode error: Unknown cbor data type for this macro-defined enum.)",
    ),
    (
        "conway value with an extra element",
        "Ok(Multiasset(1, {Hash<28>(\"11111111111111111111111111111111111111111111111111111111\"): {Bytes(ByteVec([])): PositiveCoin(1)}})) at 36",
    ),
    (
        "datum option definite",
        "Ok(Hash(Hash<32>(\"1111111111111111111111111111111111111111111111111111111111111111\"))) at 36",
    ),
    (
        "datum option indefinite",
        "Err(decode error at position 0: flat enum requires definite-length array)",
    ),
    (
        "datum option with an extra element",
        "Ok(Hash(Hash<32>(\"1111111111111111111111111111111111111111111111111111111111111111\"))) at 37",
    ),
    (
        "header version definite",
        "Ok(HeaderBody { block_number: 0, slot: 0, prev_hash: None, issuer_vkey: Bytes(ByteVec([])), vrf_vkey: Bytes(ByteVec([])), vrf_result: VrfCert(Bytes(ByteVec([])), Bytes(ByteVec([]))), block_body_size: 0, block_body_hash: Hash<32>(\"1111111111111111111111111111111111111111111111111111111111111111\"), operational_cert: OperationalCert { operational_cert_hot_vkey: Bytes(ByteVec([])), operational_cert_sequence_number: 0, operational_cert_kes_period: 0, operational_cert_sigma: Bytes(ByteVec([])) }, protocol_version: (9, 0) }) at 52",
    ),
    (
        "header version indefinite",
        "Err(decode error at position 49: invalid 2-tuple length)",
    ),
    (
        "header version of three numbers",
        "Err(decode error at position 49: invalid 2-tuple length)",
    ),
    (
        "alonzo update version definite",
        "Ok(ProtocolParamUpdate { minfee_a: None, minfee_b: None, max_block_body_size: None, max_transaction_size: None, max_block_header_size: None, key_deposit: None, pool_deposit: None, maximum_epoch: None, desired_number_of_stake_pools: None, pool_pledge_influence: None, expansion_rate: None, treasury_growth_rate: None, decentralization_constant: None, extra_entropy: None, protocol_version: Some((6, 0)), min_pool_cost: None, ada_per_utxo_byte: None, cost_models_for_script_languages: None, execution_costs: None, max_tx_ex_units: None, max_block_ex_units: None, max_value_size: None, collateral_percentage: None, max_collateral_inputs: None }) at 5",
    ),
    (
        "alonzo update version null",
        "Ok(ProtocolParamUpdate { minfee_a: None, minfee_b: None, max_block_body_size: None, max_transaction_size: None, max_block_header_size: None, key_deposit: None, pool_deposit: None, maximum_epoch: None, desired_number_of_stake_pools: None, pool_pledge_influence: None, expansion_rate: None, treasury_growth_rate: None, decentralization_constant: None, extra_entropy: None, protocol_version: None, min_pool_cost: None, ada_per_utxo_byte: None, cost_models_for_script_languages: None, execution_costs: None, max_tx_ex_units: None, max_block_ex_units: None, max_value_size: None, collateral_percentage: None, max_collateral_inputs: None }) at 3",
    ),
    (
        "alonzo update version indefinite",
        "Err(decode error at position 2: invalid 2-tuple length)",
    ),
    (
        "alonzo update version of three numbers",
        "Err(decode error at position 2: invalid 2-tuple length)",
    ),
    (
        "babbage update version definite",
        "Ok(ProtocolParamUpdate { minfee_a: None, minfee_b: None, max_block_body_size: None, max_transaction_size: None, max_block_header_size: None, key_deposit: None, pool_deposit: None, maximum_epoch: None, desired_number_of_stake_pools: None, pool_pledge_influence: None, expansion_rate: None, treasury_growth_rate: None, protocol_version: Some((7, 0)), min_pool_cost: None, ada_per_utxo_byte: None, cost_models_for_script_languages: None, execution_costs: None, max_tx_ex_units: None, max_block_ex_units: None, max_value_size: None, collateral_percentage: None, max_collateral_inputs: None }) at 5",
    ),
    (
        "babbage update version indefinite",
        "Err(decode error at position 2: invalid 2-tuple length)",
    ),
    (
        "babbage update version of three numbers",
        "Err(decode error at position 2: invalid 2-tuple length)",
    ),
    (
        "voting procedure with an anchor",
        "820182616158201111111111111111111111111111111111111111111111111111111111111111",
    ),
    ("voting procedure without an anchor", "8101"),
    (
        "constitution with a guardrail script",
        "8282616158201111111111111111111111111111111111111111111111111111111111111111581c11111111111111111111111111111111111111111111111111111111",
    ),
    (
        "constitution without a guardrail script",
        "8182616158201111111111111111111111111111111111111111111111111111111111111111",
    ),
    (
        "alonzo certificate indefinite",
        "Err(decode error at position 0: flat enum requires definite-length array)",
    ),
    (
        "conway certificate indefinite",
        "Err(decode error at position 0: flat enum requires definite-length array)",
    ),
    (
        "babbage script reference indefinite",
        "Err(decode error at position 0: flat enum requires definite-length array)",
    ),
    (
        "conway script reference indefinite",
        "Err(decode error at position 0: flat enum requires definite-length array)",
    ),
    (
        "conway gov action indefinite",
        "Err(decode error at position 0: flat enum requires definite-length array)",
    ),
    (
        "conway hard fork version indefinite",
        "Err(decode error at position 3: invalid 2-tuple length)",
    ),
    (
        "conway hard fork version of three numbers",
        "Err(decode error at position 3: invalid 2-tuple length)",
    ),
];

#[test]
fn each_case_has_its_expected_result() {
    let cases = cases();
    let names: Vec<_> = cases.iter().map(|(name, ..)| *name).collect();
    let expected_names: Vec<_> = EXPECTED.iter().map(|(name, _)| *name).collect();
    assert_eq!(
        names,
        expected_names,
        "every case needs one expected result, in order:\n{}",
        cases
            .iter()
            .map(|(name, _, result)| format!("    ({name:?}, {result:?}),"))
            .collect::<Vec<_>>()
            .join("\n")
    );

    let held: Vec<_> = cases
        .iter()
        .zip(EXPECTED)
        .filter(|((_, builds, _), _)| !cfg!(feature = "unstable") || *builds == Every)
        .collect();
    assert!(!held.is_empty());
    let wrong: Vec<_> = held
        .iter()
        .filter(|((_, _, result), (_, expected))| result != expected)
        .map(|((name, _, result), (_, expected))| {
            format!("{name}:\n  expected {expected}\n  got      {result}")
        })
        .collect();
    assert!(
        wrong.is_empty(),
        "{} of {} cases differ:\n{}",
        wrong.len(),
        held.len(),
        wrong.join("\n")
    );
}
