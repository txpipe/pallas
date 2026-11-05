//! Fixture decoders and transaction builders for the schema test modules.

use pallas_codec::minicbor::{self, data::Tag};
use pallas_traverse as trv;

use crate::{LedgerContext, TxoRef, UtxoMap};

/// A mapper context that resolves no output.
#[derive(Clone)]
pub struct NoLedger;

impl LedgerContext for NoLedger {
    fn get_utxos(&self, _refs: &[TxoRef]) -> Option<UtxoMap> {
        None
    }

    fn get_slot_timestamp(&self, _slot: u64) -> Option<u64> {
        None
    }
}

/// A mapper context that resolves the outputs it holds and no others.
#[derive(Clone)]
pub struct HeldOutputs(pub UtxoMap);

impl LedgerContext for HeldOutputs {
    fn get_utxos(&self, refs: &[TxoRef]) -> Option<UtxoMap> {
        Some(
            refs.iter()
                .filter_map(|x| self.0.get_key_value(x))
                .map(|(k, v)| (*k, v.clone()))
                .collect(),
        )
    }

    fn get_slot_timestamp(&self, _slot: u64) -> Option<u64> {
        None
    }
}

/// A Conway output that pays the coin given to an enterprise testnet address.
pub fn conway_output(coin: u64) -> crate::EraCbor {
    let mut address = [0x41; 29];
    address[0] = 0x60;

    let mut e = minicbor::Encoder::new(Vec::new());
    e.array(2).unwrap();
    e.bytes(&address).unwrap();
    e.u64(coin).unwrap();

    (trv::Era::Conway, e.into_writer())
}

/// The Conway redeemer tag of a certificate's redeemer.
const CERTIFICATE_REDEEMER_TAG: u8 = 2;

/// Decodes a hex encoded Conway transaction.
pub fn conway_tx(tx_str: &str) -> trv::MultiEraTx<'_> {
    let cbor: &'static [u8] = Box::leak(hex::decode(tx_str).unwrap().into_boxed_slice());
    trv::MultiEraTx::decode_for_era(trv::Era::Conway, cbor).unwrap()
}

/// Decodes a hex encoded Dijkstra block.
pub fn dijkstra_block(block_str: &str) -> trv::MultiEraBlock<'_> {
    let cbor: &'static [u8] = Box::leak(hex::decode(block_str).unwrap().into_boxed_slice());
    trv::MultiEraBlock::decode(cbor).unwrap()
}

/// Decodes a hex encoded Dijkstra transaction.
pub fn dijkstra_tx(tx_str: &str) -> trv::MultiEraTx<'_> {
    let cbor: &'static [u8] = Box::leak(hex::decode(tx_str).unwrap().into_boxed_slice());
    trv::MultiEraTx::decode_for_era(trv::Era::Dijkstra, cbor).unwrap()
}

/// A Conway transaction with a redeemer for the certificate at each of positions 0 and 1.
pub fn conway_tx_with_certificate_redeemers() -> trv::MultiEraTx<'static> {
    let mut e = minicbor::Encoder::new(Vec::new());
    e.array(4).unwrap();

    // One input, no outputs, a fee.
    e.map(3).unwrap();
    e.u8(0).unwrap();
    e.tag(Tag::new(258)).unwrap();
    e.array(1).unwrap();
    e.array(2).unwrap();
    e.bytes(&[0x11; 32]).unwrap();
    e.u8(0).unwrap();
    e.u8(1).unwrap();
    e.array(0).unwrap();
    e.u8(2).unwrap();
    e.u32(1_000).unwrap();

    // The redeemer map, keyed by purpose and position.
    e.map(1).unwrap();
    e.u8(5).unwrap();
    e.map(2).unwrap();
    for (order, memory, steps) in [(0u32, 1_000u64, 2_000u64), (1, 3_000, 4_000)] {
        e.array(2).unwrap();
        e.u8(CERTIFICATE_REDEEMER_TAG).unwrap();
        e.u32(order).unwrap();

        e.array(2).unwrap();
        e.tag(Tag::new(121)).unwrap();
        e.array(0).unwrap();
        e.array(2).unwrap();
        e.u64(memory).unwrap();
        e.u64(steps).unwrap();
    }

    e.bool(true).unwrap();
    e.null().unwrap();

    let cbor: &'static [u8] = Box::leak(e.into_writer().into_boxed_slice());
    trv::MultiEraTx::decode_for_era(trv::Era::Conway, cbor).unwrap()
}

/// The ttl `conway_tx_with_optional_body_keys` writes at body key 3.
pub const OPTIONAL_TTL: u64 = 900;

/// The auxiliary data hash `conway_tx_with_optional_body_keys` writes at body key 7.
pub const OPTIONAL_AUX_DATA_HASH: [u8; 32] = [0x21; 32];

/// The validity start `conway_tx_with_optional_body_keys` writes at body key 8.
pub const OPTIONAL_VALIDITY_START: u64 = 100;

/// The script data hash `conway_tx_with_optional_body_keys` writes at body key 11.
pub const OPTIONAL_SCRIPT_DATA_HASH: [u8; 32] = [0x22; 32];

/// The network id `conway_tx_with_optional_body_keys` writes at body key 15.
pub const OPTIONAL_NETWORK_ID: u8 = 0;

/// The total collateral `conway_tx_with_optional_body_keys` writes at body key 17.
pub const OPTIONAL_TOTAL_COLLATERAL: u64 = 5_000;

/// The treasury value `conway_tx_with_optional_body_keys` writes at body key 21.
pub const OPTIONAL_TREASURY_VALUE: u64 = 7_000;

/// The treasury donation `conway_tx_with_optional_body_keys` writes at body key 22.
pub const OPTIONAL_DONATION: u64 = 50;

/// A Conway transaction whose body writes keys 3, 7, 8, 11, 13, 15, 17, 21 and 22 and whose auxiliary data is an empty metadata map.
pub fn conway_tx_with_optional_body_keys() -> trv::MultiEraTx<'static> {
    let mut e = minicbor::Encoder::new(Vec::new());
    e.array(4).unwrap();

    e.map(12).unwrap();
    e.u8(0).unwrap();
    e.tag(Tag::new(258)).unwrap();
    e.array(1).unwrap();
    e.array(2).unwrap();
    e.bytes(&[0x11; 32]).unwrap();
    e.u8(0).unwrap();
    e.u8(1).unwrap();
    e.array(0).unwrap();
    e.u8(2).unwrap();
    e.u32(1_000).unwrap();
    e.u8(3).unwrap();
    e.u64(OPTIONAL_TTL).unwrap();
    e.u8(7).unwrap();
    e.bytes(&OPTIONAL_AUX_DATA_HASH).unwrap();
    e.u8(8).unwrap();
    e.u64(OPTIONAL_VALIDITY_START).unwrap();
    e.u8(11).unwrap();
    e.bytes(&OPTIONAL_SCRIPT_DATA_HASH).unwrap();
    e.u8(13).unwrap();
    e.tag(Tag::new(258)).unwrap();
    e.array(1).unwrap();
    e.array(2).unwrap();
    e.bytes(&[0x12; 32]).unwrap();
    e.u8(0).unwrap();
    e.u8(15).unwrap();
    e.u8(OPTIONAL_NETWORK_ID).unwrap();
    e.u8(17).unwrap();
    e.u64(OPTIONAL_TOTAL_COLLATERAL).unwrap();
    e.u8(21).unwrap();
    e.u64(OPTIONAL_TREASURY_VALUE).unwrap();
    e.u8(22).unwrap();
    e.u64(OPTIONAL_DONATION).unwrap();

    e.map(0).unwrap();
    e.bool(true).unwrap();
    e.map(0).unwrap();

    let cbor: &'static [u8] = Box::leak(e.into_writer().into_boxed_slice());
    trv::MultiEraTx::decode_for_era(trv::Era::Conway, cbor).unwrap()
}

/// The first key hash of the required signers in `conway_tx_with_required_signers`.
pub const FIRST_REQUIRED_SIGNER: [u8; 28] = [0x41; 28];

/// The second key hash of the required signers in `conway_tx_with_required_signers`.
pub const SECOND_REQUIRED_SIGNER: [u8; 28] = [0x42; 28];

/// A Conway transaction whose body requires two signers.
pub fn conway_tx_with_required_signers() -> trv::MultiEraTx<'static> {
    let mut e = minicbor::Encoder::new(Vec::new());
    e.array(4).unwrap();

    e.map(4).unwrap();
    e.u8(0).unwrap();
    e.tag(Tag::new(258)).unwrap();
    e.array(1).unwrap();
    e.array(2).unwrap();
    e.bytes(&[0x12; 32]).unwrap();
    e.u8(0).unwrap();
    e.u8(1).unwrap();
    e.array(0).unwrap();
    e.u8(2).unwrap();
    e.u32(1_000).unwrap();
    e.u8(14).unwrap();
    e.tag(Tag::new(258)).unwrap();
    e.array(2).unwrap();
    e.bytes(&FIRST_REQUIRED_SIGNER).unwrap();
    e.bytes(&SECOND_REQUIRED_SIGNER).unwrap();

    e.map(0).unwrap();
    e.bool(true).unwrap();
    e.null().unwrap();

    let cbor: &'static [u8] = Box::leak(e.into_writer().into_boxed_slice());
    trv::MultiEraTx::decode_for_era(trv::Era::Conway, cbor).unwrap()
}

/// A testnet key reward account whose 28 credential bytes are all the byte given.
const fn reward_account(fill: u8) -> [u8; 29] {
    let mut account = [fill; 29];
    account[0] = 0xe0;
    account
}

/// The account `dijkstra_tx_with_account_fields` deposits into.
pub const DEPOSIT_ACCOUNT: [u8; 29] = reward_account(0x21);

/// The coin `dijkstra_tx_with_account_fields` deposits into `DEPOSIT_ACCOUNT`.
pub const DEPOSIT_COIN: u64 = 4_000_000;

/// The account of `dijkstra_tx_with_account_fields` whose interval has only a lower bound.
pub const LOWER_BOUND_ACCOUNT: [u8; 29] = reward_account(0x22);

/// The account of `dijkstra_tx_with_account_fields` whose interval has only an upper bound.
pub const UPPER_BOUND_ACCOUNT: [u8; 29] = reward_account(0x23);

/// The account of `dijkstra_tx_with_account_fields` whose interval has both bounds.
pub const BOUNDED_ACCOUNT: [u8; 29] = reward_account(0x24);

/// The account of `dijkstra_tx_with_account_fields` whose interval is a bare coin.
pub const COIN_ACCOUNT: [u8; 29] = reward_account(0x25);

/// The account of the one starting interval of `dijkstra_tx_with_account_fields`, a lower bound.
pub const STARTING_ACCOUNT: [u8; 29] = reward_account(0x26);

/// The account the sub transaction of `dijkstra_tx_with_account_fields` deposits into.
pub const SUB_DEPOSIT_ACCOUNT: [u8; 29] = reward_account(0x27);

/// The account of the interval of the sub transaction of `dijkstra_tx_with_account_fields`, an upper bound.
pub const SUB_INTERVAL_ACCOUNT: [u8; 29] = reward_account(0x28);

/// Writes `[lower, upper]` with nil for a bound that is absent.
fn write_interval(e: &mut minicbor::Encoder<Vec<u8>>, lower: Option<u64>, upper: Option<u64>) {
    e.array(2).unwrap();
    for bound in [lower, upper] {
        match bound {
            Some(coin) => e.u64(coin).unwrap(),
            None => e.null().unwrap(),
        };
    }
}

/// A Dijkstra transaction with a direct deposit, an account balance interval of each of the four shapes, a starting interval, and one sub transaction with a deposit and an interval of its own.
pub fn dijkstra_tx_with_account_fields() -> trv::MultiEraTx<'static> {
    let mut e = minicbor::Encoder::new(Vec::new());
    e.array(4).unwrap();

    e.map(7).unwrap();
    e.u8(0).unwrap();
    e.tag(Tag::new(258)).unwrap();
    e.array(1).unwrap();
    e.array(2).unwrap();
    e.bytes(&[0x11; 32]).unwrap();
    e.u8(0).unwrap();
    e.u8(1).unwrap();
    e.array(0).unwrap();
    e.u8(2).unwrap();
    e.u32(1_000).unwrap();

    // sub_transaction = [body, witness set, auxiliary data or nil]
    e.u8(23).unwrap();
    e.tag(Tag::new(258)).unwrap();
    e.array(1).unwrap();
    e.array(3).unwrap();
    e.map(4).unwrap();
    e.u8(0).unwrap();
    e.tag(Tag::new(258)).unwrap();
    e.array(1).unwrap();
    e.array(2).unwrap();
    e.bytes(&[0x12; 32]).unwrap();
    e.u8(0).unwrap();
    e.u8(1).unwrap();
    e.array(0).unwrap();
    e.u8(25).unwrap();
    e.map(1).unwrap();
    e.bytes(&SUB_DEPOSIT_ACCOUNT).unwrap();
    e.u64(12).unwrap();
    e.u8(26).unwrap();
    e.map(1).unwrap();
    e.bytes(&SUB_INTERVAL_ACCOUNT).unwrap();
    write_interval(&mut e, None, Some(13));
    e.map(0).unwrap();
    e.null().unwrap();

    e.u8(25).unwrap();
    e.map(1).unwrap();
    e.bytes(&DEPOSIT_ACCOUNT).unwrap();
    e.u64(DEPOSIT_COIN).unwrap();

    e.u8(26).unwrap();
    e.map(4).unwrap();
    e.bytes(&LOWER_BOUND_ACCOUNT).unwrap();
    write_interval(&mut e, Some(5), None);
    e.bytes(&UPPER_BOUND_ACCOUNT).unwrap();
    write_interval(&mut e, None, Some(6));
    e.bytes(&BOUNDED_ACCOUNT).unwrap();
    write_interval(&mut e, Some(7), Some(8));
    e.bytes(&COIN_ACCOUNT).unwrap();
    e.u64(9).unwrap();

    e.u8(27).unwrap();
    e.map(1).unwrap();
    e.bytes(&STARTING_ACCOUNT).unwrap();
    write_interval(&mut e, Some(10), None);

    e.map(0).unwrap();
    e.null().unwrap();
    e.bool(true).unwrap();

    let cbor: &'static [u8] = Box::leak(e.into_writer().into_boxed_slice());
    trv::MultiEraTx::decode_for_era(trv::Era::Dijkstra, cbor).unwrap()
}

/// The key credential of the top body's guards.
pub const GUARD_KEY_HASH: [u8; 28] = [0x31; 28];

/// The script credential of the top body's guards.
pub const GUARD_SCRIPT_HASH: [u8; 28] = [0x32; 28];

/// The key hash of the sub body's guards.
pub const SUB_GUARD_KEY_HASH: [u8; 28] = [0x33; 28];

/// A Dijkstra transaction whose guards are a key and a script credential, and whose one sub transaction's guards are a bare key hash.
pub fn dijkstra_tx_with_guards() -> trv::MultiEraTx<'static> {
    let mut e = minicbor::Encoder::new(Vec::new());
    e.array(4).unwrap();

    e.map(5).unwrap();
    e.u8(0).unwrap();
    e.tag(Tag::new(258)).unwrap();
    e.array(1).unwrap();
    e.array(2).unwrap();
    e.bytes(&[0x13; 32]).unwrap();
    e.u8(0).unwrap();
    e.u8(1).unwrap();
    e.array(0).unwrap();
    e.u8(2).unwrap();
    e.u32(1_000).unwrap();

    e.u8(14).unwrap();
    e.tag(Tag::new(258)).unwrap();
    e.array(2).unwrap();
    e.array(2).unwrap();
    e.u8(0).unwrap();
    e.bytes(&GUARD_KEY_HASH).unwrap();
    e.array(2).unwrap();
    e.u8(1).unwrap();
    e.bytes(&GUARD_SCRIPT_HASH).unwrap();

    e.u8(23).unwrap();
    e.tag(Tag::new(258)).unwrap();
    e.array(1).unwrap();
    e.array(3).unwrap();
    e.map(3).unwrap();
    e.u8(0).unwrap();
    e.tag(Tag::new(258)).unwrap();
    e.array(1).unwrap();
    e.array(2).unwrap();
    e.bytes(&[0x14; 32]).unwrap();
    e.u8(0).unwrap();
    e.u8(1).unwrap();
    e.array(0).unwrap();
    e.u8(14).unwrap();
    e.tag(Tag::new(258)).unwrap();
    e.array(1).unwrap();
    e.bytes(&SUB_GUARD_KEY_HASH).unwrap();
    e.map(0).unwrap();
    e.null().unwrap();

    e.map(0).unwrap();
    e.null().unwrap();
    e.bool(true).unwrap();

    let cbor: &'static [u8] = Box::leak(e.into_writer().into_boxed_slice());
    trv::MultiEraTx::decode_for_era(trv::Era::Dijkstra, cbor).unwrap()
}

/// The key credential the top body requires of the top level guards, with a datum.
pub const TOP_LEVEL_GUARD_KEY_HASH: [u8; 28] = [0x34; 28];

/// The integer datum the top body gives its key credential.
pub const TOP_LEVEL_GUARD_DATUM: i64 = 7;

/// The script credential the top body requires of the top level guards, with no datum.
pub const TOP_LEVEL_GUARD_SCRIPT_HASH: [u8; 28] = [0x35; 28];

/// The key credential the sub body requires of the top level guards.
pub const SUB_TOP_LEVEL_GUARD_KEY_HASH: [u8; 28] = [0x36; 28];

/// A Dijkstra transaction whose body requires a key credential with an integer datum and a script credential with none, and whose one sub body requires a key credential with a constructor datum.
pub fn dijkstra_tx_with_required_top_level_guards() -> trv::MultiEraTx<'static> {
    let mut e = minicbor::Encoder::new(Vec::new());
    e.array(4).unwrap();

    e.map(5).unwrap();
    e.u8(0).unwrap();
    e.tag(Tag::new(258)).unwrap();
    e.array(1).unwrap();
    e.array(2).unwrap();
    e.bytes(&[0x15; 32]).unwrap();
    e.u8(0).unwrap();
    e.u8(1).unwrap();
    e.array(0).unwrap();
    e.u8(2).unwrap();
    e.u32(1_000).unwrap();

    e.u8(23).unwrap();
    e.tag(Tag::new(258)).unwrap();
    e.array(1).unwrap();
    e.array(3).unwrap();
    e.map(3).unwrap();
    e.u8(0).unwrap();
    e.tag(Tag::new(258)).unwrap();
    e.array(1).unwrap();
    e.array(2).unwrap();
    e.bytes(&[0x16; 32]).unwrap();
    e.u8(0).unwrap();
    e.u8(1).unwrap();
    e.array(0).unwrap();
    e.u8(24).unwrap();
    e.map(1).unwrap();
    e.array(2).unwrap();
    e.u8(0).unwrap();
    e.bytes(&SUB_TOP_LEVEL_GUARD_KEY_HASH).unwrap();
    e.tag(Tag::new(121)).unwrap();
    e.array(0).unwrap();
    e.map(0).unwrap();
    e.null().unwrap();

    e.u8(24).unwrap();
    e.map(2).unwrap();
    e.array(2).unwrap();
    e.u8(0).unwrap();
    e.bytes(&TOP_LEVEL_GUARD_KEY_HASH).unwrap();
    e.i64(TOP_LEVEL_GUARD_DATUM).unwrap();
    e.array(2).unwrap();
    e.u8(1).unwrap();
    e.bytes(&TOP_LEVEL_GUARD_SCRIPT_HASH).unwrap();
    e.null().unwrap();

    e.map(0).unwrap();
    e.null().unwrap();
    e.bool(true).unwrap();

    let cbor: &'static [u8] = Box::leak(e.into_writer().into_boxed_slice());
    trv::MultiEraTx::decode_for_era(trv::Era::Dijkstra, cbor).unwrap()
}

/// The PlutusV2, PlutusV3 and PlutusV4 scripts in the auxiliary data of `dijkstra_tx_with_aux_plutus_scripts`.
pub const AUX_PLUTUS_SCRIPTS: [&[u8]; 3] = [&[0x42], &[0x43], &[0x44]];

/// The PlutusV2, PlutusV3 and PlutusV4 scripts in the auxiliary data of the sub transaction of `dijkstra_tx_with_aux_plutus_scripts`.
pub const SUB_AUX_PLUTUS_SCRIPTS: [&[u8]; 3] = [&[0x52], &[0x53], &[0x54]];

/// Writes post Alonzo auxiliary data holding the scripts given at keys 3, 4 and 5, PlutusV2 to PlutusV4.
fn write_aux_plutus_scripts(e: &mut minicbor::Encoder<Vec<u8>>, scripts: [&[u8]; 3]) {
    e.tag(Tag::new(259)).unwrap();
    e.map(3).unwrap();
    for (key, script) in [3u8, 4, 5].into_iter().zip(scripts) {
        e.u8(key).unwrap();
        e.array(1).unwrap();
        e.bytes(script).unwrap();
    }
}

/// A Dijkstra transaction whose auxiliary data holds `AUX_PLUTUS_SCRIPTS`, and whose one sub transaction's auxiliary data holds `SUB_AUX_PLUTUS_SCRIPTS`.
pub fn dijkstra_tx_with_aux_plutus_scripts() -> trv::MultiEraTx<'static> {
    let mut e = minicbor::Encoder::new(Vec::new());
    e.array(4).unwrap();

    e.map(4).unwrap();
    e.u8(0).unwrap();
    e.tag(Tag::new(258)).unwrap();
    e.array(1).unwrap();
    e.array(2).unwrap();
    e.bytes(&[0x17; 32]).unwrap();
    e.u8(0).unwrap();
    e.u8(1).unwrap();
    e.array(0).unwrap();
    e.u8(2).unwrap();
    e.u32(1_000).unwrap();

    e.u8(23).unwrap();
    e.tag(Tag::new(258)).unwrap();
    e.array(1).unwrap();
    e.array(3).unwrap();
    e.map(2).unwrap();
    e.u8(0).unwrap();
    e.tag(Tag::new(258)).unwrap();
    e.array(1).unwrap();
    e.array(2).unwrap();
    e.bytes(&[0x18; 32]).unwrap();
    e.u8(0).unwrap();
    e.u8(1).unwrap();
    e.array(0).unwrap();
    e.map(0).unwrap();
    write_aux_plutus_scripts(&mut e, SUB_AUX_PLUTUS_SCRIPTS);

    e.map(0).unwrap();
    write_aux_plutus_scripts(&mut e, AUX_PLUTUS_SCRIPTS);
    e.bool(true).unwrap();

    let cbor: &'static [u8] = Box::leak(e.into_writer().into_boxed_slice());
    trv::MultiEraTx::decode_for_era(trv::Era::Dijkstra, cbor).unwrap()
}

/// A Conway parameter update read from the CBOR map given.
pub fn conway_update(update_cbor: &[u8]) -> pallas_primitives::conway::ProtocolParamUpdate {
    minicbor::decode(update_cbor).expect("a parameter update map decodes")
}
