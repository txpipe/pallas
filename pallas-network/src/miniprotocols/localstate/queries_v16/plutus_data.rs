//! Stack-safe node-to-client datum operations. This representation keeps
//! its existing structural ordering and definite-map wire normalization.
use super::{BigInt, BoundedBytes, Constr, PlutusData};
use pallas_codec::tree::{
    Arity, IndexedNode, MAX_DECODE_DEPTH, TreeDecode, Visit, cmp_tree, decode_tree_with_max_depth,
    fold_tree, walk_tree,
};
use pallas_codec::{
    minicbor::{
        self,
        data::{IanaTag, Tag, Type},
    },
    utils::{Int, KeyValuePairs, MaybeIndefArray},
};
use std::cmp::Ordering;

impl PlutusData {
    /// Extracts the `Constr` payload without cloning it.
    ///
    /// Returns the original value unchanged if it has a different variant.
    pub fn into_constr(mut self) -> Result<Constr<Self>, Self> {
        match &mut self {
            Self::Constr(value) => Ok(std::mem::replace(
                value,
                Constr {
                    tag: 121,
                    any_constructor: None,
                    fields: MaybeIndefArray::Def(Vec::new()),
                },
            )),
            _ => Err(self),
        }
    }

    /// Extracts the `Map` payload without cloning it.
    ///
    /// Returns the original value unchanged if it has a different variant.
    pub fn into_map(mut self) -> Result<KeyValuePairs<Self, Self>, Self> {
        match &mut self {
            Self::Map(value) => Ok(std::mem::replace(value, KeyValuePairs::Def(Vec::new()))),
            _ => Err(self),
        }
    }

    /// Extracts the `Array` payload without cloning it.
    ///
    /// Returns the original value unchanged if it has a different variant.
    pub fn into_array(mut self) -> Result<MaybeIndefArray<Self>, Self> {
        match &mut self {
            Self::Array(value) => Ok(std::mem::replace(value, MaybeIndefArray::Def(Vec::new()))),
            _ => Err(self),
        }
    }

    /// Extracts the `BigInt` payload without cloning it.
    ///
    /// Returns the original value unchanged if it has a different variant.
    pub fn into_bigint(mut self) -> Result<BigInt, Self> {
        match &mut self {
            Self::BigInt(value) => Ok(std::mem::replace(value, BigInt::Int(Int::from(0)))),
            _ => Err(self),
        }
    }

    /// Extracts the `BoundedBytes` payload without cloning it.
    ///
    /// Returns the original value unchanged if it has a different variant.
    pub fn into_bounded_bytes(mut self) -> Result<BoundedBytes, Self> {
        match &mut self {
            Self::BoundedBytes(value) => Ok(std::mem::replace(value, Vec::new().into())),
            _ => Err(self),
        }
    }
}

impl IndexedNode for PlutusData {
    fn child_count(&self) -> usize {
        match self {
            Self::Constr(x) => x.fields.len(),
            Self::Map(kvs) => kvs.len() * 2,
            Self::Array(xs) => xs.len(),
            Self::BigInt(_) | Self::BoundedBytes(_) => 0,
        }
    }

    fn child(&self, index: usize) -> &Self {
        match self {
            Self::Constr(x) => &x.fields[index],
            Self::Map(kvs) => {
                let (k, v) = &kvs[index / 2];
                if index.is_multiple_of(2) { k } else { v }
            }
            Self::Array(xs) => &xs[index],
            Self::BigInt(_) | Self::BoundedBytes(_) => unreachable!("leaves have no children"),
        }
    }
}

fn array<A>(indefinite: bool, items: Vec<A>) -> MaybeIndefArray<A> {
    if indefinite {
        MaybeIndefArray::Indef(items)
    } else {
        MaybeIndefArray::Def(items)
    }
}

/// Pairs up an alternating key, value list.
fn pairs<A>(items: Vec<A>) -> Vec<(A, A)> {
    let mut pairs = Vec::with_capacity(items.len() / 2);
    let mut items = items.into_iter();
    while let (Some(k), Some(v)) = (items.next(), items.next()) {
        pairs.push((k, v));
    }
    pairs
}

fn map(indefinite: bool, pairs: Vec<(PlutusData, PlutusData)>) -> PlutusData {
    PlutusData::Map(if indefinite {
        KeyValuePairs::Indef(pairs)
    } else {
        KeyValuePairs::Def(pairs)
    })
}

/// Copies with a heap-backed stack: a derived clone recurses per nesting
/// level and overflows on chain-deep datums.
impl Clone for PlutusData {
    fn clone(&self) -> Self {
        fold_tree(self, |node, children| match node {
            Self::Constr(x) => Self::Constr(Constr {
                tag: x.tag,
                any_constructor: x.any_constructor,
                fields: array(matches!(x.fields, MaybeIndefArray::Indef(_)), children),
            }),
            Self::Map(kvs) => map(matches!(kvs, KeyValuePairs::Indef(_)), pairs(children)),
            Self::Array(xs) => {
                Self::Array(array(matches!(xs, MaybeIndefArray::Indef(_)), children))
            }
            Self::BigInt(x) => Self::BigInt(x.clone()),
            Self::BoundedBytes(x) => Self::BoundedBytes(x.clone()),
        })
    }
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

/// Orders by variant, then by a node's own data, then by children; the
/// container encoding (definite or indefinite) does not take part.
impl Ord for PlutusData {
    fn cmp(&self, other: &Self) -> Ordering {
        fn rank(x: &PlutusData) -> u8 {
            match x {
                PlutusData::Constr(_) => 0,
                PlutusData::Map(_) => 1,
                PlutusData::BigInt(_) => 2,
                PlutusData::BoundedBytes(_) => 3,
                PlutusData::Array(_) => 4,
            }
        }
        cmp_tree(self, other, |a, b| match (a, b) {
            (Self::Constr(a), Self::Constr(b)) => a
                .tag
                .cmp(&b.tag)
                .then(a.any_constructor.cmp(&b.any_constructor))
                .then(
                    matches!(a.fields, MaybeIndefArray::Indef(_))
                        .cmp(&matches!(b.fields, MaybeIndefArray::Indef(_))),
                ),
            (Self::Map(a), Self::Map(b)) => {
                matches!(a, KeyValuePairs::Indef(_)).cmp(&matches!(b, KeyValuePairs::Indef(_)))
            }
            (Self::Array(a), Self::Array(b)) => {
                matches!(a, MaybeIndefArray::Indef(_)).cmp(&matches!(b, MaybeIndefArray::Indef(_)))
            }
            (Self::BigInt(a), Self::BigInt(b)) => a.cmp(b),
            (Self::BoundedBytes(a), Self::BoundedBytes(b)) => a.cmp(b),
            _ => rank(a).cmp(&rank(b)),
        })
    }
}

// Private wrapper so the tree-decoding builder stays out of the public API.
struct Node(PlutusData);

enum Partial {
    Leaf(PlutusData),
    Array {
        indefinite: bool,
        items: Vec<PlutusData>,
    },
    /// Keys and values arrive alternately.
    Map {
        indefinite: bool,
        items: Vec<PlutusData>,
    },
    Constr {
        tag: u64,
        any_constructor: Option<u64>,
        indefinite: bool,
        fields: Vec<PlutusData>,
    },
}

/// Holds a node's state while its children decode. `PlutusData` drops
/// recursively, so completed children are released recursively when decoding
/// fails.
struct Builder(Option<Partial>);

impl Builder {
    fn new(partial: Partial) -> Self {
        Self(Some(partial))
    }

    fn partial(&mut self) -> &mut Partial {
        self.0.as_mut().expect("taken only when the node ends")
    }
}

impl<'b, C> TreeDecode<'b, C> for Node {
    type Builder = Builder;

    fn begin(
        d: &mut minicbor::Decoder<'b>,
        ctx: &mut C,
    ) -> Result<(Builder, Arity), minicbor::decode::Error> {
        let leaf = |x| Ok((Builder::new(Partial::Leaf(x)), Arity::Leaf));

        match d.datatype()? {
            Type::Tag => {
                let tag = d.probe().tag()?;

                if tag == IanaTag::PosBignum.tag() || tag == IanaTag::NegBignum.tag() {
                    return leaf(PlutusData::BigInt(d.decode_with(ctx)?));
                }

                let tag = d.tag()?.as_u64();
                let any_constructor = match tag {
                    121..=127 | 1280..=1400 => None,
                    102 => {
                        d.array()?;
                        Some(d.u64()?)
                    }
                    _ => {
                        return Err(minicbor::decode::Error::message(
                            "unknown tag for plutus data tag",
                        ));
                    }
                };
                let len = d.array()?;
                let partial = Partial::Constr {
                    tag,
                    any_constructor,
                    indefinite: len.is_none(),
                    fields: Vec::new(),
                };
                Ok((Builder::new(partial), len.into()))
            }
            Type::U8
            | Type::U16
            | Type::U32
            | Type::U64
            | Type::I8
            | Type::I16
            | Type::I32
            | Type::I64
            | Type::Int => leaf(PlutusData::BigInt(d.decode_with(ctx)?)),
            Type::Map | Type::MapIndef => {
                let len = d.map()?;
                let arity = match len {
                    Some(pairs) => Arity::Fixed(pairs.checked_mul(2).ok_or_else(|| {
                        minicbor::decode::Error::message("plutus data map too long")
                    })?),
                    None => Arity::Indefinite,
                };
                let partial = Partial::Map {
                    indefinite: len.is_none(),
                    items: Vec::new(),
                };
                Ok((Builder::new(partial), arity))
            }
            Type::Bytes | Type::BytesIndef => leaf(PlutusData::BoundedBytes(d.decode_with(ctx)?)),
            Type::Array | Type::ArrayIndef => {
                let len = d.array()?;
                let partial = Partial::Array {
                    indefinite: len.is_none(),
                    items: Vec::new(),
                };
                Ok((Builder::new(partial), len.into()))
            }
            any => Err(minicbor::decode::Error::message(format!(
                "bad cbor data type ({any:?}) for plutus data"
            ))),
        }
    }

    fn child(builder: &mut Builder, child: Node) -> Result<(), minicbor::decode::Error> {
        match builder.partial() {
            Partial::Array { items, .. }
            | Partial::Map { items, .. }
            | Partial::Constr { fields: items, .. } => items.push(child.0),
            Partial::Leaf(_) => unreachable!("leaves report Arity::Leaf"),
        }
        Ok(())
    }

    fn end(
        mut builder: Builder,
        _: &mut minicbor::Decoder<'b>,
        _: &mut C,
    ) -> Result<Node, minicbor::decode::Error> {
        // Checked while the builder still owns the items; on error they drop
        // recursively, like every `PlutusData`.
        if let Partial::Map { items, .. } = builder.partial()
            && items.len() % 2 != 0
        {
            return Err(minicbor::decode::Error::message(
                "plutus data map ended after a key",
            ));
        }

        let partial = builder.0.take().expect("taken only when the node ends");
        let data = match partial {
            Partial::Leaf(x) => x,
            Partial::Array { indefinite, items } => PlutusData::Array(array(indefinite, items)),
            Partial::Map { indefinite, items } => map(indefinite, pairs(items)),
            Partial::Constr {
                tag,
                any_constructor,
                indefinite,
                fields,
            } => PlutusData::Constr(Constr {
                tag,
                any_constructor,
                fields: array(indefinite, fields),
            }),
        };
        Ok(Node(data))
    }
}

/// Decodes with a heap-backed stack: datums nest as deep as a transaction
/// has bytes, and a decoder that recurses per level overflows the thread
/// stack well before that.
impl<'b, C> minicbor::decode::Decode<'b, C> for PlutusData {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        decode_tree_with_max_depth::<C, Node>(d, ctx, MAX_DECODE_DEPTH).map(|node| node.0)
    }
}

/// Writes a definite or indefinite array header.
fn array_header<W: minicbor::encode::Write>(
    e: &mut minicbor::Encoder<W>,
    items: &MaybeIndefArray<PlutusData>,
) -> Result<(), minicbor::encode::Error<W::Error>> {
    match items {
        MaybeIndefArray::Def(xs) => e.array(xs.len() as u64)?,
        MaybeIndefArray::Indef(_) => e.begin_array()?,
    };
    Ok(())
}

/// Encodes with a heap-backed stack. Preserve the protocol codec's
/// normalization of maps to definite form, while retaining array forms.
impl<C> minicbor::encode::Encode<C> for PlutusData {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        walk_tree(self, |visit| {
            match visit {
                Visit::Enter(Self::Constr(x)) => {
                    e.tag(Tag::new(x.tag))?;
                    if x.tag == 102 {
                        e.array(2)?;
                        e.u64(x.any_constructor.unwrap_or_default())?;
                    }
                    array_header(e, &x.fields)?;
                }
                Visit::Enter(Self::Array(xs)) => array_header(e, xs)?,
                Visit::Enter(Self::Map(KeyValuePairs::Def(kvs))) => {
                    e.map(kvs.len() as u64)?;
                }
                Visit::Enter(Self::Map(KeyValuePairs::Indef(kvs))) => {
                    e.map(kvs.len() as u64)?;
                }
                Visit::Enter(Self::BigInt(x)) => {
                    e.encode_with(x, ctx)?;
                }
                Visit::Enter(Self::BoundedBytes(x)) => {
                    e.encode_with(x, ctx)?;
                }
                Visit::Exit(
                    Self::Constr(Constr {
                        fields: MaybeIndefArray::Indef(_),
                        ..
                    })
                    | Self::Array(MaybeIndefArray::Indef(_)),
                ) => {
                    e.end()?;
                }
                Visit::Between(_) | Visit::Exit(_) => {}
            }
            Ok(())
        })
    }
}
