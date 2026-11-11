//! Stack-safe script trees used in node-to-client query results and errors.
use super::primitives::NativeScript as Script;
use pallas_codec::{
    minicbor::{self, Decode, Decoder, Encode, Encoder},
    tree::{
        Arity, MAX_DECODE_DEPTH, TreeDecode, TreeNode, Visit, decode_tree_with_max_depth, eq_tree,
        fold_tree, walk_tree,
    },
};

impl TreeNode for Script {
    fn children(&self) -> &[Self] {
        match self {
            Self::ScriptAll(x) | Self::ScriptAny(x) | Self::ScriptNOfK(_, x) => x,
            _ => &[],
        }
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Self>> {
        match self {
            Self::ScriptAll(x) | Self::ScriptAny(x) | Self::ScriptNOfK(_, x) => Some(x),
            _ => None,
        }
    }
}
struct Node(Script);
impl<'b, C> TreeDecode<'b, C> for Node {
    type Builder = Script;
    fn begin(d: &mut Decoder<'b>, ctx: &mut C) -> Result<(Script, Arity), minicbor::decode::Error> {
        // Preserve this codec's existing outer-array handling. Child arrays
        // may be definite or indefinite, just as with Vec::decode.
        d.array()?;
        let script = match d.u32()? {
            0 => Script::ScriptPubkey(d.decode_with(ctx)?),
            1 => Script::ScriptAll(Vec::new()),
            2 => Script::ScriptAny(Vec::new()),
            3 => Script::ScriptNOfK(d.decode_with(ctx)?, Vec::new()),
            4 => Script::InvalidBefore(d.decode_with(ctx)?),
            5 => Script::InvalidHereafter(d.decode_with(ctx)?),
            _ => {
                return Err(minicbor::decode::Error::message(
                    "unknown variant id for native script",
                ));
            }
        };
        let arity = if matches!(
            script,
            Script::ScriptAll(_) | Script::ScriptAny(_) | Script::ScriptNOfK(_, _)
        ) {
            d.array()?.into()
        } else {
            Arity::Leaf
        };
        Ok((script, arity))
    }
    fn child(node: &mut Script, child: Node) -> Result<(), minicbor::decode::Error> {
        node.children_mut().expect("container").push(child.0);
        Ok(())
    }
    fn end(node: Script, _: &mut Decoder<'b>, _: &mut C) -> Result<Node, minicbor::decode::Error> {
        Ok(Node(node))
    }
}
impl Script {
    /// Extracts an all-of child list without cloning; returns other variants unchanged.
    pub fn into_all(mut self) -> Result<Vec<Self>, Self> {
        match &mut self {
            Self::ScriptAll(xs) => Ok(std::mem::take(xs)),
            _ => Err(self),
        }
    }
    /// Extracts an any-of child list without cloning; returns other variants unchanged.
    pub fn into_any(mut self) -> Result<Vec<Self>, Self> {
        match &mut self {
            Self::ScriptAny(xs) => Ok(std::mem::take(xs)),
            _ => Err(self),
        }
    }
    /// Extracts a threshold and its children without cloning; returns other variants unchanged.
    pub fn into_n_of_k(mut self) -> Result<(i64, Vec<Self>), Self> {
        match &mut self {
            Self::ScriptNOfK(n, xs) => Ok((*n, std::mem::take(xs))),
            _ => Err(self),
        }
    }
}
impl<'b, C> Decode<'b, C> for Script {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        decode_tree_with_max_depth::<C, Node>(d, ctx, MAX_DECODE_DEPTH).map(|node| node.0)
    }
}
impl<C> Encode<C> for Script {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        walk_tree(self, |visit| {
            if let Visit::Enter(node) = visit {
                match node {
                    Self::ScriptPubkey(x) => {
                        e.array(2)?.u32(0)?.encode_with(x, ctx)?;
                    }
                    Self::ScriptAll(xs) => {
                        e.array(2)?.u32(1)?.array(xs.len() as u64)?;
                    }
                    Self::ScriptAny(xs) => {
                        e.array(2)?.u32(2)?.array(xs.len() as u64)?;
                    }
                    Self::ScriptNOfK(n, xs) => {
                        e.array(3)?
                            .u32(3)?
                            .encode_with(n, ctx)?
                            .array(xs.len() as u64)?;
                    }
                    Self::InvalidBefore(x) => {
                        e.array(2)?.u32(4)?.encode_with(x, ctx)?;
                    }
                    Self::InvalidHereafter(x) => {
                        e.array(2)?.u32(5)?.encode_with(x, ctx)?;
                    }
                }
            }
            Ok(())
        })
    }
}
impl Clone for Script {
    fn clone(&self) -> Self {
        fold_tree(self, |node, children| match node {
            Self::ScriptPubkey(x) => Self::ScriptPubkey(*x),
            Self::ScriptAll(_) => Self::ScriptAll(children),
            Self::ScriptAny(_) => Self::ScriptAny(children),
            Self::ScriptNOfK(n, _) => Self::ScriptNOfK(*n, children),
            Self::InvalidBefore(x) => Self::InvalidBefore(*x),
            Self::InvalidHereafter(x) => Self::InvalidHereafter(*x),
        })
    }
}
impl PartialEq for Script {
    fn eq(&self, other: &Self) -> bool {
        eq_tree(self, other, |a, b| match (a, b) {
            (Self::ScriptPubkey(a), Self::ScriptPubkey(b)) => a == b,
            (Self::ScriptAll(_), Self::ScriptAll(_)) | (Self::ScriptAny(_), Self::ScriptAny(_)) => {
                true
            }
            (Self::ScriptNOfK(a, _), Self::ScriptNOfK(b, _)) => a == b,
            (Self::InvalidBefore(a), Self::InvalidBefore(b))
            | (Self::InvalidHereafter(a), Self::InvalidHereafter(b)) => a == b,
            _ => false,
        })
    }
}
impl Eq for Script {}
