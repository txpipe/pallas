//! Shared stack-safe machinery for the flat-encoded native script enums.
//!
//! Every era's `NativeScript` has the same wire shape: a definite array of
//! `[variant, fields..]` where the container variants end in a child list,
//! and the tree may be arbitrarily deep within a small transaction. An era
//! supplies its variant table through [`FlatScript`] and takes decode,
//! encode, clone, equality and drop from the drivers here, so no step of a
//! script's lifecycle recurses per nesting level.

use pallas_codec::minicbor::{self, Decoder, Encoder};
use pallas_codec::tree::{
    Arity, TreeDecode, TreeNode, Visit, decode_tree, eq_tree, fold_tree, walk_tree,
};

/// An era's native script variant table.
pub(crate) trait FlatScript: TreeNode {
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

impl<'b, C, S: FlatScript> TreeDecode<'b, C> for Node<S> {
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

pub(crate) fn decode<'b, C, S: FlatScript>(
    d: &mut Decoder<'b>,
    ctx: &mut C,
) -> Result<S, minicbor::decode::Error> {
    decode_tree::<C, Node<S>>(d, ctx).map(|node| node.0)
}

pub(crate) fn encode<C, W: minicbor::encode::Write, S: FlatScript>(
    script: &S,
    e: &mut Encoder<W>,
    ctx: &mut C,
) -> Result<(), minicbor::encode::Error<W::Error>> {
    walk_tree(script, |visit| match visit {
        Visit::Enter(script) => script.write_header(e, ctx),
        Visit::Between(_) | Visit::Exit(_) => Ok(()),
    })
}

pub(crate) fn clone<S: FlatScript>(script: &S) -> S {
    fold_tree(script, S::rebuild)
}

pub(crate) fn eq<S: FlatScript>(left: &S, right: &S) -> bool {
    eq_tree(left, right, S::same_node)
}

/// The trait impls a [`FlatScript`] enum forwards to the drivers above.
macro_rules! flat_script_impls {
    ($script:ty) => {
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

pub(crate) use flat_script_impls;
