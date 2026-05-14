//! Encode and decode Cardano addresses of every kind.
//!
//! Byron, Shelley payment, and stake addresses are all parsed through the same
//! [`Address`] entry point: feed it a bech32 / base58 / hex string (or raw
//! bytes), match on the variant, and inspect the network, payment credential,
//! delegation, or pointer parts. Address shape follows
//! [CIP-19](https://cips.cardano.org/cips/cip19/).
//!
//! # Usage
//!
//! ```
//! use pallas_addresses::Address;
//!
//! let addr = Address::from_bech32(
//!     "addr1qx2fxv2umyhttkxyxp8x0dlpdt3k6cwng5pxj3jhsydzer3\
//!      n0d3vllmyqwsx5wktcd8cc3sq835lu7drv2xwl2wywfgse35a3x",
//! )?;
//!
//! match addr {
//!     Address::Byron(b)   => println!("byron:   {}", b.to_base58()),
//!     Address::Shelley(s) => println!("shelley: {} on {:?}", s.to_bech32()?, s.network()),
//!     Address::Stake(s)   => println!("stake:   {}", s.to_bech32()?),
//! }
//! # Ok::<_, pallas_addresses::Error>(())
//! ```
//!
//! # Overview
//!
//! - [`Address`] — top-level decoded form, dispatching to the three variants.
//! - [`ByronAddress`], [`ShelleyAddress`], [`StakeAddress`] — per-kind
//!   decoded representations.
//! - [`ShelleyPaymentPart`], [`ShelleyDelegationPart`], [`StakePayload`],
//!   [`Pointer`] — the structural pieces that make up a Shelley / stake
//!   address.
//! - [`Network`] — Mainnet / Testnet / `Other(u8)` discriminator parsed from
//!   the address header.
//! - [`byron`] submodule — Byron-specific structures and CBOR helpers.
//! - [`varuint`] submodule — variable-length integer codec used by stake
//!   pointers.
//!
//! # Usage as part of `pallas`
//!
//! When depending on the umbrella [`pallas`] crate, this crate is re-exported
//! as `pallas::ledger::addresses`.
//!
//! [`pallas`]: https://crates.io/crates/pallas

/// Byron-era addresses (base58, CBOR-encoded).
pub mod byron;
/// Variable-length unsigned integer codec used inside Shelley pointer addresses.
pub mod varuint;

use std::{fmt::Display, io::Cursor, str::FromStr};

use pallas_crypto::hash::Hash;
use thiserror::Error;

/// Errors produced while decoding or encoding a Cardano address.
#[derive(Error, Debug)]
pub enum Error {
    /// Failed to decode a bech32 string.
    #[error("error decoding from bech32 {0}")]
    BadBech32(bech32::DecodeError),

    /// Failed to decode a base58 string.
    #[error("error decoding base58 value")]
    BadBase58(base58::FromBase58Error),

    /// Failed to decode a hexadecimal string.
    #[error("error decoding hex value")]
    BadHex,

    /// The input string does not match any recognized address format.
    #[error("unknown or bad string format for address {0}")]
    UnknownStringFormat(String),

    /// The address byte sequence is empty and has no header byte.
    #[error("address header not found")]
    MissingHeader,

    /// The address header byte does not match any defined Cardano address type.
    #[error("address header is invalid {0:08b}")]
    InvalidHeader(u8),

    /// The requested operation does not apply to Byron addresses.
    #[error("invalid operation for Byron address")]
    InvalidForByron,

    /// The requested operation does not apply to this address content.
    #[error("invalid operation for address content")]
    InvalidForContent,

    /// The Byron address payload is not valid CBOR.
    #[error("invalid CBOR for Byron address {0}")]
    InvalidByronCbor(pallas_codec::minicbor::decode::Error),

    /// The network nibble in the header does not map to a known bech32 HRP.
    #[error("unkown hrp for network {0:08b}")]
    UnknownNetworkHrp(u8),

    /// An embedded hash had an unexpected byte length.
    #[error("invalid hash size {0}")]
    InvalidHashSize(usize),

    /// The total address byte length is shorter than required by its type.
    #[error("invalid address length {0}")]
    InvalidAddressLength(usize),

    /// The pointer payload of a Shelley pointer address could not be parsed.
    #[error("invalid pointer data")]
    InvalidPointerData,

    /// A variable-length uint inside a pointer address failed to decode.
    #[error("variable-length uint error: {0}")]
    VarUintError(varuint::Error),
}

/// Hash of a payment verification key (Blake2b-224).
pub type PaymentKeyHash = Hash<28>;
/// Hash of a stake verification key (Blake2b-224).
pub type StakeKeyHash = Hash<28>;
/// Hash of a script (Blake2b-224).
pub type ScriptHash = Hash<28>;

/// Absolute slot number on the Cardano chain.
pub type Slot = u64;
/// Index of a transaction within a block.
pub type TxIdx = u64;
/// Index of a certificate within a transaction.
pub type CertIdx = u64;

/// An on-chain pointer to a stake key
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
    /// Build a pointer from its three components.
    pub fn new(slot: Slot, tx_idx: TxIdx, cert_idx: CertIdx) -> Self {
        Pointer(slot, tx_idx, cert_idx)
    }

    /// Parse a pointer from its variable-length byte encoding.
    pub fn parse(bytes: &[u8]) -> Result<Self, Error> {
        let mut cursor = Cursor::new(bytes);
        let a = varuint::read(&mut cursor).map_err(Error::VarUintError)?;
        let b = varuint::read(&mut cursor).map_err(Error::VarUintError)?;
        let c = varuint::read(&mut cursor).map_err(Error::VarUintError)?;

        Ok(Pointer(a, b, c))
    }

    /// Encode the pointer as its variable-length byte representation.
    pub fn to_vec(&self) -> Vec<u8> {
        let mut cursor = Cursor::new(vec![]);
        varuint::write(&mut cursor, self.0);
        varuint::write(&mut cursor, self.1);
        varuint::write(&mut cursor, self.2);

        cursor.into_inner()
    }

    /// Slot number component of the pointer.
    pub fn slot(&self) -> u64 {
        self.0
    }

    /// Transaction index component of the pointer.
    pub fn tx_idx(&self) -> u64 {
        self.1
    }

    /// Certificate index component of the pointer.
    pub fn cert_idx(&self) -> u64 {
        self.2
    }
}

/// The payment part of a Shelley address.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash)]
pub enum ShelleyPaymentPart {
    /// Payment is controlled by a verification key with the given hash.
    Key(PaymentKeyHash),
    /// Payment is controlled by a script with the given hash.
    Script(ScriptHash),
}

impl ShelleyPaymentPart {
    /// Build a key-hash payment part.
    pub fn key_hash(hash: Hash<28>) -> Self {
        Self::Key(hash)
    }

    /// Build a script-hash payment part.
    pub fn script_hash(hash: Hash<28>) -> Self {
        Self::Script(hash)
    }

    /// Get a reference to the inner hash of this address part.
    pub fn as_hash(&self) -> &Hash<28> {
        match self {
            Self::Key(x) => x,
            Self::Script(x) => x,
        }
    }

    /// Encodes this address part as its 28 raw bytes.
    pub fn to_vec(&self) -> Vec<u8> {
        match self {
            Self::Key(x) => x.to_vec(),
            Self::Script(x) => x.to_vec(),
        }
    }

    /// Encode the payment part as a lowercase hexadecimal string.
    pub fn to_hex(&self) -> String {
        let bytes = self.to_vec();
        hex::encode(bytes)
    }

    /// Encode the payment part as a bech32 string under its CIP-5 HRP.
    pub fn to_bech32(&self) -> String {
        let hrp = match self {
            Self::Key(_) => "addr_vkh",
            Self::Script(_) => "addr_shared_vkh",
        };
        let bytes = self.to_vec();
        encode_bech32(&bytes, hrp)
    }

    /// Indicates if this is the hash of a script.
    pub fn is_script(&self) -> bool {
        matches!(self, Self::Script(_))
    }
}

/// The delegation part of a Shelley address.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash)]
pub enum ShelleyDelegationPart {
    /// Delegation is controlled by a stake key with the given hash.
    Key(StakeKeyHash),
    /// Delegation is controlled by a script with the given hash.
    Script(ScriptHash),
    /// Delegation is identified by a pointer to a stake-key registration.
    Pointer(Pointer),
    /// Address has no delegation part (enterprise address).
    Null,
}

impl ShelleyDelegationPart {
    /// Build a key-hash delegation part.
    pub fn key_hash(hash: Hash<28>) -> Self {
        Self::Key(hash)
    }

    /// Build a script-hash delegation part.
    pub fn script_hash(hash: Hash<28>) -> Self {
        Self::Script(hash)
    }

    /// Build a pointer delegation part by parsing its variable-length encoding.
    pub fn from_pointer(bytes: &[u8]) -> Result<Self, Error> {
        let pointer = Pointer::parse(bytes)?;
        Ok(Self::Pointer(pointer))
    }

    /// Get a reference to the inner hash of this address part, if it carries one.
    pub fn as_hash(&self) -> Option<&Hash<28>> {
        match self {
            Self::Key(x) => Some(x),
            Self::Script(x) => Some(x),
            Self::Pointer(_) => None,
            Self::Null => None,
        }
    }

    /// Encode the delegation part as its raw byte representation.
    pub fn to_vec(&self) -> Vec<u8> {
        match self {
            Self::Key(x) => x.to_vec(),
            Self::Script(x) => x.to_vec(),
            Self::Pointer(x) => x.to_vec(),
            Self::Null => vec![],
        }
    }

    /// Encode the delegation part as a lowercase hexadecimal string.
    pub fn to_hex(&self) -> String {
        let bytes = self.to_vec();
        hex::encode(bytes)
    }

    /// Encode the delegation part as a bech32 string under its CIP-5 HRP.
    pub fn to_bech32(&self) -> Result<String, Error> {
        let hrp = match self {
            Self::Key(_) => "stake_vkh",
            Self::Script(_) => "stake_shared_vkh",
            _ => return Err(Error::InvalidForContent),
        };

        let bytes = self.to_vec();
        Ok(encode_bech32(&bytes, hrp))
    }

    /// Indicates if this is the hash of a script.
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

    /// Indicates if this is the hash of a script.
    pub fn is_script(&self) -> bool {
        matches!(self, StakePayload::Script(_))
    }

    /// Get a reference to the inner hash of this address part.
    pub fn as_hash(&self) -> &Hash<28> {
        match self {
            StakePayload::Stake(x) => x,
            StakePayload::Script(x) => x,
        }
    }
}

/// The network tag of an address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
pub enum Network {
    /// The Cardano testnet (network id 0).
    Testnet,
    /// The Cardano mainnet (network id 1).
    Mainnet,
    /// Any other network id (custom or unrecognized).
    Other(u8),
}

impl From<u8> for Network {
    fn from(id: u8) -> Self {
        match id {
            0 => Network::Testnet,
            1 => Network::Mainnet,
            x => Network::Other(x),
        }
    }
}

/// A decoded Shelley address.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash)]
pub struct ShelleyAddress(Network, ShelleyPaymentPart, ShelleyDelegationPart);

/// The payload of a stake address.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash)]
pub enum StakePayload {
    /// Stake credential controlled by a verification-key hash.
    Stake(StakeKeyHash),
    /// Stake credential controlled by a script hash.
    Script(ScriptHash),
}

/// A decoded stake address.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash)]
pub struct StakeAddress(Network, StakePayload);

pub use byron::ByronAddress;

/// A decoded Cardano address of any type.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash)]
pub enum Address {
    /// A Byron-era base58/CBOR address.
    Byron(ByronAddress),
    /// A Shelley-era payment address.
    Shelley(ShelleyAddress),
    /// A stake address.
    Stake(StakeAddress),
}

fn encode_bech32(addr: &[u8], hrp: &str) -> String {
    let hrp = bech32::Hrp::parse(hrp).expect("hardcoded address hrp is valid");
    bech32::encode::<bech32::Bech32>(hrp, addr).expect("address fits within bech32 limits")
}

fn decode_bech32(bech32: &str) -> Result<(String, Vec<u8>), Error> {
    let (hrp, addr) = bech32::decode(bech32).map_err(Error::BadBech32)?;
    Ok((hrp.to_string(), addr))
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
            if payload.len() < 29 {
                return Err(Error::InvalidAddressLength(payload.len()));
            }

            let net = parse_network(header);
            let h1 = slice_to_hash(&payload[0..=27])?;
            let p1 = ShelleyPaymentPart::$payment(h1);
            let p2 = ShelleyDelegationPart::from_pointer(&payload[28..])?;
            let addr = ShelleyAddress(net, p1, p2);

            Ok(addr.into())
        }
    };
    ($name:tt, $payment:tt, $delegation:tt) => {
        fn $name(header: u8, payload: &[u8]) -> Result<Address, Error> {
            if payload.len() < 56 {
                return Err(Error::InvalidAddressLength(payload.len()));
            }

            let net = parse_network(header);
            let h1 = slice_to_hash(&payload[0..=27])?;
            let p1 = ShelleyPaymentPart::$payment(h1);
            let h2 = slice_to_hash(&payload[28..=55])?;
            let p2 = ShelleyDelegationPart::$delegation(h2);
            let addr = ShelleyAddress(net, p1, p2);

            Ok(addr.into())
        }
    };
    ($name:tt, $payment:tt) => {
        fn $name(header: u8, payload: &[u8]) -> Result<Address, Error> {
            if payload.len() < 28 {
                return Err(Error::InvalidAddressLength(payload.len()));
            }

            let net = parse_network(header);
            let h1 = slice_to_hash(&payload[0..=27])?;
            let p1 = ShelleyPaymentPart::$payment(h1);
            let addr = ShelleyAddress(net, p1, ShelleyDelegationPart::Null);

            Ok(addr.into())
        }
    };
}

macro_rules! parse_stake_fn {
    ($name:tt, $type:tt) => {
        fn $name(header: u8, payload: &[u8]) -> Result<Address, Error> {
            if payload.len() < 28 {
                return Err(Error::InvalidAddressLength(payload.len()));
            }

            let net = parse_network(header);
            let p1 = StakePayload::$type(&payload[0..=27])?;
            let addr = StakeAddress(net, p1);

            Ok(addr.into())
        }
    };
}

// types 0-7 are Shelley addresses
parse_shelley_fn!(parse_type_0, key_hash, key_hash);
parse_shelley_fn!(parse_type_1, script_hash, key_hash);
parse_shelley_fn!(parse_type_2, key_hash, script_hash);
parse_shelley_fn!(parse_type_3, script_hash, script_hash);
parse_shelley_fn!(parse_type_4, key_hash, pointer);
parse_shelley_fn!(parse_type_5, script_hash, pointer);
parse_shelley_fn!(parse_type_6, key_hash);
parse_shelley_fn!(parse_type_7, script_hash);

// type 8 (1000) are Byron addresses
fn parse_type_8(header: u8, payload: &[u8]) -> Result<Address, Error> {
    let vec = [&[header], payload].concat();
    let inner = pallas_codec::minicbor::decode(&vec).map_err(Error::InvalidByronCbor)?;
    Ok(Address::Byron(inner))
}

// types 14-15 are Stake addresses
parse_stake_fn!(parse_type_14, stake_key);
parse_stake_fn!(parse_type_15, script);

fn bytes_to_address(bytes: &[u8]) -> Result<Address, Error> {
    let header = *bytes.first().ok_or(Error::MissingHeader)?;
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

impl Network {
    /// True if this is the Cardano mainnet.
    pub fn is_mainnet(&self) -> bool {
        matches!(self, Network::Mainnet)
    }

    /// Numeric network id encoded in the address header.
    pub fn value(&self) -> u8 {
        match self {
            Network::Testnet => 0,
            Network::Mainnet => 1,
            Network::Other(x) => *x,
        }
    }
}

impl ShelleyAddress {
    /// Build a Shelley address from its network, payment, and delegation parts.
    pub fn new(
        network: Network,
        payment: ShelleyPaymentPart,
        delegation: ShelleyDelegationPart,
    ) -> Self {
        Self(network, payment, delegation)
    }

    /// Gets the network associated with this address.
    pub fn network(&self) -> Network {
        self.0
    }

    /// Gets a numeric id describing the type of the address.
    pub fn typeid(&self) -> u8 {
        match (&self.1, &self.2) {
            (ShelleyPaymentPart::Key(_), ShelleyDelegationPart::Key(_)) => 0b0000,
            (ShelleyPaymentPart::Script(_), ShelleyDelegationPart::Key(_)) => 0b0001,
            (ShelleyPaymentPart::Key(_), ShelleyDelegationPart::Script(_)) => 0b0010,
            (ShelleyPaymentPart::Script(_), ShelleyDelegationPart::Script(_)) => 0b0011,
            (ShelleyPaymentPart::Key(_), ShelleyDelegationPart::Pointer(_)) => 0b0100,
            (ShelleyPaymentPart::Script(_), ShelleyDelegationPart::Pointer(_)) => 0b0101,
            (ShelleyPaymentPart::Key(_), ShelleyDelegationPart::Null) => 0b0110,
            (ShelleyPaymentPart::Script(_), ShelleyDelegationPart::Null) => 0b0111,
        }
    }

    /// Combine the type id and network nibbles into the address header byte.
    pub fn to_header(&self) -> u8 {
        let type_id = self.typeid();
        let type_id = type_id << 4;
        let network = self.0.value();

        type_id | network
    }

    /// Get a reference to the payment part of this address.
    pub fn payment(&self) -> &ShelleyPaymentPart {
        &self.1
    }

    /// Get a reference to the delegation part of this address.
    pub fn delegation(&self) -> &ShelleyDelegationPart {
        &self.2
    }

    /// Gets the bech32 human-readable-part for this address.
    pub fn hrp(&self) -> Result<&'static str, Error> {
        match &self.0 {
            Network::Testnet => Ok("addr_test"),
            Network::Mainnet => Ok("addr"),
            Network::Other(x) => Err(Error::UnknownNetworkHrp(*x)),
        }
    }

    /// Encode the address as its raw byte representation (header + payload).
    pub fn to_vec(&self) -> Vec<u8> {
        let header = self.to_header();
        let payment = self.1.to_vec();
        let delegation = self.2.to_vec();

        [&[header], payment.as_slice(), delegation.as_slice()].concat()
    }

    /// Encode the address as a lowercase hexadecimal string.
    pub fn to_hex(&self) -> String {
        let bytes = self.to_vec();
        hex::encode(bytes)
    }

    /// Encode the address as a bech32 string under its network HRP.
    pub fn to_bech32(&self) -> Result<String, Error> {
        let hrp = self.hrp()?;
        let bytes = self.to_vec();
        Ok(encode_bech32(&bytes, hrp))
    }

    /// Indicates if either the payment or delegation part is a script.
    pub fn has_script(&self) -> bool {
        self.payment().is_script() || self.delegation().is_script()
    }
}

impl TryFrom<ShelleyAddress> for StakeAddress {
    type Error = Error;

    fn try_from(value: ShelleyAddress) -> Result<Self, Self::Error> {
        let payload = match value.delegation() {
            ShelleyDelegationPart::Key(h) => StakePayload::Stake(*h),
            ShelleyDelegationPart::Script(h) => StakePayload::Script(*h),
            _ => return Err(Error::InvalidForContent),
        };

        Ok(StakeAddress(value.network(), payload))
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
    /// Build a stake address from its network and payload.
    pub fn new(network: Network, payload: StakePayload) -> Self {
        Self(network, payload)
    }

    /// Gets the network associated with this address.
    pub fn network(&self) -> Network {
        self.0
    }

    /// Gets a numeric id describing the type of the address.
    pub fn typeid(&self) -> u8 {
        match &self.1 {
            StakePayload::Stake(_) => 0b1110,
            StakePayload::Script(_) => 0b1111,
        }
    }

    /// Builds the header byte for this address.
    pub fn to_header(&self) -> u8 {
        let type_id = self.typeid();
        let type_id = type_id << 4;
        let network = self.0.value();

        type_id | network
    }

    /// Gets the payload of this address.
    pub fn payload(&self) -> &StakePayload {
        &self.1
    }

    /// Gets the bech32 human-readable-part for this address.
    pub fn hrp(&self) -> Result<&'static str, Error> {
        match &self.0 {
            Network::Testnet => Ok("stake_test"),
            Network::Mainnet => Ok("stake"),
            Network::Other(x) => Err(Error::UnknownNetworkHrp(*x)),
        }
    }

    /// Encode the address as its raw byte representation (header + payload).
    pub fn to_vec(&self) -> Vec<u8> {
        let header = self.to_header();

        [&[header], self.1.as_ref()].concat()
    }

    /// Encode the address as a lowercase hexadecimal string.
    pub fn to_hex(&self) -> String {
        let bytes = self.to_vec();
        hex::encode(bytes)
    }

    /// Encode the address as a bech32 string under its network HRP.
    pub fn to_bech32(&self) -> Result<String, Error> {
        let hrp = self.hrp()?;
        let bytes = self.to_vec();
        Ok(encode_bech32(&bytes, hrp))
    }

    /// Indicates if this stake address is controlled by a script.
    pub fn is_script(&self) -> bool {
        self.payload().is_script()
    }
}

impl Address {
    /// Tries to encode an Address into a bech32 string
    pub fn to_bech32(&self) -> Result<String, Error> {
        match self {
            Address::Byron(_) => Err(Error::InvalidForByron),
            Address::Shelley(x) => x.to_bech32(),
            Address::Stake(x) => x.to_bech32(),
        }
    }

    /// Tries to parse a bech32 value into an Address
    pub fn from_bech32(bech32: &str) -> Result<Self, Error> {
        bech32_to_address(bech32)
    }

    /// Tries to decode the raw bytes of an address.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        bytes_to_address(bytes)
    }

    /// Tries to parse a hex value into an Address.
    pub fn from_hex(bytes: &str) -> Result<Self, Error> {
        let bytes = hex::decode(bytes).map_err(|_| Error::BadHex)?;
        bytes_to_address(&bytes)
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

    /// Encode the address as its raw byte representation.
    pub fn to_vec(&self) -> Vec<u8> {
        match self {
            Address::Byron(x) => x.to_vec(),
            Address::Shelley(x) => x.to_vec(),
            Address::Stake(x) => x.to_vec(),
        }
    }

    /// Encode the address as a lowercase hexadecimal string.
    pub fn to_hex(&self) -> String {
        match self {
            Address::Byron(x) => x.to_hex(),
            Address::Shelley(x) => x.to_hex(),
            Address::Stake(x) => x.to_hex(),
        }
    }
}

impl Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Address::Byron(x) => f.write_str(&x.to_base58()),
            Address::Shelley(x) => f.write_str(&x.to_bech32().unwrap_or_else(|_| x.to_hex())),
            Address::Stake(x) => f.write_str(&x.to_bech32().unwrap_or_else(|_| x.to_hex())),
        }
    }
}

impl FromStr for Address {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(x) = Address::from_bech32(s) {
            return Ok(x);
        }

        if let Ok(x) = ByronAddress::from_base58(s) {
            return Ok(x.into());
        }

        if let Ok(x) = Address::from_hex(s) {
            return Ok(x);
        }

        Err(Error::UnknownStringFormat(s.to_owned()))
    }
}

impl TryFrom<&[u8]> for Address {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        bytes_to_address(value)
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
        (
            "addr1qx2fxv2umyhttkxyxp8x0dlpdt3k6cwng5pxj3jhsydzer3n0d3vllmyqwsx5wktcd8cc3sq835lu7drv2xwl2wywfgse35a3x",
            0u8,
        ),
        (
            "addr1z8phkx6acpnf78fuvxn0mkew3l0fd058hzquvz7w36x4gten0d3vllmyqwsx5wktcd8cc3sq835lu7drv2xwl2wywfgs9yc0hh",
            1u8,
        ),
        (
            "addr1yx2fxv2umyhttkxyxp8x0dlpdt3k6cwng5pxj3jhsydzerkr0vd4msrxnuwnccdxlhdjar77j6lg0wypcc9uar5d2shs2z78ve",
            2u8,
        ),
        (
            "addr1x8phkx6acpnf78fuvxn0mkew3l0fd058hzquvz7w36x4gt7r0vd4msrxnuwnccdxlhdjar77j6lg0wypcc9uar5d2shskhj42g",
            3u8,
        ),
        (
            "addr1gx2fxv2umyhttkxyxp8x0dlpdt3k6cwng5pxj3jhsydzer5pnz75xxcrzqf96k",
            4u8,
        ),
        (
            "addr128phkx6acpnf78fuvxn0mkew3l0fd058hzquvz7w36x4gtupnz75xxcrtw79hu",
            5u8,
        ),
        (
            "addr1vx2fxv2umyhttkxyxp8x0dlpdt3k6cwng5pxj3jhsydzers66hrl8",
            6u8,
        ),
        (
            "addr1w8phkx6acpnf78fuvxn0mkew3l0fd058hzquvz7w36x4gtcyjy7wx",
            7u8,
        ),
        (
            "stake1uyehkck0lajq8gr28t9uxnuvgcqrc6070x3k9r8048z8y5gh6ffgw",
            14u8,
        ),
        (
            "stake178phkx6acpnf78fuvxn0mkew3l0fd058hzquvz7w36x4gtcccycj5",
            15u8,
        ),
        (
            "37btjrVyb4KDXBNC4haBVPCrro8AQPHwvCMp3RFhhSVWwfFmZ6wwzSK6JK1hY6wHNmtrpTf1kdbva8TCneM2YsiXT7mrzT21EacHnPpz5YyUdj64na",
            8u8,
        ),
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
            match Address::from_str(original) {
                Ok(Address::Byron(_)) => (),
                Ok(addr) => {
                    let ours = addr.to_bech32().unwrap();
                    assert_eq!(original, ours);
                }
                _ => panic!("should be able to decode from bech32"),
            }
        }
    }

    #[test]
    fn roundtrip_string() {
        for vector in MAINNET_TEST_VECTORS {
            let original = vector.0;
            let addr = Address::from_str(original).unwrap();
            let ours = addr.to_string();
            assert_eq!(original, ours);
        }
    }

    #[test]
    fn typeid_matches() {
        for vector in MAINNET_TEST_VECTORS {
            let original = vector.0;
            let addr = Address::from_str(original).unwrap();
            assert_eq!(addr.typeid(), vector.1);
        }
    }

    #[test]
    fn network_matches() {
        for vector in MAINNET_TEST_VECTORS {
            let original = vector.0;
            let addr = Address::from_str(original).unwrap();

            match addr {
                Address::Byron(_) => assert!(addr.network().is_none()),
                _ => assert!(matches!(addr.network(), Some(Network::Mainnet))),
            }
        }
    }

    #[test]
    fn payload_matches() {
        for vector in MAINNET_TEST_VECTORS {
            let original = vector.0;
            let addr = Address::from_str(original).unwrap();

            match addr {
                Address::Shelley(x) => {
                    match x.payment() {
                        ShelleyPaymentPart::Key(hash) => {
                            let expected = &hash_vector_key(PAYMENT_PUBLIC_KEY);
                            assert_eq!(hash, expected);
                        }
                        ShelleyPaymentPart::Script(hash) => {
                            let (_, expected) = &decode_bech32(SCRIPT_HASH).unwrap();
                            let expected = &Hash::<28>::from_str(&hex::encode(expected)).unwrap();
                            assert_eq!(hash, expected);
                        }
                    };

                    match x.delegation() {
                        ShelleyDelegationPart::Key(hash) => {
                            let expected = &hash_vector_key(STAKE_PUBLIC_KEY);
                            assert_eq!(hash, expected);
                        }
                        ShelleyDelegationPart::Script(hash) => {
                            let (_, expected) = &decode_bech32(SCRIPT_HASH).unwrap();
                            let expected = &Hash::<28>::from_str(&hex::encode(expected)).unwrap();
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
                        let expected = &Hash::<28>::from_str(&hex::encode(expected)).unwrap();
                        assert_eq!(hash, expected);
                    }
                },
                Address::Byron(_) => {
                    // byron has it's own payload tests
                }
            };
        }
    }

    #[test]
    fn construct_from_parts() {
        let payment_hash = hash_vector_key(PAYMENT_PUBLIC_KEY);
        let delegation_hash = hash_vector_key(STAKE_PUBLIC_KEY);

        let addr: Address = ShelleyAddress::new(
            Network::Mainnet,
            ShelleyPaymentPart::key_hash(payment_hash),
            ShelleyDelegationPart::key_hash(delegation_hash),
        )
        .into();

        assert_eq!(addr.to_bech32().unwrap(), MAINNET_TEST_VECTORS[0].0);
    }

    #[test]
    fn test_minted_invalid_pointed_address() {
        let addr = Address::from_hex(
            "40C19D7D05E90EEB6394B53313FE79D47077DE33068C6B813BBE5C9D5681FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF7F81FFFFFFFFFFFFFFFF7F81FFFFFFFFFFFFFFFF7F",
        );
        assert!(matches!(addr, Ok(Address::Shelley(_))));
    }

    #[test]
    fn test_shelley_into_stake() {
        let addr = Address::from_bech32("addr1qx2fxv2umyhttkxyxp8x0dlpdt3k6cwng5pxj3jhsydzer3n0d3vllmyqwsx5wktcd8cc3sq835lu7drv2xwl2wywfgse35a3x").unwrap();

        match addr {
            Address::Shelley(shelley_addr) => {
                let stake_addr: StakeAddress = shelley_addr.clone().try_into().unwrap();
                assert_eq!(stake_addr.network(), shelley_addr.network());

                let stake_hash = stake_addr.payload().as_hash();
                let shelley_hash = shelley_addr.delegation().as_hash().unwrap();
                assert_eq!(stake_hash, shelley_hash);
            }
            _ => panic!(),
        }
    }

    #[test]
    fn test_minted_extra_bytes_base_address() {
        let addr = Address::from_hex(
            "015bad085057ac10ecc7060f7ac41edd6f63068d8963ef7d86ca58669e5ecf2d283418a60be5a848a2380eb721000da1e0bbf39733134beca4cb57afb0b35fc89c63061c9914e055001a518c7516",
        );
        assert!(matches!(addr, Ok(Address::Shelley(_))));
    }
}
