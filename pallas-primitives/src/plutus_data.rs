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
use std::cmp::Ordering;
use std::{fmt, ops::Deref};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum PlutusData {
    Constr(Constr<PlutusData>),
    Map(KeyValuePairs<PlutusData, PlutusData>),
    Array(MaybeIndefArray<PlutusData>),
    BigInt(BigInt),
    BoundedBytes(BoundedBytes),
}

impl Eq for PlutusData {}

impl PartialEq for PlutusData {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl PartialOrd for PlutusData {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PlutusData {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Constr(left), Self::Constr(right)) => left.cmp(right),
            (Self::Constr(..), _) => Ordering::Less,
            (_, Self::Constr(..)) => Ordering::Greater,
            (Self::Map(left), Self::Map(right)) => left.deref().cmp(right.deref()),
            (Self::Map(..), _) => Ordering::Less,
            (_, Self::Map(..)) => Ordering::Greater,
            (Self::Array(left), Self::Array(right)) => left.deref().cmp(right.deref()),
            (Self::Array(..), _) => Ordering::Less,
            (_, Self::Array(..)) => Ordering::Greater,
            (Self::BigInt(left), Self::BigInt(right)) => left.cmp(right),
            (Self::BigInt(..), _) => Ordering::Less,
            (_, Self::BigInt(..)) => Ordering::Greater,
            (Self::BoundedBytes(left), Self::BoundedBytes(right)) => left.cmp(right),
        }
    }
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum BigInt {
    Int(Int),
    BigUInt(BoundedBytes),
    BigNInt(BoundedBytes),
}

impl Eq for BigInt {}

impl PartialEq for BigInt {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl PartialOrd for BigInt {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BigInt {
    fn cmp(&self, other: &Self) -> Ordering {
        fn to_bytes(i: &BigInt) -> (bool, Vec<u8>) {
            match i {
                BigInt::Int(i) => {
                    let i = Into::<i128>::into(*i);
                    (
                        i < 0,
                        i.abs()
                            .to_be_bytes()
                            .into_iter()
                            .skip_while(|b| b == &0)
                            .collect(),
                    )
                }
                BigInt::BigUInt(bs) => {
                    (false, bs.iter().skip_while(|b| b == &&0).copied().collect())
                }
                BigInt::BigNInt(bs) => {
                    (true, bs.iter().skip_while(|b| b == &&0).copied().collect())
                }
            }
        }

        let (left_is_negative, left) = to_bytes(self);

        let (right_is_negative, right) = to_bytes(other);

        if left.is_empty() && right.is_empty() {
            return Ordering::Equal;
        }

        if left_is_negative && !right_is_negative {
            return Ordering::Less;
        }

        if !left_is_negative && right_is_negative {
            return Ordering::Greater;
        }

        let when_positives = match left.len().cmp(&right.len()) {
            Ordering::Equal => left.cmp(&right),
            ordering => ordering,
        };

        if left_is_negative && right_is_negative {
            when_positives.reverse()
        } else {
            when_positives
        }
    }
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Constr<A> {
    pub tag: u64,
    pub any_constructor: Option<u64>,
    pub fields: MaybeIndefArray<A>,
}

impl<A: Ord> Eq for Constr<A> {}

impl<A: Ord> PartialEq for Constr<A> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl<A: Ord> PartialOrd for Constr<A> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<A: Ord> Ord for Constr<A> {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.constr_index().cmp(&other.constr_index()) {
            Ordering::Equal => self.fields.deref().cmp(other.fields.deref()),
            ordering => ordering,
        }
    }
}

impl<A> Constr<A> {
    pub fn constr_index(&self) -> u64 {
        match self.tag {
            121..=127 => self.tag - 121,
            1280..=1400 => self.tag - 1280 + 7,
            102 => self
                .any_constructor
                .unwrap_or_else(|| panic!("malformed Constr: missing 'any_constructor'")),
            tag => panic!("malformed Constr: invalid tag {tag:?}"),
        }
    }
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
    use test_case::test_case;

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

    /// Swap some Def to Indef (or vice-versa), in an existing PlutusData. The
    /// 'depth' parameter is used to avoid always changing the outer-most
    /// layer, but also try to alter some nested element, if any.
    fn alter_any_encoding(data: PlutusData, depth: usize) -> PlutusData {
        let alter_kvs = |kvs: Vec<(PlutusData, PlutusData)>| -> Vec<(PlutusData, PlutusData)> {
            kvs.into_iter()
                .map(|(k, v)| {
                    (
                        alter_any_encoding(k, depth - 1),
                        alter_any_encoding(v, depth - 1),
                    )
                })
                .collect()
        };

        let alter_vec = |xs: Vec<PlutusData>| -> Vec<PlutusData> {
            xs.into_iter()
                .map(|x| alter_any_encoding(x, depth - 1))
                .collect()
        };

        match data {
            PlutusData::BigInt(i) => PlutusData::BigInt(i),
            PlutusData::BoundedBytes(bs) => PlutusData::BoundedBytes(bs),
            PlutusData::Map(m) => PlutusData::Map(match m {
                KeyValuePairs::Def(kvs) if depth > 1 => KeyValuePairs::Def(alter_kvs(kvs)),
                KeyValuePairs::Def(kvs) => KeyValuePairs::Indef(kvs),
                KeyValuePairs::Indef(kvs) if depth > 1 => KeyValuePairs::Indef(alter_kvs(kvs)),
                KeyValuePairs::Indef(kvs) => KeyValuePairs::Def(kvs),
            }),
            PlutusData::Array(m) => PlutusData::Array(match m {
                MaybeIndefArray::Def(xs) if depth > 1 => MaybeIndefArray::Def(alter_vec(xs)),
                MaybeIndefArray::Def(xs) => MaybeIndefArray::Indef(xs),
                MaybeIndefArray::Indef(xs) if depth > 1 => MaybeIndefArray::Indef(alter_vec(xs)),
                MaybeIndefArray::Indef(xs) => MaybeIndefArray::Def(xs),
            }),
            PlutusData::Constr(Constr {
                tag,
                any_constructor,
                fields,
            }) => PlutusData::Constr(Constr {
                tag,
                any_constructor,
                fields: match fields {
                    MaybeIndefArray::Def(xs) if depth > 1 => MaybeIndefArray::Def(alter_vec(xs)),
                    MaybeIndefArray::Def(xs) => MaybeIndefArray::Indef(xs),
                    MaybeIndefArray::Indef(xs) if depth > 1 => {
                        MaybeIndefArray::Indef(alter_vec(xs))
                    }
                    MaybeIndefArray::Indef(xs) => MaybeIndefArray::Def(xs),
                },
            }),
        }
    }

    proptest! {
        #[test]
        fn equals_list_irrespective_of_encoding((depth, left) in (1..=3_usize, any_plutus_data(3))) {
            let right = alter_any_encoding(left.clone(), depth);
            assert_eq!(left, right);
            assert_eq!(right, left);
        }
    }

    fn int(i: i64) -> PlutusData {
        PlutusData::BigInt(BigInt::Int(i.into()))
    }

    fn biguint(bs: &[u8]) -> PlutusData {
        PlutusData::BigInt(BigInt::BigUInt(BoundedBytes::from(bs.to_vec())))
    }

    fn bignint(bs: &[u8]) -> PlutusData {
        PlutusData::BigInt(BigInt::BigNInt(BoundedBytes::from(bs.to_vec())))
    }

    fn bytes(bs: &[u8]) -> PlutusData {
        PlutusData::BoundedBytes(BoundedBytes::from(bs.to_vec()))
    }

    fn array_def(xs: &[PlutusData]) -> PlutusData {
        PlutusData::Array(MaybeIndefArray::Def(xs.to_vec()))
    }

    fn array_indef(xs: &[PlutusData]) -> PlutusData {
        PlutusData::Array(MaybeIndefArray::Indef(xs.to_vec()))
    }

    fn map_def(kvs: &[(PlutusData, PlutusData)]) -> PlutusData {
        PlutusData::Map(KeyValuePairs::Def(kvs.to_vec()))
    }

    fn map_indef(kvs: &[(PlutusData, PlutusData)]) -> PlutusData {
        PlutusData::Map(KeyValuePairs::Indef(kvs.to_vec()))
    }

    fn constr(tag: u64, fields: &[PlutusData]) -> PlutusData {
        PlutusData::Constr(Constr {
            tag,
            any_constructor: None,
            fields: MaybeIndefArray::Def(fields.to_vec()),
        })
    }

    fn constr_any(any_constructor: u64, fields: &[PlutusData]) -> PlutusData {
        PlutusData::Constr(Constr {
            tag: 102,
            any_constructor: Some(any_constructor),
            fields: MaybeIndefArray::Def(fields.to_vec()),
        })
    }

    // Bytes <-> ...
    #[test_case(bytes(&[]), bytes(&[]) => Ordering::Equal)]
    #[test_case(bytes(&[1, 2, 3]), bytes(&[4, 5, 6]) => Ordering::Less)]
    #[test_case(bytes(&[1, 2, 3]), bytes(&[1, 2, 3]) => Ordering::Equal)]
    #[test_case(bytes(&[4, 5, 6]), bytes(&[1, 2, 3]) => Ordering::Greater)]
    #[test_case(bytes(&[1, 2, 3]), bytes(&[2, 2, 3]) => Ordering::Less)]
    #[test_case(bytes(&[1, 2, 3]), bytes(&[1, 2]) => Ordering::Greater)]
    #[test_case(bytes(&[2, 2]), bytes(&[1, 2, 3]) => Ordering::Greater)]
    #[test_case(bytes(&[]), constr(121, &[]) => Ordering::Greater)]
    #[test_case(bytes(&[]), map_def(&[]) => Ordering::Greater)]
    #[test_case(bytes(&[]), map_indef(&[]) => Ordering::Greater)]
    #[test_case(bytes(&[]), array_def(&[]) => Ordering::Greater)]
    #[test_case(bytes(&[]), array_indef(&[]) => Ordering::Greater)]
    #[test_case(bytes(&[]), int(0) => Ordering::Greater)]
    // Int <-> ...
    #[test_case(int(42), int(14) => Ordering::Greater)]
    #[test_case(int(14), int(14) => Ordering::Equal)]
    #[test_case(int(14), int(42) => Ordering::Less)]
    #[test_case(int(0), int(-1) => Ordering::Greater)]
    #[test_case(int(-2), int(-1) => Ordering::Less)]
    #[test_case(int(0), biguint(&[0]) => Ordering::Equal)]
    #[test_case(int(14), biguint(&[14]) => Ordering::Equal)]
    #[test_case(int(14), biguint(&[42]) => Ordering::Less)]
    #[test_case(biguint(&[14]), int(42) => Ordering::Less)]
    #[test_case(biguint(&[42]), int(14) => Ordering::Greater)]
    #[test_case(biguint(&[14, 255]), int(42) => Ordering::Greater)]
    #[test_case(bignint(&[0]), int(0) => Ordering::Equal)]
    #[test_case(bignint(&[14, 255]), int(-42) => Ordering::Less)]
    #[test_case(biguint(&[]), int(0) => Ordering::Equal)]
    #[test_case(biguint(&[0, 0, 1]), int(1) => Ordering::Equal)]
    #[test_case(int(0), constr(121, &[]) => Ordering::Greater)]
    #[test_case(int(0), map_def(&[]) => Ordering::Greater)]
    #[test_case(int(0), map_indef(&[]) => Ordering::Greater)]
    #[test_case(int(0), array_def(&[]) => Ordering::Greater)]
    #[test_case(int(0), array_indef(&[]) => Ordering::Greater)]
    #[test_case(int(0), bytes(&[]) => Ordering::Less)]
    // Array <-> ...
    #[test_case(array_def(&[]), array_def(&[]) => Ordering::Equal)]
    #[test_case(array_def(&[]), array_indef(&[]) => Ordering::Equal)]
    #[test_case(array_indef(&[]), array_def(&[]) => Ordering::Equal)]
    #[test_case(array_def(&[int(14), int(42)]), array_def(&[int(14), int(42)]) => Ordering::Equal)]
    #[test_case(array_def(&[int(14), int(42)]), array_def(&[int(15)]) => Ordering::Less)]
    #[test_case(array_def(&[int(14), int(42)]), array_def(&[int(1), int(2), int(3)]) => Ordering::Greater)]
    #[test_case(array_def(&[int(14), int(42)]), array_indef(&[int(14), int(42)]) => Ordering::Equal)]
    #[test_case(array_indef(&[int(14), int(42)]), array_def(&[int(15)]) => Ordering::Less)]
    #[test_case(array_def(&[int(14), int(42)]), array_indef(&[int(1), int(2), int(3)]) => Ordering::Greater)]
    #[test_case(array_def(&[]), constr(121, &[]) => Ordering::Greater)]
    #[test_case(array_def(&[]), map_def(&[]) => Ordering::Greater)]
    #[test_case(array_def(&[]), map_indef(&[]) => Ordering::Greater)]
    #[test_case(array_def(&[]), int(0) => Ordering::Less)]
    #[test_case(array_def(&[]), bytes(&[]) => Ordering::Less)]
    // Map <--> ...
    #[test_case(map_def(&[]), map_def(&[]) => Ordering::Equal)]
    #[test_case(map_def(&[]), map_indef(&[]) => Ordering::Equal)]
    #[test_case(map_indef(&[]), map_def(&[]) => Ordering::Equal)]
    #[test_case(
        map_def(&[(int(14), int(42))]),
        map_def(&[(int(14), int(41))])
        => Ordering::Greater
    )]
    #[test_case(
        map_def(&[(int(14), int(41))]),
        map_def(&[(int(14), int(42))])
        => Ordering::Less
    )]
    #[test_case(
        map_def(&[(int(14), int(42))]),
        map_def(&[(int(14), int(42))])
        => Ordering::Equal
    )]
    #[test_case(
        map_def(&[(int(14), int(42))]),
        map_indef(&[(int(14), int(42)), (int(1), int(999))])
        => Ordering::Less
    )]
    #[test_case(
        map_def(&[(int(15), int(42))]),
        map_indef(&[(int(14), int(42)), (int(1), int(999))])
        => Ordering::Greater
    )]
    #[test_case(map_def(&[]), constr(121, &[]) => Ordering::Greater)]
    #[test_case(map_def(&[]), array_def(&[]) => Ordering::Less)]
    #[test_case(map_def(&[]), array_indef(&[]) => Ordering::Less)]
    #[test_case(map_def(&[]), int(0) => Ordering::Less)]
    #[test_case(map_def(&[]), bytes(&[]) => Ordering::Less)]
    // Constr <-->
    #[test_case(constr(121, &[]), constr(121, &[]) => Ordering::Equal)]
    #[test_case(constr(122, &[]), constr(121, &[]) => Ordering::Greater)]
    #[test_case(constr(122, &[]), constr(121, &[int(999)]) => Ordering::Greater)]
    #[test_case(constr(126, &[int(999)]), constr(1281, &[]) => Ordering::Less)]
    #[test_case(constr_any(0, &[]), constr(121, &[]) => Ordering::Equal)]
    #[test_case(constr_any(1, &[]), constr(121, &[]) => Ordering::Greater)]
    #[test_case(constr_any(7, &[int(14)]), constr(1280, &[]) => Ordering::Greater)]
    #[test_case(constr_any(7, &[int(14)]), constr(1281, &[]) => Ordering::Less)]
    #[test_case(constr_any(121, &[]), map_def(&[]) => Ordering::Less)]
    #[test_case(constr_any(121, &[]), map_indef(&[]) => Ordering::Less)]
    #[test_case(constr_any(121, &[]), array_def(&[]) => Ordering::Less)]
    #[test_case(constr_any(121, &[]), array_indef(&[]) => Ordering::Less)]
    #[test_case(constr_any(121, &[]), int(0) => Ordering::Less)]
    #[test_case(constr_any(121, &[]), bytes(&[]) => Ordering::Less)]
    fn ordering(left: PlutusData, right: PlutusData) -> Ordering {
        left.cmp(&right)
    }
}
