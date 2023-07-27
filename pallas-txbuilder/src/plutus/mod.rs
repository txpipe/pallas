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

#[cfg(test)]
mod tests {
    use super::*;
    use pallas_primitives::babbage::{BigInt, Constr, PlutusData};

    #[test]
    fn test_build_int() {
        let output = int(1);
        assert_eq!(output, PlutusData::BigInt(BigInt::Int(1.into())));
    }

    #[test]
    fn test_build_uint() {
        let output = uint(1u64);
        assert_eq!(
            output,
            PlutusData::BigInt(BigInt::BigUInt(vec![0, 0, 0, 0, 0, 0, 0, 1].into()))
        );
    }

    #[test]
    fn test_build_nuint() {
        let output = nint(1u64);
        assert_eq!(
            output,
            PlutusData::BigInt(BigInt::BigNInt(vec![0, 0, 0, 0, 0, 0, 0, 1].into()))
        );
    }

    #[test]
    fn test_build_array() {
        let output: PlutusData = array().item(int(1)).item(int(2)).item(int(3)).into();
        assert_eq!(
            output,
            PlutusData::Array(vec![
                PlutusData::BigInt(BigInt::Int(1.into())),
                PlutusData::BigInt(BigInt::Int(2.into())),
                PlutusData::BigInt(BigInt::Int(3.into()))
            ])
        )
    }

    #[test]
    fn test_build_map() {
        let output: PlutusData = map().item(int(1), int(2)).item(int(2), int(3)).into();

        assert_eq!(
            output,
            PlutusData::Map(
                vec![
                    (
                        PlutusData::BigInt(BigInt::Int(1.into())),
                        PlutusData::BigInt(BigInt::Int(2.into()))
                    ),
                    (
                        PlutusData::BigInt(BigInt::Int(2.into())),
                        PlutusData::BigInt(BigInt::Int(3.into()))
                    )
                ]
                .into()
            )
        )
    }

    #[test]
    fn test_build_any_constr() {
        let output: PlutusData = any_constr(1).field(int(1)).field(int(2)).into();

        assert_eq!(
            output,
            PlutusData::Constr(Constr {
                tag: 1,
                any_constructor: None,
                fields: vec![
                    PlutusData::BigInt(BigInt::Int(1.into())),
                    PlutusData::BigInt(BigInt::Int(2.into())),
                ]
            })
        )
    }

    #[test]
    fn test_build_constr() {
        let output: PlutusData = constr(1, 2).field(int(1)).field(int(2)).into();

        assert_eq!(
            output,
            PlutusData::Constr(Constr {
                tag: 1,
                any_constructor: Some(2),
                fields: vec![
                    PlutusData::BigInt(BigInt::Int(1.into())),
                    PlutusData::BigInt(BigInt::Int(2.into())),
                ]
            })
        )
    }

    #[test]
    fn test_build_complex() {
        let output: PlutusData = map()
            .item(
                constr(1, 2).field(int(1)).field(int(2)),
                array().item(int(5)).item(int(6)),
            )
            .into();

        assert_eq!(
            output,
            PlutusData::Map(
                vec![(
                    PlutusData::Constr(Constr {
                        tag: 1,
                        any_constructor: Some(2),
                        fields: vec![
                            PlutusData::BigInt(BigInt::Int(1.into())),
                            PlutusData::BigInt(BigInt::Int(2.into())),
                        ]
                    }),
                    PlutusData::Array(vec![
                        PlutusData::BigInt(BigInt::Int(5.into())),
                        PlutusData::BigInt(BigInt::Int(6.into())),
                    ])
                ),]
                .into()
            )
        )
    }
}
