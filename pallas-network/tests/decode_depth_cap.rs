//! Decoding refuses nesting at `MAX_DECODE_DEPTH` (`GetCBOR` wrappers at a
//! lower limit of their own), so whatever decodes also drops. Debug builds
//! overflow well below the cap on a 2 MiB stack, so they drop on 16 MiB;
//! `cargo test --release` drops on 2 MiB, and `PlutusData` on 4 MiB because
//! its drop's stack use at the cap varies from build to build.
use pallas_codec::{
    MAX_DECODE_DEPTH,
    minicbor::{self, Decode},
};
use pallas_network::miniprotocols::{
    localstate::queries_v16::{BlockQuery, PlutusData},
    localtxsubmission::primitives::NativeScript,
};

const DROP_STACK: usize = if cfg!(debug_assertions) {
    16 << 20
} else {
    2 << 20
};

const PLUTUS_DROP_STACK: usize = if cfg!(debug_assertions) {
    16 << 20
} else {
    4 << 20
};

/// One nesting level as the bytes before and after its child, and a leaf.
type Shape = (&'static [u8], &'static [u8], &'static [u8]);

const PLUTUS: &[Shape] = &[
    (&[0x81], &[0x00], &[]),
    (&[0x9f], &[0x00], &[0xff]),
    (&[0xa1, 0x00], &[0x00], &[]),
    (&[0xbf, 0x00], &[0x00], &[0xff]),
    // The nested value is the map's key.
    (&[0xa1], &[0x00], &[0x00]),
    (&[0xd8, 0x79, 0x81], &[0x00], &[]),
    (&[0xd8, 0x66, 0x82, 0x00, 0x81], &[0x00], &[]),
];

const SCRIPTS: &[Shape] = &[
    (&[0x82, 0x01, 0x81], &[0x82, 0x04, 0x00], &[]),
    (&[0x82, 0x02, 0x9f], &[0x82, 0x04, 0x00], &[0xff]),
    (&[0x83, 0x03, 0x01, 0x81], &[0x82, 0x04, 0x00], &[]),
];

/// `GetCBOR` wrappers around `GetLedgerTip`.
const QUERIES: &[Shape] = &[(&[0x82, 0x09], &[0x81, 0x00], &[])];

fn nested((prefix, leaf, suffix): Shape, depth: usize) -> Vec<u8> {
    let mut bytes = prefix.repeat(depth);
    bytes.extend_from_slice(leaf);
    bytes.extend(suffix.repeat(depth));
    bytes
}

fn on_stack(size: usize, f: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(size)
        .spawn(f)
        .unwrap()
        .join()
        .unwrap();
}

fn assert_depth_error<T: for<'b> Decode<'b, ()>>(bytes: &[u8]) {
    let Err(error) = minicbor::decode::<T>(bytes) else {
        panic!("decoded nesting at or past the cap");
    };
    assert!(
        error.to_string().contains("decode depth limit"),
        "unexpected error: {error}"
    );
}

/// `BlockQuery`'s own, lower limit on `GetCBOR` wrappers.
const GET_CBOR_CAP: usize = 1_024;

fn check_cap<T: for<'b> Decode<'b, ()> + 'static>(
    shapes: &'static [Shape],
    cap: usize,
    stack: usize,
) {
    for &shape in shapes {
        assert_depth_error::<T>(&nested(shape, cap));
        on_stack(stack, move || {
            let value = minicbor::decode::<T>(&nested(shape, cap - 1)).unwrap();
            drop(value);
        });
        on_stack(256 << 10, move || {
            assert_depth_error::<T>(&nested(shape, 10 * MAX_DECODE_DEPTH));
        });
    }
}

#[test]
fn plutus_data_stops_at_the_depth_cap() {
    check_cap::<PlutusData>(PLUTUS, MAX_DECODE_DEPTH, PLUTUS_DROP_STACK);
}

#[test]
fn native_script_stops_at_the_depth_cap() {
    check_cap::<NativeScript>(SCRIPTS, MAX_DECODE_DEPTH, DROP_STACK);
}

#[test]
fn block_query_stops_at_the_depth_cap() {
    check_cap::<BlockQuery>(QUERIES, GET_CBOR_CAP, DROP_STACK);
}
