use pallas_codec::minicbor::{self, data::Type};
use pallas_codec::tree::{Arity, IndexedNode, TreeDecode, decode_tree, fold_tree};
use pallas_codec::utils::KeyValuePairs;

use crate::Metadatum;

impl IndexedNode for Metadatum {
    fn child_count(&self) -> usize {
        match self {
            Self::Array(xs) => xs.len(),
            Self::Map(kvs) => kvs.len() * 2,
            Self::Int(_) | Self::Bytes(_) | Self::Text(_) => 0,
        }
    }

    fn child(&self, index: usize) -> &Self {
        match self {
            Self::Array(xs) => &xs[index],
            Self::Map(kvs) => {
                let (k, v) = &kvs[index / 2];
                if index.is_multiple_of(2) { k } else { v }
            }
            Self::Int(_) | Self::Bytes(_) | Self::Text(_) => {
                unreachable!("leaves have no children")
            }
        }
    }
}

/// Pairs up an alternating key, value list.
fn pairs(items: Vec<Metadatum>) -> Vec<(Metadatum, Metadatum)> {
    let mut pairs = Vec::with_capacity(items.len() / 2);
    let mut items = items.into_iter();
    while let (Some(k), Some(v)) = (items.next(), items.next()) {
        pairs.push((k, v));
    }
    pairs
}

fn map(indefinite: bool, pairs: Vec<(Metadatum, Metadatum)>) -> Metadatum {
    Metadatum::Map(if indefinite {
        KeyValuePairs::Indef(pairs)
    } else {
        KeyValuePairs::Def(pairs)
    })
}

/// Copies with a heap-backed stack: a derived clone recurses per nesting
/// level and overflows on chain-deep metadata.
impl Clone for Metadatum {
    fn clone(&self) -> Self {
        fold_tree(self, |node, children| match node {
            Self::Int(x) => Self::Int(*x),
            Self::Bytes(x) => Self::Bytes(x.clone()),
            Self::Text(x) => Self::Text(x.clone()),
            Self::Array(_) => Self::Array(children),
            Self::Map(kvs) => map(matches!(kvs, KeyValuePairs::Indef(_)), pairs(children)),
        })
    }
}

// Private wrapper so the tree-decoding builder stays out of the public API.
struct Node(Metadatum);

enum Partial {
    Leaf(Metadatum),
    Array(Vec<Metadatum>),
    /// Keys and values arrive alternately.
    Map {
        indefinite: bool,
        items: Vec<Metadatum>,
    },
}

/// Holds a node's state while its children decode. When decoding fails the
/// completed children it owns are released iteratively: `Metadatum` itself
/// drops recursively, and a completed child can be as deep as the input.
struct Builder(Option<Partial>);

impl Builder {
    fn new(partial: Partial) -> Self {
        Self(Some(partial))
    }

    fn partial(&mut self) -> &mut Partial {
        self.0.as_mut().expect("taken only when the node ends")
    }
}

impl Drop for Builder {
    fn drop(&mut self) {
        let mut pending = match self.0.take() {
            None | Some(Partial::Leaf(_)) => return,
            Some(Partial::Array(items) | Partial::Map { items, .. }) => items,
        };
        while let Some(datum) = pending.pop() {
            match datum {
                Metadatum::Array(xs) => pending.extend(xs),
                Metadatum::Map(KeyValuePairs::Def(kvs) | KeyValuePairs::Indef(kvs)) => {
                    for (k, v) in kvs {
                        pending.push(k);
                        pending.push(v);
                    }
                }
                Metadatum::Int(_) | Metadatum::Bytes(_) | Metadatum::Text(_) => {}
            }
        }
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
            Type::U8
            | Type::U16
            | Type::U32
            | Type::U64
            | Type::I8
            | Type::I16
            | Type::I32
            | Type::I64
            | Type::Int => leaf(Metadatum::Int(d.decode_with(ctx)?)),
            Type::Bytes => leaf(Metadatum::Bytes(d.decode_with(ctx)?)),
            Type::String | Type::StringIndef => leaf(Metadatum::Text(d.decode_with(ctx)?)),
            Type::Array | Type::ArrayIndef => {
                let len = d.array()?;
                Ok((Builder::new(Partial::Array(Vec::new())), len.into()))
            }
            Type::Map | Type::MapIndef => {
                let len = d.map()?;
                let arity = match len {
                    Some(pairs) => Arity::Fixed(pairs.checked_mul(2).ok_or_else(|| {
                        minicbor::decode::Error::message("metadatum map too long")
                    })?),
                    None => Arity::Indefinite,
                };
                let partial = Partial::Map {
                    indefinite: len.is_none(),
                    items: Vec::new(),
                };
                Ok((Builder::new(partial), arity))
            }
            any => Err(minicbor::decode::Error::message(format!(
                "bad cbor data type ({any:?}) for metadatum"
            ))),
        }
    }

    fn child(builder: &mut Builder, child: Node) -> Result<(), minicbor::decode::Error> {
        match builder.partial() {
            Partial::Array(items) | Partial::Map { items, .. } => items.push(child.0),
            Partial::Leaf(_) => unreachable!("leaves report Arity::Leaf"),
        }
        Ok(())
    }

    fn end(
        mut builder: Builder,
        _: &mut minicbor::Decoder<'b>,
        _: &mut C,
    ) -> Result<Node, minicbor::decode::Error> {
        // Checked while the builder still owns the items, so a chain-deep
        // dangling key is released iteratively by its drop.
        if let Partial::Map { items, .. } = builder.partial()
            && items.len() % 2 != 0
        {
            return Err(minicbor::decode::Error::message(
                "metadatum map ended after a key",
            ));
        }

        let partial = builder.0.take().expect("taken only when the node ends");
        let datum = match partial {
            Partial::Leaf(x) => x,
            Partial::Array(items) => Metadatum::Array(items),
            Partial::Map { indefinite, items } => map(indefinite, pairs(items)),
        };
        Ok(Node(datum))
    }
}

/// Decodes with a heap-backed stack: metadata nests as deep as a transaction
/// has bytes, and a decoder that recurses per level overflows the thread
/// stack well before that.
impl<'b, C> minicbor::decode::Decode<'b, C> for Metadatum {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        decode_tree::<C, Node>(d, ctx).map(|node| node.0)
    }
}

enum Step<'a> {
    Node(&'a Metadatum),
    /// Close an indefinite-length container.
    Break,
}

/// Encodes with a heap-backed stack, for the same reason decoding does.
impl<C> minicbor::encode::Encode<C> for Metadatum {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        let mut pending = vec![Step::Node(self)];
        while let Some(step) = pending.pop() {
            let datum = match step {
                Step::Node(datum) => datum,
                Step::Break => {
                    e.end()?;
                    continue;
                }
            };
            match datum {
                Metadatum::Int(x) => {
                    e.encode_with(x, ctx)?;
                }
                Metadatum::Bytes(x) => {
                    e.encode_with(x, ctx)?;
                }
                Metadatum::Text(x) => {
                    e.encode_with(x, ctx)?;
                }
                Metadatum::Array(xs) => {
                    e.array(xs.len() as u64)?;
                    pending.extend(xs.iter().rev().map(Step::Node));
                }
                Metadatum::Map(kvs) => {
                    match kvs {
                        KeyValuePairs::Def(kvs) => {
                            e.map(kvs.len() as u64)?;
                        }
                        KeyValuePairs::Indef(_) => {
                            e.begin_map()?;
                            pending.push(Step::Break);
                        }
                    }
                    for (k, v) in kvs.iter().rev() {
                        pending.push(Step::Node(v));
                        pending.push(Step::Node(k));
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pallas_codec::tree::{Visit, walk_tree};
    use pallas_codec::utils::Int;

    fn nested(level: &[u8], depth: usize, leaf: &[u8], close: &[u8]) -> Vec<u8> {
        let mut bytes = level.repeat(depth);
        bytes.extend_from_slice(leaf);
        bytes.extend(close.repeat(depth));
        bytes
    }

    fn depth_of(datum: &Metadatum) -> usize {
        let mut depth = 0;
        let mut cursor = datum;
        loop {
            let next = match cursor {
                Metadatum::Array(xs) => xs.first(),
                Metadatum::Map(kvs) => kvs.first().map(|(_, v)| v),
                _ => return depth,
            };
            let Some(next) = next else { return depth };
            cursor = next;
            depth += 1;
        }
    }

    /// One level of nesting per shape: definite and indefinite arrays,
    /// definite and indefinite single-entry maps.
    const SHAPES: &[(&[u8], &[u8])] = &[
        (&[0x81], &[]),
        (&[0x9f], &[0xff]),
        (&[0xa1, 0x00], &[]),
        (&[0xbf, 0x00], &[0xff]),
    ];

    #[test]
    fn decodes_deep_nesting_on_a_small_stack() {
        std::thread::Builder::new()
            .stack_size(128 * 1024)
            .spawn(|| {
                for (level, close) in SHAPES {
                    let depth = 20_000;
                    let bytes = nested(level, depth, &[0x00], close);
                    let datum: Metadatum = minicbor::decode(&bytes).unwrap();
                    // Drop still recurses; leak so only decoding is under test.
                    let datum = std::mem::ManuallyDrop::new(datum);
                    assert_eq!(depth_of(&datum), depth, "shape {level:02x?}");
                }
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn encodes_deep_nesting_on_a_small_stack() {
        std::thread::Builder::new()
            .stack_size(128 * 1024)
            .spawn(|| {
                let depth = 20_000;
                let encodings: [(&[u8], &[u8]); 3] = [
                    (&[0x81], &[]),
                    (&[0xa1, 0x00], &[]),
                    (&[0xbf, 0x00], &[0xff]),
                ];
                for (datum, (level, close)) in deep_shapes(depth).into_iter().zip(encodings) {
                    let bytes = minicbor::to_vec(&*datum).unwrap();
                    assert_eq!(bytes, nested(level, depth, &[0x00], close));
                }
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn rejects_malformed_deep_nesting_and_cleans_up_on_a_small_stack() {
        std::thread::Builder::new()
            .stack_size(128 * 1024)
            .spawn(|| {
                for (level, close) in SHAPES {
                    // Cut before the leaf, so no subtree completes, and cut
                    // the last byte, which for indefinite shapes leaves a
                    // completed deep child for error cleanup to release.
                    let bytes = nested(level, 20_000, &[0x00], close);
                    for cut in [level.len() * 20_000, bytes.len() - 1] {
                        assert!(
                            minicbor::decode::<Metadatum>(&bytes[..cut]).is_err(),
                            "shape {level:02x?} cut at {cut}"
                        );
                    }
                }

                // A parent expecting two children: the first is complete and
                // deep, the second is missing or malformed.
                for tail in [&[][..], &[0xff][..]] {
                    let mut bytes = vec![0x82];
                    bytes.extend(nested(&[0x81], 20_000, &[0x00], &[]));
                    bytes.extend_from_slice(tail);
                    assert!(minicbor::decode::<Metadatum>(&bytes).is_err());
                }

                // An indefinite map that breaks right after a complete, deep
                // key: the dangling key is released during error cleanup.
                for (level, close) in SHAPES {
                    let mut bytes = vec![0xbf];
                    bytes.extend(nested(level, 20_000, &[0x00], close));
                    bytes.push(0xff);
                    assert!(
                        minicbor::decode::<Metadatum>(&bytes).is_err(),
                        "dangling key of shape {level:02x?}"
                    );
                }
            })
            .unwrap()
            .join()
            .unwrap();
    }

    fn wrap(depth: usize, level: impl Fn(Metadatum) -> Metadatum) -> Metadatum {
        let mut datum = Metadatum::Int(Int::from(0));
        for _ in 0..depth {
            datum = level(datum);
        }
        datum
    }

    fn key() -> Metadatum {
        Metadatum::Int(Int::from(0))
    }

    /// Chain-deep values of every container shape, leaked because `Drop`
    /// still recurses and only the operation under test should run.
    fn deep_shapes(depth: usize) -> Vec<std::mem::ManuallyDrop<Metadatum>> {
        [
            wrap(depth, |x| Metadatum::Array(vec![x])),
            wrap(depth, |x| {
                Metadatum::Map(KeyValuePairs::Def(vec![(key(), x)]))
            }),
            wrap(depth, |x| {
                Metadatum::Map(KeyValuePairs::Indef(vec![(key(), x)]))
            }),
        ]
        .into_iter()
        .map(std::mem::ManuallyDrop::new)
        .collect()
    }

    fn render(datum: &Metadatum) -> String {
        let mut out = String::new();
        walk_tree::<_, std::fmt::Error>(datum, |visit| {
            match visit {
                Visit::Enter(Metadatum::Int(x)) => out.push_str(&x.to_string()),
                Visit::Enter(Metadatum::Text(x)) => out.push_str(x),
                Visit::Enter(Metadatum::Bytes(x)) => out.push_str(&hex::encode(x.as_slice())),
                Visit::Enter(Metadatum::Array(_)) => out.push('['),
                Visit::Enter(Metadatum::Map(_)) => out.push('{'),
                Visit::Between(_) => out.push(','),
                Visit::Exit(Metadatum::Array(_)) => out.push(']'),
                Visit::Exit(Metadatum::Map(_)) => out.push('}'),
                Visit::Exit(_) => {}
            }
            Ok(())
        })
        .unwrap();
        out
    }

    #[test]
    fn walks_keys_and_values_in_order() {
        let bytes = hex::decode("a2018200430102036161bf20626162ff").unwrap();
        let datum: Metadatum = minicbor::decode(&bytes).unwrap();
        assert_eq!(render(&datum), "{1,[0,010203],a,{-1,ab}}");
        assert_eq!(render(&datum.clone()), render(&datum));
        assert_eq!(datum.clone(), datum);
    }

    #[test]
    fn clones_deep_nesting_on_a_small_stack() {
        std::thread::Builder::new()
            .stack_size(128 * 1024)
            .spawn(|| {
                let depth = 20_000;
                for datum in deep_shapes(depth) {
                    let copy = std::mem::ManuallyDrop::new(Metadatum::clone(&datum));
                    assert_eq!(depth_of(&copy), depth);
                    assert_eq!(
                        minicbor::to_vec(&*copy).unwrap(),
                        minicbor::to_vec(&*datum).unwrap()
                    );
                }
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn round_trips_leaves_and_containers() {
        for hex in [
            "00",
            "1863",
            "20",
            "3863",
            "43010203",
            "63616263",
            "8100",
            "a10001",
            "bf0001ff",
            "a2 01 82 00 43010203 61 6b bf 20 63616263 ff",
        ] {
            let bytes = hex::decode(hex.replace(' ', "")).unwrap();
            let datum: Metadatum = minicbor::decode(&bytes).unwrap();
            assert_eq!(minicbor::to_vec(&datum).unwrap(), bytes, "{hex}");
        }
    }

    #[test]
    fn indefinite_arrays_encode_as_definite() {
        let datum: Metadatum = minicbor::decode(&hex::decode("9f00ff").unwrap()).unwrap();
        assert_eq!(datum, Metadatum::Array(vec![Metadatum::Int(Int::from(0))]));
        assert_eq!(
            minicbor::to_vec(&datum).unwrap(),
            hex::decode("8100").unwrap()
        );
    }

    #[test]
    fn rejects_dangling_keys_and_foreign_types() {
        for hex in ["bf00ff", "a100", "f6", "f5", "c249010000000000000000"] {
            let bytes = hex::decode(hex).unwrap();
            assert!(minicbor::decode::<Metadatum>(&bytes).is_err(), "{hex}");
        }
    }
}
