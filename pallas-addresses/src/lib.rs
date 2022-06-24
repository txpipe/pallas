//! Interact with Cardano addresses of any type
//!
//! This module contains utilities to decode / encode Cardano addresses from /
//! to different formats. The entry point to most of the methods is the
//! [Address] enum, which holds the decoded values of either a Byron, Shelley or
//! Stake address.
//!
//! For more information regarding Cardano addresses and their formats, please refer to [CIP-19](https://cips.cardano.org/cips/cip19/).

pub mod varuint;

use std::io::Cursor;

use pallas_crypto::hash::Hash;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("error converting from/to bech32 {0}")]
    BadBech32(bech32::Error),

    #[error("address header not found")]
    MissingHeader,

    #[error("address header is invalid {0:08b}")]
    InvalidHeader(u8),

    #[error("invalid operation for Byron address")]
    InvalidForByron,

    #[error("unkown hrp for network {0:08b}")]
    UnknownNetworkHrp(u8),

    #[error("invalid hash size {0}")]
    InvalidHashSize(usize),

    #[error("invalid pointer data")]
    InvalidPointerData,

    #[error("variable-length uint error: {0}")]
    VarUintError(varuint::Error),
}

pub type PaymentKeyHash = Hash<28>;

pub type StakeKeyHash = Hash<28>;

pub type ScriptHash = Hash<28>;

pub type Slot = u64;
pub type TxIdx = u64;
pub type CertIdx = u64;

/// An on-chain pointer to a stake key
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct Pointer(Slot, TxIdx, CertIdx);

fn slice_to_hash(slice: &[u8]) -> Result<Hash<28>, Error> {
    if slice.len() == 28 {
        let mut sized = [0u8; 28];
        sized.copy_from_slice(slice);
        Ok(sized.into())
    } else {
        Err(Error::InvalidHashSize(slice.len()))
    }
}

impl Pointer {
    pub fn new(slot: Slot, tx_idx: TxIdx, cert_idx: CertIdx) -> Self {
        Pointer(slot, tx_idx, cert_idx)
    }

    pub fn parse(bytes: &[u8]) -> Result<Self, Error> {
        let mut cursor = Cursor::new(bytes);
        let a = varuint::read(&mut cursor).map_err(Error::VarUintError)?;
        let b = varuint::read(&mut cursor).map_err(Error::VarUintError)?;
        let c = varuint::read(&mut cursor).map_err(Error::VarUintError)?;

        Ok(Pointer(a, b, c))
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut cursor = Cursor::new(vec![]);
        varuint::write(&mut cursor, self.0);
        varuint::write(&mut cursor, self.1);
        varuint::write(&mut cursor, self.2);

        cursor.into_inner()
    }

    pub fn slot(&self) -> u64 {
        self.0
    }

    pub fn tx_idx(&self) -> u64 {
        self.1
    }

    pub fn cert_idx(&self) -> u64 {
        self.2
    }
}

/// The payment part of a Shelley address
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum ShelleyPaymentPart {
    PaymentKey(PaymentKeyHash),
    Script(ScriptHash),
}

impl ShelleyPaymentPart {
    fn payment_key(bytes: &[u8]) -> Result<Self, Error> {
        slice_to_hash(bytes).map(ShelleyPaymentPart::PaymentKey)
    }

    fn script(bytes: &[u8]) -> Result<Self, Error> {
        slice_to_hash(bytes).map(ShelleyPaymentPart::Script)
    }

    /// Get a reference to the inner hash of this address part
    pub fn as_hash(&self) -> &Hash<28> {
        match self {
            Self::PaymentKey(x) => x,
            Self::Script(x) => x,
        }
    }

    /// Encodes this address as a sequence of bytes
    pub fn to_vec(&self) -> Vec<u8> {
        match self {
            Self::PaymentKey(x) => x.to_vec(),
            Self::Script(x) => x.to_vec(),
        }
    }

    /// Indicates if this is the hash of a script
    pub fn is_script(&self) -> bool {
        matches!(self, Self::Script(_))
    }
}

/// The delegation part of a Shelley address
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum ShelleyDelegationPart {
    StakeKey(StakeKeyHash),
    Script(ScriptHash),
    Pointer(Pointer),
    Null,
}

impl ShelleyDelegationPart {
    fn stake_key(bytes: &[u8]) -> Result<Self, Error> {
        slice_to_hash(bytes).map(Self::StakeKey)
    }

    fn script(bytes: &[u8]) -> Result<Self, Error> {
        slice_to_hash(bytes).map(Self::Script)
    }

    fn pointer(bytes: &[u8]) -> Result<Self, Error> {
        let pointer = Pointer::parse(bytes)?;
        Ok(Self::Pointer(pointer))
    }

    /// Get a reference to the inner hash of this address part
    pub fn as_hash(&self) -> Option<&Hash<28>> {
        match self {
            Self::StakeKey(x) => Some(x),
            Self::Script(x) => Some(x),
            Self::Pointer(_) => todo!(),
            Self::Null => todo!(),
        }
    }

    pub fn to_vec(&self) -> Vec<u8> {
        match self {
            Self::StakeKey(x) => x.to_vec(),
            Self::Script(x) => x.to_vec(),
            Self::Pointer(x) => x.to_vec(),
            Self::Null => vec![],
        }
    }

    pub fn is_script(&self) -> bool {
        matches!(self, ShelleyDelegationPart::Script(_))
    }
}

impl StakePayload {
    fn stake_key(bytes: &[u8]) -> Result<Self, Error> {
        slice_to_hash(bytes).map(StakePayload::Stake)
    }

    fn script(bytes: &[u8]) -> Result<Self, Error> {
        slice_to_hash(bytes).map(StakePayload::Script)
    }

    pub fn is_script(&self) -> bool {
        matches!(self, StakePayload::Script(_))
    }
}

/// The network tag of an address
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum Network {
    Testnet,
    Mainnet,
    Other(u8),
}

/// A decoded Shelley address
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct ShelleyAddress(Network, ShelleyPaymentPart, ShelleyDelegationPart);

/// The payload of a Stake address
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum StakePayload {
    Stake(StakeKeyHash),
    Script(ScriptHash),
}

/// A decoded Stake address
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct StakeAddress(Network, StakePayload);

/// Newtype representing a Byron address
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct ByronAddress(Vec<u8>);

/// A decoded Cardano address of any type
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum Address {
    Byron(ByronAddress),
    Shelley(ShelleyAddress),
    Stake(StakeAddress),
}

fn encode_bech32(addr: &[u8], hrp: &str) -> Result<String, Error> {
    let base32 = bech32::ToBase32::to_base32(&addr);
    bech32::encode(hrp, base32, bech32::Variant::Bech32).map_err(Error::BadBech32)
}

fn decode_bech32(bech32: &str) -> Result<(String, Vec<u8>), Error> {
    let (hrp, addr, _) = bech32::decode(bech32).map_err(Error::BadBech32)?;
    let base10 = bech32::FromBase32::from_base32(&addr).map_err(Error::BadBech32)?;
    Ok((hrp, base10))
}

fn parse_network(header: u8) -> Network {
    let masked = header & 0b0000_1111;

    match masked {
        0b_0000_0000 => Network::Testnet,
        0b_0000_0001 => Network::Mainnet,
        _ => Network::Other(masked),
    }
}

macro_rules! parse_shelley_fn {
    ($name:tt, $payment:tt, pointer) => {
        fn $name(header: u8, payload: &[u8]) -> Result<Address, Error> {
            let net = parse_network(header);
            let p1 = ShelleyPaymentPart::$payment(&payload[0..=27])?;
            let p2 = ShelleyDelegationPart::pointer(&payload[28..])?;
            let addr = ShelleyAddress(net, p1, p2);

            Ok(addr.into())
        }
    };
    ($name:tt, $payment:tt, $delegation:tt) => {
        fn $name(header: u8, payload: &[u8]) -> Result<Address, Error> {
            let net = parse_network(header);
            let p1 = ShelleyPaymentPart::$payment(&payload[0..=27])?;
            let p2 = ShelleyDelegationPart::$delegation(&payload[28..=55])?;
            let addr = ShelleyAddress(net, p1, p2);

            Ok(addr.into())
        }
    };
    ($name:tt, $payment:tt) => {
        fn $name(header: u8, payload: &[u8]) -> Result<Address, Error> {
            let net = parse_network(header);
            let p1 = ShelleyPaymentPart::$payment(&payload[0..=27])?;
            let addr = ShelleyAddress(net, p1, ShelleyDelegationPart::Null);

            Ok(addr.into())
        }
    };
}

macro_rules! parse_stake_fn {
    ($name:tt, $type:tt) => {
        fn $name(header: u8, payload: &[u8]) -> Result<Address, Error> {
            let net = parse_network(header);
            let p1 = StakePayload::$type(&payload[0..=27])?;
            let addr = StakeAddress(net, p1);

            Ok(addr.into())
        }
    };
}

// types 0-7 are Shelley addresses
parse_shelley_fn!(parse_type_0, payment_key, stake_key);
parse_shelley_fn!(parse_type_1, script, stake_key);
parse_shelley_fn!(parse_type_2, payment_key, script);
parse_shelley_fn!(parse_type_3, script, script);
parse_shelley_fn!(parse_type_4, payment_key, pointer);
parse_shelley_fn!(parse_type_5, script, pointer);
parse_shelley_fn!(parse_type_6, payment_key);
parse_shelley_fn!(parse_type_7, script);

// type 8 (1000) are Byron addresses
fn parse_type_8(header: u8, payload: &[u8]) -> Result<Address, Error> {
    let vec = [&[header], payload].concat();
    Ok(Address::Byron(ByronAddress(vec)))
}

// types 14-15 are Stake addresses
parse_stake_fn!(parse_type_14, stake_key);
parse_stake_fn!(parse_type_15, script);

fn bytes_to_address(bytes: &[u8]) -> Result<Address, Error> {
    let header = *bytes.get(0).ok_or(Error::MissingHeader)?;
    let payload = &bytes[1..];

    match header & 0b1111_0000 {
        0b0000_0000 => parse_type_0(header, payload),
        0b0001_0000 => parse_type_1(header, payload),
        0b0010_0000 => parse_type_2(header, payload),
        0b0011_0000 => parse_type_3(header, payload),
        0b0100_0000 => parse_type_4(header, payload),
        0b0101_0000 => parse_type_5(header, payload),
        0b0110_0000 => parse_type_6(header, payload),
        0b0111_0000 => parse_type_7(header, payload),
        0b1000_0000 => parse_type_8(header, payload),
        0b1110_0000 => parse_type_14(header, payload),
        0b1111_0000 => parse_type_15(header, payload),
        _ => Err(Error::InvalidHeader(header)),
    }
}

fn bech32_to_address(bech32: &str) -> Result<Address, Error> {
    let (_, bytes) = decode_bech32(bech32)?;
    bytes_to_address(&bytes)
}

fn address_to_bech32(addr: &Address) -> Result<String, Error> {
    match addr {
        Address::Byron(_) => Err(Error::InvalidForByron),
        Address::Shelley(ref x) => {
            let hrp = x.hrp()?;
            let bytes = x.to_vec();
            encode_bech32(&bytes, hrp)
        }
        Address::Stake(ref x) => {
            let hrp = x.hrp()?;
            let bytes = x.to_vec();
            encode_bech32(&bytes, hrp)
        }
    }
}

impl Network {
    pub fn is_mainnet(&self) -> bool {
        matches!(self, Network::Mainnet)
    }

    pub fn value(&self) -> u8 {
        match self {
            Network::Testnet => 0,
            Network::Mainnet => 1,
            Network::Other(x) => *x,
        }
    }
}

impl ByronAddress {
    /// Gets a numeric id describing the type of the address
    pub fn typeid(&self) -> u8 {
        0b1000
    }
}

impl ShelleyAddress {
    /// Gets the network assoaciated with this address
    pub fn network(&self) -> Network {
        self.0
    }

    /// Gets a numeric id describing the type of the address
    pub fn typeid(&self) -> u8 {
        match (&self.1, &self.2) {
            (ShelleyPaymentPart::PaymentKey(_), ShelleyDelegationPart::StakeKey(_)) => 0b0000,
            (ShelleyPaymentPart::Script(_), ShelleyDelegationPart::StakeKey(_)) => 0b0001,
            (ShelleyPaymentPart::PaymentKey(_), ShelleyDelegationPart::Script(_)) => 0b0010,
            (ShelleyPaymentPart::Script(_), ShelleyDelegationPart::Script(_)) => 0b0011,
            (ShelleyPaymentPart::PaymentKey(_), ShelleyDelegationPart::Pointer(_)) => 0b0100,
            (ShelleyPaymentPart::Script(_), ShelleyDelegationPart::Pointer(_)) => 0b0101,
            (ShelleyPaymentPart::PaymentKey(_), ShelleyDelegationPart::Null) => 0b0110,
            (ShelleyPaymentPart::Script(_), ShelleyDelegationPart::Null) => 0b0111,
        }
    }

    pub fn to_header(&self) -> u8 {
        let type_id = self.typeid();
        let type_id = type_id << 4;
        let network = self.0.value();

        type_id | network
    }

    pub fn payment(&self) -> &ShelleyPaymentPart {
        &self.1
    }

    pub fn delegation(&self) -> &ShelleyDelegationPart {
        &self.2
    }

    /// Gets the bech32 human-readable-part for this address
    pub fn hrp(&self) -> Result<&'static str, Error> {
        match &self.0 {
            Network::Testnet => Ok("addr_test"),
            Network::Mainnet => Ok("addr"),
            Network::Other(x) => Err(Error::UnknownNetworkHrp(*x)),
        }
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let header = self.to_header();
        let payment = self.1.to_vec();
        let delegation = self.2.to_vec();

        [&[header], payment.as_slice(), delegation.as_slice()].concat()
    }

    /// Indicates if either the payment or delegation part is a script
    pub fn has_script(&self) -> bool {
        self.payment().is_script() || self.delegation().is_script()
    }
}

impl AsRef<[u8]> for StakePayload {
    fn as_ref(&self) -> &[u8] {
        match self {
            Self::Stake(x) => x.as_ref(),
            Self::Script(x) => x.as_ref(),
        }
    }
}

impl StakeAddress {
    /// Gets the network assoaciated with this address
    pub fn network(&self) -> Network {
        self.0
    }

    /// Gets a numeric id describing the type of the address
    pub fn typeid(&self) -> u8 {
        match &self.1 {
            StakePayload::Stake(_) => 0b1110,
            StakePayload::Script(_) => 0b1111,
        }
    }

    /// Builds the header for this address
    pub fn to_header(&self) -> u8 {
        let type_id = self.typeid();
        let type_id = type_id << 4;
        let network = self.0.value();

        type_id | network
    }

    /// Gets the payload of this address
    pub fn payload(&self) -> &StakePayload {
        &self.1
    }

    /// Gets the bech32 human-readable-part for this address
    pub fn hrp(&self) -> Result<&'static str, Error> {
        match &self.0 {
            Network::Testnet => Ok("stake_test"),
            Network::Mainnet => Ok("stake"),
            Network::Other(x) => Err(Error::UnknownNetworkHrp(*x)),
        }
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let header = self.to_header();

        [&[header], self.1.as_ref()].concat()
    }

    pub fn is_script(&self) -> bool {
        self.payload().is_script()
    }
}

impl Address {
    /// Tries to encode an Address into a bech32 string
    pub fn to_bech32(&self) -> Result<String, Error> {
        address_to_bech32(self)
    }

    /// Tries to parse a bech32 address into an Address
    pub fn from_bech32(bech32: &str) -> Result<Self, Error> {
        bech32_to_address(bech32)
    }

    /// Gets the network assoaciated with this address
    pub fn network(&self) -> Option<Network> {
        match self {
            Address::Byron(_) => None,
            Address::Shelley(x) => Some(x.network()),
            Address::Stake(x) => Some(x.network()),
        }
    }

    /// Gets a numeric id describing the type of the address
    pub fn typeid(&self) -> u8 {
        match self {
            Address::Byron(x) => x.typeid(),
            Address::Shelley(x) => x.typeid(),
            Address::Stake(x) => x.typeid(),
        }
    }

    /// Gets the bech32 human-readable-part for this address
    pub fn hrp(&self) -> Result<&'static str, Error> {
        match self {
            Address::Byron(_) => Err(Error::InvalidForByron),
            Address::Shelley(x) => x.hrp(),
            Address::Stake(x) => x.hrp(),
        }
    }

    /// Indicates if this is address includes a script hash
    pub fn has_script(&self) -> bool {
        match self {
            Address::Byron(_) => false,
            Address::Shelley(x) => x.has_script(),
            Address::Stake(x) => x.is_script(),
        }
    }

    /// Indicates if this is an enterpise address
    pub fn is_enterprise(&self) -> bool {
        match self {
            Address::Shelley(x) => matches!(x.delegation(), ShelleyDelegationPart::Null),
            _ => false,
        }
    }
}

impl From<ByronAddress> for Address {
    fn from(addr: ByronAddress) -> Self {
        Address::Byron(addr)
    }
}

impl From<ShelleyAddress> for Address {
    fn from(addr: ShelleyAddress) -> Self {
        Address::Shelley(addr)
    }
}

impl From<StakeAddress> for Address {
    fn from(addr: StakeAddress) -> Self {
        Address::Stake(addr)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    const MAINNET_TEST_VECTORS: &[(&str, u8)] = &[
        ("addr1qx2fxv2umyhttkxyxp8x0dlpdt3k6cwng5pxj3jhsydzer3n0d3vllmyqwsx5wktcd8cc3sq835lu7drv2xwl2wywfgse35a3x", 00u8),
        ("addr1z8phkx6acpnf78fuvxn0mkew3l0fd058hzquvz7w36x4gten0d3vllmyqwsx5wktcd8cc3sq835lu7drv2xwl2wywfgs9yc0hh", 01u8),
        ("addr1yx2fxv2umyhttkxyxp8x0dlpdt3k6cwng5pxj3jhsydzerkr0vd4msrxnuwnccdxlhdjar77j6lg0wypcc9uar5d2shs2z78ve", 02u8),
        ("addr1x8phkx6acpnf78fuvxn0mkew3l0fd058hzquvz7w36x4gt7r0vd4msrxnuwnccdxlhdjar77j6lg0wypcc9uar5d2shskhj42g", 03u8),
        ("addr1gx2fxv2umyhttkxyxp8x0dlpdt3k6cwng5pxj3jhsydzer5pnz75xxcrzqf96k", 04u8),
        ("addr128phkx6acpnf78fuvxn0mkew3l0fd058hzquvz7w36x4gtupnz75xxcrtw79hu", 05u8),
        ("addr1vx2fxv2umyhttkxyxp8x0dlpdt3k6cwng5pxj3jhsydzers66hrl8", 06u8),
        ("addr1w8phkx6acpnf78fuvxn0mkew3l0fd058hzquvz7w36x4gtcyjy7wx", 07u8),
        ("stake1uyehkck0lajq8gr28t9uxnuvgcqrc6070x3k9r8048z8y5gh6ffgw", 14u8),
        ("stake178phkx6acpnf78fuvxn0mkew3l0fd058hzquvz7w36x4gtcccycj5", 15u8),
    ];

    const PAYMENT_PUBLIC_KEY: &str =
        "addr_vk1w0l2sr2zgfm26ztc6nl9xy8ghsk5sh6ldwemlpmp9xylzy4dtf7st80zhd";
    const STAKE_PUBLIC_KEY: &str =
        "stake_vk1px4j0r2fk7ux5p23shz8f3y5y2qam7s954rgf3lg5merqcj6aetsft99wu";
    const SCRIPT_HASH: &str = "script1cda3khwqv60360rp5m7akt50m6ttapacs8rqhn5w342z7r35m37";

    fn hash_vector_key(key: &str) -> Hash<28> {
        let (_, x) = decode_bech32(key).unwrap();
        pallas_crypto::hash::Hasher::<224>::hash(&x)
    }

    #[test]
    fn roundtrip_bech32() {
        for vector in MAINNET_TEST_VECTORS {
            let original = vector.0;
            let addr = Address::from_bech32(original).unwrap();
            let ours = addr.to_bech32().unwrap();
            assert_eq!(original, ours);
        }
    }

    #[test]
    fn typeid_matches() {
        for vector in MAINNET_TEST_VECTORS {
            let original = vector.0;
            let addr = Address::from_bech32(original).unwrap();
            assert_eq!(addr.typeid(), vector.1);
        }
    }

    #[test]
    fn network_matches() {
        for vector in MAINNET_TEST_VECTORS {
            let original = vector.0;
            let addr = Address::from_bech32(original).unwrap();
            assert!(matches!(addr.network(), Some(Network::Mainnet)));
        }
    }

    #[test]
    fn payload_matches() {
        for vector in MAINNET_TEST_VECTORS {
            let original = vector.0;
            let addr = Address::from_bech32(original).unwrap();

            match addr {
                Address::Shelley(x) => {
                    match x.payment() {
                        ShelleyPaymentPart::PaymentKey(hash) => {
                            let expected = &hash_vector_key(PAYMENT_PUBLIC_KEY);
                            assert_eq!(hash, expected);
                        }
                        ShelleyPaymentPart::Script(hash) => {
                            let (_, expected) = &decode_bech32(SCRIPT_HASH).unwrap();
                            let expected = &Hash::<28>::from_str(&hex::encode(&expected)).unwrap();
                            assert_eq!(hash, expected);
                        }
                    };

                    match x.delegation() {
                        ShelleyDelegationPart::StakeKey(hash) => {
                            let expected = &hash_vector_key(STAKE_PUBLIC_KEY);
                            assert_eq!(hash, expected);
                        }
                        ShelleyDelegationPart::Script(hash) => {
                            let (_, expected) = &decode_bech32(SCRIPT_HASH).unwrap();
                            let expected = &Hash::<28>::from_str(&hex::encode(&expected)).unwrap();
                            assert_eq!(hash, expected);
                        }
                        ShelleyDelegationPart::Pointer(ptr) => {
                            assert_eq!(ptr.slot(), 2498243);
                            assert_eq!(ptr.tx_idx(), 27);
                            assert_eq!(ptr.cert_idx(), 3);
                        }
                        _ => (),
                    };
                }
                Address::Stake(x) => match x.payload() {
                    StakePayload::Stake(hash) => {
                        let expected = &hash_vector_key(STAKE_PUBLIC_KEY);
                        assert_eq!(hash, expected);
                    }
                    StakePayload::Script(hash) => {
                        let (_, expected) = &decode_bech32(SCRIPT_HASH).unwrap();
                        let expected = &Hash::<28>::from_str(&hex::encode(&expected)).unwrap();
                        assert_eq!(hash, expected);
                    }
                },
                Address::Byron(_) => (),
            };
        }
    }
}
