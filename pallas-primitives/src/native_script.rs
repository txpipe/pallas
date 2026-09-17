//! Native scripts, shared across eras.
//!
//! Every era's `NativeScript` has the wire shape Shelley gave it: a definite
//! array of `[variant, fields..]` where `ScriptAll`, `ScriptAny` and
//! `ScriptNOfK` end in a child list. Later eras only add leaf variants:
//! Dijkstra's `ScriptRequireGuard`, for example. The tree may be arbitrarily
//! deep within a small transaction, so nothing here recurses per nesting
//! level.
//!
//! An era declares its enum with the six Shelley variants plus its own
//! leaves, and [`impl_native_script!`] gives it the decode, encode, clone,
//! equality and drop implementations, all driven by [`pallas_codec::tree`].

use pallas_codec::minicbor::{self, Decoder, Encoder};
use pallas_codec::tree::{
    Arity, TreeDecode, TreeNode, Visit, decode_tree, eq_tree, fold_tree, walk_tree,
};

/// An era's native script enum, as the drivers below see it.
///
/// Implemented by [`impl_native_script!`], never by hand.
pub(crate) trait NativeScript: TreeNode {
    /// The array length a variant is encoded with, counting its index and a
    /// container's child list, or `None` for an unknown variant.
    fn field_count(variant: i64) -> Option<u64>;

    /// Decode a known variant's fields up to, but excluding, a container's
    /// child list. Containers come back with an empty child list.
    fn begin<'b, C>(
        variant: i64,
        d: &mut Decoder<'b>,
        ctx: &mut C,
    ) -> Result<Self, minicbor::decode::Error>;

    /// Write the array header, the variant index and the fields, with a
    /// container's child-list header last.
    fn write_header<C, W: minicbor::encode::Write>(
        &self,
        e: &mut Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>>;

    /// The node rebuilt over `children`; leaves ignore them.
    fn rebuild(&self, children: Vec<Self>) -> Self;

    /// Compare a node's own data, ignoring children.
    fn same_node(&self, other: &Self) -> bool;
}

// Private wrapper so the tree-decoding builder stays out of the public API.
struct Node<S>(S);

struct Builder<S> {
    script: S,
    trailing_fields: u64,
}

impl<'b, C, S: NativeScript> TreeDecode<'b, C> for Node<S> {
    type Builder = Builder<S>;

    fn begin(
        d: &mut Decoder<'b>,
        ctx: &mut C,
    ) -> Result<(Builder<S>, Arity), minicbor::decode::Error> {
        let position = d.position();
        // Match the former #[cbor(flat)] codec, including its acceptance of
        // extra fields and indefinite child lists (but not indefinite variants).
        let len = d.array()?.ok_or_else(|| {
            minicbor::decode::Error::message("flat enum requires definite-length array")
                .at(position)
        })?;
        if len == 0 {
            return Err(
                minicbor::decode::Error::message("flat enum requires non-empty array").at(position),
            );
        }
        let variant_position = d.position();
        let variant = d.i64()?;
        let required = S::field_count(variant).ok_or_else(|| {
            minicbor::decode::Error::unknown_variant(variant).at(variant_position)
        })?;
        if len < required {
            return Err(minicbor::decode::Error::missing_value((len - 1) as i64).at(position));
        }
        let mut script = S::begin(variant, d, ctx)?;
        let arity = if script.children_mut().is_some() {
            d.array()?.into()
        } else {
            Arity::Leaf
        };
        let builder = Builder {
            script,
            trailing_fields: len - required,
        };
        Ok((builder, arity))
    }

    fn child(builder: &mut Builder<S>, child: Node<S>) -> Result<(), minicbor::decode::Error> {
        builder
            .script
            .children_mut()
            .expect("only containers report children")
            .push(child.0);
        Ok(())
    }

    fn end(
        builder: Builder<S>,
        d: &mut Decoder<'b>,
        _: &mut C,
    ) -> Result<Node<S>, minicbor::decode::Error> {
        for _ in 0..builder.trailing_fields {
            d.skip()?;
        }
        Ok(Node(builder.script))
    }
}

pub(crate) fn decode<'b, C, S: NativeScript>(
    d: &mut Decoder<'b>,
    ctx: &mut C,
) -> Result<S, minicbor::decode::Error> {
    decode_tree::<C, Node<S>>(d, ctx).map(|node| node.0)
}

pub(crate) fn encode<C, W: minicbor::encode::Write, S: NativeScript>(
    script: &S,
    e: &mut Encoder<W>,
    ctx: &mut C,
) -> Result<(), minicbor::encode::Error<W::Error>> {
    walk_tree(script, |visit| match visit {
        Visit::Enter(script) => script.write_header(e, ctx),
        Visit::Between(_) | Visit::Exit(_) => Ok(()),
    })
}

pub(crate) fn clone<S: NativeScript>(script: &S) -> S {
    fold_tree(script, S::rebuild)
}

pub(crate) fn eq<S: NativeScript>(left: &S, right: &S) -> bool {
    eq_tree(left, right, S::same_node)
}

/// Implements an era's `NativeScript` enum: the six Shelley variants, plus
/// any era-specific leaves given as `index => Variant(Type)`.
///
/// ```text
/// impl_native_script!(NativeScript);
/// impl_native_script!(NativeScript { 6 => ScriptRequireGuard(StakeCredential) });
/// ```
macro_rules! impl_native_script {
    ($script:ident $({ $( $index:literal => $leaf:ident($leaf_ty:ty) ),* $(,)? })?) => {
        impl pallas_codec::tree::TreeNode for $script {
            fn children(&self) -> &[Self] {
                match self {
                    Self::ScriptAll(xs) | Self::ScriptAny(xs) | Self::ScriptNOfK(_, xs) => xs,
                    _ => &[],
                }
            }

            fn children_mut(&mut self) -> Option<&mut Vec<Self>> {
                match self {
                    Self::ScriptAll(xs) | Self::ScriptAny(xs) | Self::ScriptNOfK(_, xs) => Some(xs),
                    _ => None,
                }
            }
        }

        impl crate::native_script::NativeScript for $script {
            fn field_count(variant: i64) -> Option<u64> {
                match variant {
                    0 | 1 | 2 | 4 | 5 => Some(2),
                    3 => Some(3),
                    $($( $index => Some(2), )*)?
                    _ => None,
                }
            }

            fn begin<'b, C>(
                variant: i64,
                d: &mut pallas_codec::minicbor::Decoder<'b>,
                ctx: &mut C,
            ) -> Result<Self, pallas_codec::minicbor::decode::Error> {
                Ok(match variant {
                    0 => Self::ScriptPubkey(d.decode_with(ctx)?),
                    1 => Self::ScriptAll(Vec::new()),
                    2 => Self::ScriptAny(Vec::new()),
                    3 => Self::ScriptNOfK(d.i64()?, Vec::new()),
                    4 => Self::InvalidBefore(d.u64()?),
                    5 => Self::InvalidHereafter(d.u64()?),
                    $($( $index => Self::$leaf(d.decode_with::<C, $leaf_ty>(ctx)?), )*)?
                    _ => unreachable!("field_count rejects unknown variants"),
                })
            }

            fn write_header<C, W: pallas_codec::minicbor::encode::Write>(
                &self,
                e: &mut pallas_codec::minicbor::Encoder<W>,
                ctx: &mut C,
            ) -> Result<(), pallas_codec::minicbor::encode::Error<W::Error>> {
                match self {
                    Self::ScriptPubkey(x) => {
                        e.array(2)?.u8(0)?.encode_with(x, ctx)?;
                    }
                    Self::ScriptAll(xs) => {
                        e.array(2)?.u8(1)?.array(xs.len() as u64)?;
                    }
                    Self::ScriptAny(xs) => {
                        e.array(2)?.u8(2)?.array(xs.len() as u64)?;
                    }
                    Self::ScriptNOfK(n, xs) => {
                        e.array(3)?.u8(3)?.i64(*n)?.array(xs.len() as u64)?;
                    }
                    Self::InvalidBefore(x) => {
                        e.array(2)?.u8(4)?.u64(*x)?;
                    }
                    Self::InvalidHereafter(x) => {
                        e.array(2)?.u8(5)?.u64(*x)?;
                    }
                    $($( Self::$leaf(x) => {
                        e.array(2)?.u8($index)?.encode_with(x, ctx)?;
                    } )*)?
                }
                Ok(())
            }

            fn rebuild(&self, children: Vec<Self>) -> Self {
                match self {
                    Self::ScriptPubkey(x) => Self::ScriptPubkey(*x),
                    Self::ScriptAll(_) => Self::ScriptAll(children),
                    Self::ScriptAny(_) => Self::ScriptAny(children),
                    Self::ScriptNOfK(n, _) => Self::ScriptNOfK(*n, children),
                    Self::InvalidBefore(x) => Self::InvalidBefore(*x),
                    Self::InvalidHereafter(x) => Self::InvalidHereafter(*x),
                    $($( Self::$leaf(x) => Self::$leaf(x.clone()), )*)?
                }
            }

            fn same_node(&self, other: &Self) -> bool {
                match (self, other) {
                    (Self::ScriptPubkey(a), Self::ScriptPubkey(b)) => a == b,
                    (Self::ScriptAll(_), Self::ScriptAll(_))
                    | (Self::ScriptAny(_), Self::ScriptAny(_)) => true,
                    (Self::ScriptNOfK(a, _), Self::ScriptNOfK(b, _)) => a == b,
                    (Self::InvalidBefore(a), Self::InvalidBefore(b))
                    | (Self::InvalidHereafter(a), Self::InvalidHereafter(b)) => a == b,
                    $($( (Self::$leaf(a), Self::$leaf(b)) => a == b, )*)?
                    _ => false,
                }
            }
        }

        impl Drop for $script {
            fn drop(&mut self) {
                pallas_codec::tree::drop_children(self);
            }
        }

        impl Clone for $script {
            fn clone(&self) -> Self {
                crate::native_script::clone(self)
            }
        }

        impl PartialEq for $script {
            fn eq(&self, other: &Self) -> bool {
                crate::native_script::eq(self, other)
            }
        }

        impl<'b, C> pallas_codec::minicbor::decode::Decode<'b, C> for $script {
            fn decode(
                d: &mut pallas_codec::minicbor::Decoder<'b>,
                ctx: &mut C,
            ) -> Result<Self, pallas_codec::minicbor::decode::Error> {
                crate::native_script::decode(d, ctx)
            }
        }

        impl<C> pallas_codec::minicbor::encode::Encode<C> for $script {
            fn encode<W: pallas_codec::minicbor::encode::Write>(
                &self,
                e: &mut pallas_codec::minicbor::Encoder<W>,
                ctx: &mut C,
            ) -> Result<(), pallas_codec::minicbor::encode::Error<W::Error>> {
                crate::native_script::encode(self, e, ctx)
            }
        }
    };
}

pub(crate) use impl_native_script;
