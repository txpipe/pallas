use std::ops::Deref;

use minicbor::{data::Tag, Decode, Encode};

/// Utility for skipping parts of the CBOR payload, use only for debugging
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord)]
pub struct SkipCbor<const N: usize> {}

impl<'b, const N: usize> minicbor::Decode<'b> for SkipCbor<N> {
    fn decode(d: &mut minicbor::Decoder<'b>) -> Result<Self, minicbor::decode::Error> {
        {
            let probe = d.probe();
            log::warn!("skipped cbor value {}: {:?}", N, probe.datatype()?);
            println!("skipped cbor value {}: {:?}", N, probe.datatype()?);
        }

        d.skip()?;
        Ok(SkipCbor {})
    }
}

impl<const N: usize> minicbor::Encode for SkipCbor<N> {
    fn encode<W: minicbor::encode::Write>(
        &self,
        _e: &mut minicbor::Encoder<W>,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        todo!()
    }
}

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
            _ => Err(minicbor::decode::Error::message(
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
            _ => Err(minicbor::decode::Error::message(
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
            // TODO: this seemed necesary on alonzo, but breaks on byron. We need to double check.
            //MaybeIndefArray::Indef(x) if x.is_empty() => {
            //    e.encode(x)?;
            //}
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

/// Order-preserving set of attributes
///
/// There's no guarantee that the entries on a Cardano cbor entity that uses
/// maps for its representation will follow the canonical order specified by the
/// standard. To implement an isomorphic codec, we need a way of preserving the
/// original order in which the entries were encoded. To acomplish this, we
/// transform key-value structures into an orderer vec of `properties`, where
/// each entry represents a a cbor-encodable variant of an attribute of the
/// struct.
#[derive(Debug, PartialEq)]
pub struct OrderPreservingProperties<P>(Vec<P>);

impl<P> Deref for OrderPreservingProperties<P> {
    type Target = Vec<P>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'b, P> minicbor::decode::Decode<'b> for OrderPreservingProperties<P>
where
    P: Decode<'b>,
{
    fn decode(d: &mut minicbor::Decoder<'b>) -> Result<Self, minicbor::decode::Error> {
        let len = d.map()?.unwrap_or_default();

        let components: Result<_, _> = (0..len).map(|_| d.decode()).collect();

        Ok(Self(components?))
    }
}

impl<P> minicbor::encode::Encode for OrderPreservingProperties<P>
where
    P: Encode,
{
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.map(self.0.len() as u64)?;
        for component in &self.0 {
            e.encode(component)?;
        }

        Ok(())
    }
}

/// Wraps a struct so that it is encoded/decoded as a cbor bytes
#[derive(Debug)]
pub struct CborWrap<T>(pub T);

impl<'b, T> minicbor::Decode<'b> for CborWrap<T>
where
    T: minicbor::Decode<'b>,
{
    fn decode(d: &mut minicbor::Decoder<'b>) -> Result<Self, minicbor::decode::Error> {
        d.tag()?;
        let cbor = d.bytes()?;
        let wrapped = minicbor::decode(cbor)?;

        Ok(CborWrap(wrapped))
    }
}

impl<T> minicbor::Encode for CborWrap<T>
where
    T: minicbor::Encode,
{
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        let buf = minicbor::to_vec(&self.0).map_err(|_| {
            minicbor::encode::Error::message("error encoding cbor-wrapped structure")
        })?;

        e.tag(Tag::Cbor)?;
        e.bytes(&buf)?;

        Ok(())
    }
}

impl<T> Deref for CborWrap<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
pub struct TagWrap<I, const T: u64>(I);

impl<'b, I, const T: u64> minicbor::Decode<'b> for TagWrap<I, T>
where
    I: minicbor::Decode<'b>,
{
    fn decode(d: &mut minicbor::Decoder<'b>) -> Result<Self, minicbor::decode::Error> {
        d.tag()?;

        Ok(TagWrap(d.decode()?))
    }
}

impl<I, const T: u64> minicbor::Encode for TagWrap<I, T>
where
    I: minicbor::Encode,
{
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.tag(Tag::Unassigned(T))?;
        e.encode(&self.0)?;

        Ok(())
    }
}

/// An empty map
///
/// don't ask me why, that's what the CDDL asks for.
#[derive(Debug)]
pub struct EmptyMap;

impl<'b> minicbor::decode::Decode<'b> for EmptyMap {
    fn decode(d: &mut minicbor::Decoder<'b>) -> Result<Self, minicbor::decode::Error> {
        d.skip()?;
        Ok(EmptyMap)
    }
}

impl minicbor::encode::Encode for EmptyMap {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.map(0)?;

        Ok(())
    }
}

/// An array with zero or one elements
///
/// A common pattern seen in the CDDL is to represent optional values as an
/// array containing zero or more items. This structure reflects that pattern
/// while providing semantic meaning.
#[derive(Debug)]
pub struct ZeroOrOneArray<T>(Option<T>);

impl<T> Deref for ZeroOrOneArray<T> {
    type Target = Option<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'b, T> minicbor::decode::Decode<'b> for ZeroOrOneArray<T>
where
    T: Decode<'b>,
{
    fn decode(d: &mut minicbor::Decoder<'b>) -> Result<Self, minicbor::decode::Error> {
        let len = d.array()?;

        match len {
            Some(0) => Ok(ZeroOrOneArray(None)),
            Some(1) => Ok(ZeroOrOneArray(Some(d.decode()?))),
            Some(_) => Err(minicbor::decode::Error::message(
                "found invalid len for zero-or-one pattern",
            )),
            None => Err(minicbor::decode::Error::message(
                "found invalid indefinite len array for zero-or-one pattern",
            )),
        }
    }
}

impl<T> minicbor::encode::Encode for ZeroOrOneArray<T>
where
    T: minicbor::Encode,
{
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match &self.0 {
            Some(x) => {
                e.array(1)?;
                e.encode(x)?;
            }
            None => {
                e.array(0)?;
            }
        }

        Ok(())
    }
}
