use pallas_codec::utils::Int;
use pallas_primitives::babbage::{BigInt, PlutusData};

mod array;
mod constr;
mod map;

pub use array::*;
pub use constr::*;
pub use map::*;

pub fn int(v: impl Into<Int>) -> PlutusData {
    let as_int = BigInt::Int(v.into());
    PlutusData::BigInt(as_int)
}

pub fn uint(v: impl Into<u64>) -> PlutusData {
    let bytes_owned: Vec<u8> = v.into().to_be_bytes().to_vec();
    PlutusData::BigInt(BigInt::BigUInt(bytes_owned.into()))
}

pub fn nint(v: impl Into<u64>) -> PlutusData {
    let bytes_owned: Vec<u8> = v.into().to_be_bytes().to_vec();
    PlutusData::BigInt(BigInt::BigNInt(bytes_owned.into()))
}
