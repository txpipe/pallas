use crate::KeyValuePairs;
use pallas_codec::utils::Int;
use pallas_codec::{
    minicbor::{
        self,
        data::{IanaTag, Tag},
        Encode,
    },
    utils::MaybeIndefArray,
};
use serde::{Deserialize, Serialize};
use std::{fmt, ops::Deref};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum PlutusData {
    Constr(Constr<PlutusData>),
    Map(KeyValuePairs<PlutusData, PlutusData>),
    BigInt(BigInt),
    BoundedBytes(BoundedBytes),
    Array(MaybeIndefArray<PlutusData>),
}

impl<'b, C> minicbor::decode::Decode<'b, C> for PlutusData {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let type_ = d.datatype()?;

        match type_ {
            minicbor::data::Type::Tag => {
                let mut probe = d.probe();
                let tag = probe.tag()?;

                if tag == IanaTag::PosBignum.tag() || tag == IanaTag::NegBignum.tag() {
                    Ok(Self::BigInt(d.decode_with(ctx)?))
                } else {
                    match tag.as_u64() {
                        (121..=127) | (1280..=1400) | 102 => Ok(Self::Constr(d.decode_with(ctx)?)),
                        _ => Err(minicbor::decode::Error::message(
                            "unknown tag for plutus data tag",
                        )),
                    }
                }
            }
            minicbor::data::Type::U8
            | minicbor::data::Type::U16
            | minicbor::data::Type::U32
            | minicbor::data::Type::U64
            | minicbor::data::Type::I8
            | minicbor::data::Type::I16
            | minicbor::data::Type::I32
            | minicbor::data::Type::I64
            | minicbor::data::Type::Int => Ok(Self::BigInt(d.decode_with(ctx)?)),
            minicbor::data::Type::Map | minicbor::data::Type::MapIndef => {
                Ok(Self::Map(d.decode_with(ctx)?))
            }
            minicbor::data::Type::Bytes => Ok(Self::BoundedBytes(d.decode_with(ctx)?)),
            minicbor::data::Type::BytesIndef => {
                let mut full = Vec::new();

                for slice in d.bytes_iter()? {
                    full.extend(slice?);
                }

                Ok(Self::BoundedBytes(BoundedBytes::from(full)))
            }
            minicbor::data::Type::Array | minicbor::data::Type::ArrayIndef => {
                Ok(Self::Array(d.decode_with(ctx)?))
            }

            any => Err(minicbor::decode::Error::message(format!(
                "bad cbor data type ({any:?}) for plutus data"
            ))),
        }
    }
}

impl<C> minicbor::encode::Encode<C> for PlutusData {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            Self::Constr(a) => {
                e.encode_with(a, ctx)?;
            }
            Self::Map(a) => {
                e.encode_with(a, ctx)?;
            }
            Self::BigInt(a) => {
                e.encode_with(a, ctx)?;
            }
            Self::BoundedBytes(a) => {
                e.encode_with(a, ctx)?;
            }
            Self::Array(a) => {
                e.encode_with(a, ctx)?;
            }
        };

        Ok(())
    }
}

/*
big_int = int / big_uint / big_nint ; New
big_uint = #6.2(bounded_bytes) ; New
big_nint = #6.3(bounded_bytes) ; New
 */

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum BigInt {
    Int(Int),
    BigUInt(BoundedBytes),
    BigNInt(BoundedBytes),
}

impl<'b, C> minicbor::decode::Decode<'b, C> for BigInt {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let datatype = d.datatype()?;

        match datatype {
            minicbor::data::Type::U8
            | minicbor::data::Type::U16
            | minicbor::data::Type::U32
            | minicbor::data::Type::U64
            | minicbor::data::Type::I8
            | minicbor::data::Type::I16
            | minicbor::data::Type::I32
            | minicbor::data::Type::I64
            | minicbor::data::Type::Int => Ok(Self::Int(d.decode_with(ctx)?)),
            minicbor::data::Type::Tag => {
                let tag = d.tag()?;
                if tag == IanaTag::PosBignum.tag() {
                    Ok(Self::BigUInt(d.decode_with(ctx)?))
                } else if tag == IanaTag::NegBignum.tag() {
                    Ok(Self::BigNInt(d.decode_with(ctx)?))
                } else {
                    Err(minicbor::decode::Error::message(
                        "invalid cbor tag for big int",
                    ))
                }
            }
            _ => Err(minicbor::decode::Error::message(
                "invalid cbor data type for big int",
            )),
        }
    }
}

impl<C> minicbor::encode::Encode<C> for BigInt {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            BigInt::Int(x) => {
                e.encode_with(x, ctx)?;
            }
            BigInt::BigUInt(x) => {
                e.tag(IanaTag::PosBignum)?;
                e.encode_with(x, ctx)?;
            }
            BigInt::BigNInt(x) => {
                e.tag(IanaTag::NegBignum)?;
                e.encode_with(x, ctx)?;
            }
        };

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Constr<A> {
    pub tag: u64,
    pub any_constructor: Option<u64>,
    pub fields: MaybeIndefArray<A>,
}

impl<'b, C, A> minicbor::decode::Decode<'b, C> for Constr<A>
where
    A: minicbor::decode::Decode<'b, C>,
{
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let tag = d.tag()?;
        let x = tag.as_u64();
        match x {
            121..=127 | 1280..=1400 => Ok(Constr {
                tag: x,
                fields: d.decode_with(ctx)?,
                any_constructor: None,
            }),
            102 => {
                d.array()?;

                Ok(Constr {
                    tag: x,
                    any_constructor: Some(d.decode_with(ctx)?),
                    fields: d.decode_with(ctx)?,
                })
            }
            _ => Err(minicbor::decode::Error::message(
                "bad tag code for plutus data",
            )),
        }
    }
}

impl<C, A> minicbor::encode::Encode<C> for Constr<A>
where
    A: minicbor::encode::Encode<C>,
{
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.tag(Tag::new(self.tag))?;

        match self.tag {
            102 => {
                let x = (self.any_constructor.unwrap_or_default(), &self.fields);
                e.encode_with(x, ctx)?;
                Ok(())
            }
            _ => {
                e.encode_with(&self.fields, ctx)?;
                Ok(())
            }
        }
    }
}

/// Defined to encode PlutusData bytestring as it is done in the canonical
/// plutus implementation
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(into = "String")]
#[serde(try_from = "String")]
pub struct BoundedBytes(Vec<u8>);

impl From<Vec<u8>> for BoundedBytes {
    fn from(xs: Vec<u8>) -> Self {
        BoundedBytes(xs)
    }
}

impl From<BoundedBytes> for Vec<u8> {
    fn from(b: BoundedBytes) -> Self {
        b.0
    }
}

impl Deref for BoundedBytes {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TryFrom<String> for BoundedBytes {
    type Error = hex::FromHexError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let v = hex::decode(value)?;
        Ok(BoundedBytes(v))
    }
}

impl From<BoundedBytes> for String {
    fn from(b: BoundedBytes) -> Self {
        hex::encode(b.deref())
    }
}

impl fmt::Display for BoundedBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bytes: Vec<u8> = self.clone().into();

        f.write_str(&hex::encode(bytes))
    }
}

impl<C> Encode<C> for BoundedBytes {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        _: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        // we match the haskell implementation by encoding bytestrings longer than 64
        // bytes as indefinite lists of bytes
        const CHUNK_SIZE: usize = 64;
        let bs: &Vec<u8> = self.deref();
        if bs.len() <= 64 {
            e.bytes(bs)?;
        } else {
            e.begin_bytes()?;
            for b in bs.chunks(CHUNK_SIZE) {
                e.bytes(b)?;
            }
            e.end()?;
        }
        Ok(())
    }
}

impl<'b, C> minicbor::decode::Decode<'b, C> for BoundedBytes {
    fn decode(d: &mut minicbor::Decoder<'b>, _: &mut C) -> Result<Self, minicbor::decode::Error> {
        let mut res = Vec::new();
        for chunk in d.bytes_iter()? {
            let bs = chunk?;
            res.extend_from_slice(bs);
        }
        Ok(BoundedBytes::from(res))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BigInt, BoundedBytes, Constr, KeyValuePairs, MaybeIndefArray};
    use proptest::{prelude::*, strategy::Just};

    prop_compose! {
        pub(crate) fn any_bounded_bytes()(
            bytes in any::<Vec<u8>>(),
        ) -> BoundedBytes {
            BoundedBytes::from(bytes)
        }
    }

    pub(crate) fn any_bigint() -> impl Strategy<Value = BigInt> {
        prop_oneof![
            any::<i64>().prop_map(|i| BigInt::Int(i.into())),
            any_bounded_bytes().prop_map(BigInt::BigUInt),
            any_bounded_bytes().prop_map(BigInt::BigNInt),
        ]
    }

    fn any_constr(depth: u8) -> impl Strategy<Value = Constr<PlutusData>> {
        let any_constr_tag = prop_oneof![
            (Just(102), any::<u64>().prop_map(Some)),
            (121_u64..=127, Just(None)),
            (1280_u64..=1400, Just(None))
        ];

        let any_fields = prop::collection::vec(any_plutus_data(depth - 1), 0..depth as usize);

        (any_constr_tag, any_fields, any::<bool>()).prop_map(
            |((tag, any_constructor), fields, is_def)| Constr {
                tag,
                any_constructor,
                fields: if is_def {
                    MaybeIndefArray::Def(fields)
                } else {
                    MaybeIndefArray::Indef(fields)
                },
            },
        )
    }

    fn any_plutus_data(depth: u8) -> BoxedStrategy<PlutusData> {
        let int = any_bigint().prop_map(PlutusData::BigInt);

        let bytes = any_bounded_bytes().prop_map(PlutusData::BoundedBytes);

        if depth > 0 {
            let constr = any_constr(depth).prop_map(PlutusData::Constr);

            let array = (
                any::<bool>(),
                prop::collection::vec(any_plutus_data(depth - 1), 0..depth as usize),
            )
                .prop_map(|(is_def, xs)| {
                    PlutusData::Array(if is_def {
                        MaybeIndefArray::Def(xs)
                    } else {
                        MaybeIndefArray::Indef(xs)
                    })
                });

            let map = (
                any::<bool>(),
                prop::collection::vec(
                    (any_plutus_data(depth - 1), any_plutus_data(depth - 1)),
                    0..depth as usize,
                ),
            )
                .prop_map(|(is_def, kvs)| {
                    PlutusData::Map(if is_def {
                        KeyValuePairs::Def(kvs)
                    } else {
                        KeyValuePairs::Indef(kvs)
                    })
                });

            prop_oneof![int, bytes, constr, array, map].boxed()
        } else {
            prop_oneof![int, bytes].boxed()
        }
    }

    proptest! {
        #[test]
        fn cbor_roundtrip(original_data in any_plutus_data(3)) {
            let bytes = minicbor::to_vec(&original_data).unwrap();
            let data: PlutusData = minicbor::decode(&bytes).unwrap();
            assert_eq!(data, original_data);
        }
    }
}
