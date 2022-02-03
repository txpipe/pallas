use std::ops::Deref;

use minicbor::{Decode, Encode};

/// Custom collection to ensure ordered pairs of values
///
/// Since the ordering of the entries requires a particular order to maintain
/// canonicalization for isomorphic decoding / encoding operators, we use a Vec
/// as the underlaying struct for storage of the items (as opposed to a BTreeMap
/// or HashMap).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum KeyValuePairs<K, V> {
    Def(Vec<(K, V)>),
    Indef(Vec<(K, V)>),
}

impl<K, V> Deref for KeyValuePairs<K, V> {
    type Target = Vec<(K, V)>;

    fn deref(&self) -> &Self::Target {
        match self {
            KeyValuePairs::Def(x) => x,
            KeyValuePairs::Indef(x) => x,
        }
    }
}

impl<'b, K, V> minicbor::decode::Decode<'b> for KeyValuePairs<K, V>
where
    K: Encode + Decode<'b>,
    V: Encode + Decode<'b>,
{
    fn decode(d: &mut minicbor::Decoder<'b>) -> Result<Self, minicbor::decode::Error> {
        let datatype = d.datatype()?;

        let items: Result<Vec<_>, _> = d.map_iter::<K, V>()?.collect();
        let items = items?;

        match datatype {
            minicbor::data::Type::Map => Ok(KeyValuePairs::Def(items)),
            minicbor::data::Type::MapIndef => Ok(KeyValuePairs::Indef(items)),
            _ => Err(minicbor::decode::Error::Message(
                "invalid data type for keyvaluepairs",
            )),
        }
    }
}

impl<K, V> minicbor::encode::Encode for KeyValuePairs<K, V>
where
    K: Encode,
    V: Encode,
{
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            KeyValuePairs::Def(x) => {
                e.map(x.len() as u64)?;

                for (k, v) in x.iter() {
                    k.encode(e)?;
                    v.encode(e)?;
                }
            }
            KeyValuePairs::Indef(x) => {
                e.begin_map()?;

                for (k, v) in x.iter() {
                    k.encode(e)?;
                    v.encode(e)?;
                }

                e.end()?;
            }
        }

        Ok(())
    }
}

/// A struct that maintains a reference to whether a cbor array was indef or not
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MaybeIndefArray<A> {
    Def(Vec<A>),
    Indef(Vec<A>),
}

impl<A> Deref for MaybeIndefArray<A> {
    type Target = Vec<A>;

    fn deref(&self) -> &Self::Target {
        match self {
            MaybeIndefArray::Def(x) => x,
            MaybeIndefArray::Indef(x) => x,
        }
    }
}

impl<'b, A> minicbor::decode::Decode<'b> for MaybeIndefArray<A>
where
    A: minicbor::decode::Decode<'b>,
{
    fn decode(d: &mut minicbor::Decoder<'b>) -> Result<Self, minicbor::decode::Error> {
        let datatype = d.datatype()?;

        match datatype {
            minicbor::data::Type::Array => Ok(Self::Def(d.decode()?)),
            minicbor::data::Type::ArrayIndef => Ok(Self::Indef(d.decode()?)),
            _ => Err(minicbor::decode::Error::Message(
                "unknown data type of maybe indef array",
            )),
        }
    }
}

impl<A> minicbor::encode::Encode for MaybeIndefArray<A>
where
    A: minicbor::encode::Encode,
{
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            MaybeIndefArray::Def(x) => {
                e.encode(x)?;
            }
            MaybeIndefArray::Indef(x) if x.is_empty() => {
                e.encode(x)?;
            }
            MaybeIndefArray::Indef(x) => {
                e.begin_array()?;

                for v in x.iter() {
                    e.encode(v)?;
                }

                e.end()?;
            }
        };

        Ok(())
    }
}
