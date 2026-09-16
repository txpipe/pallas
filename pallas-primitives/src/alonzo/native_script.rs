//! Native scripts may be arbitrarily deep even within a small transaction.
//! Use heap-backed work lists throughout their CBOR lifecycle: fixing only
//! decoding would leave cloning, encoding or destruction able to overflow.

use pallas_codec::minicbor::{self, Decode, Decoder, Encode, Encoder};
use pallas_codec::tree::{Arity, TreeDecode, decode_tree};

use super::NativeScript;

impl NativeScript {
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

    fn shallow_clone(&self) -> Self {
        match self {
            Self::ScriptPubkey(x) => Self::ScriptPubkey(*x),
            Self::ScriptAll(_) => Self::ScriptAll(Vec::new()),
            Self::ScriptAny(_) => Self::ScriptAny(Vec::new()),
            Self::ScriptNOfK(n, _) => Self::ScriptNOfK(*n, Vec::new()),
            Self::InvalidBefore(x) => Self::InvalidBefore(*x),
            Self::InvalidHereafter(x) => Self::InvalidHereafter(*x),
        }
    }
}

impl Drop for NativeScript {
    fn drop(&mut self) {
        let Some(children) = self.children_mut() else {
            return;
        };
        let mut pending = std::mem::take(children);
        while let Some(mut script) = pending.pop() {
            if let Some(children) = script.children_mut() {
                pending.append(children);
            }
            // `script` now owns no descendants, so its Drop cannot recurse.
        }
    }
}

impl Clone for NativeScript {
    fn clone(&self) -> Self {
        let mut root = self.shallow_clone();
        let mut pending = vec![(self, &mut root)];
        while let Some((source, target)) = pending.pop() {
            if let Some(children) = target.children_mut() {
                children.extend(source.children().iter().map(Self::shallow_clone));
                pending.extend(source.children().iter().zip(children.iter_mut()));
            }
        }
        root
    }
}

impl PartialEq for NativeScript {
    fn eq(&self, other: &Self) -> bool {
        let mut pending = vec![(self, other)];
        while let Some((left, right)) = pending.pop() {
            let equal = match (left, right) {
                (Self::ScriptPubkey(a), Self::ScriptPubkey(b)) => a == b,
                (Self::ScriptAll(_), Self::ScriptAll(_))
                | (Self::ScriptAny(_), Self::ScriptAny(_)) => true,
                (Self::ScriptNOfK(a, _), Self::ScriptNOfK(b, _)) => a == b,
                (Self::InvalidBefore(a), Self::InvalidBefore(b))
                | (Self::InvalidHereafter(a), Self::InvalidHereafter(b)) => a == b,
                _ => false,
            };
            if !equal || left.children().len() != right.children().len() {
                return false;
            }
            pending.extend(left.children().iter().zip(right.children()));
        }
        true
    }
}

// Private wrapper so the tree-decoding builder stays out of the public API.
struct Node(NativeScript);

struct Builder {
    script: NativeScript,
    trailing_fields: u64,
}

impl<'b, C> TreeDecode<'b, C> for Node {
    type Builder = Builder;

    fn begin(
        d: &mut Decoder<'b>,
        ctx: &mut C,
    ) -> Result<(Builder, Arity), minicbor::decode::Error> {
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
        if !(0..=5).contains(&variant) {
            return Err(minicbor::decode::Error::unknown_variant(variant).at(variant_position));
        }
        let required = if variant == 3 { 3 } else { 2 };
        if len < required {
            return Err(minicbor::decode::Error::missing_value((len - 1) as i64).at(position));
        }
        let script = match variant {
            0 => NativeScript::ScriptPubkey(d.decode_with(ctx)?),
            1 => NativeScript::ScriptAll(Vec::new()),
            2 => NativeScript::ScriptAny(Vec::new()),
            3 => NativeScript::ScriptNOfK(d.u32()?, Vec::new()),
            4 => NativeScript::InvalidBefore(d.u64()?),
            5 => NativeScript::InvalidHereafter(d.u64()?),
            _ => unreachable!(),
        };
        let arity = if matches!(variant, 1..=3) {
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

    fn child(builder: &mut Builder, child: Node) -> Result<(), minicbor::decode::Error> {
        builder.script.children_mut().unwrap().push(child.0);
        Ok(())
    }

    fn end(
        builder: Builder,
        d: &mut Decoder<'b>,
        _: &mut C,
    ) -> Result<Node, minicbor::decode::Error> {
        for _ in 0..builder.trailing_fields {
            d.skip()?;
        }
        Ok(Node(builder.script))
    }
}

impl<'b, C> Decode<'b, C> for NativeScript {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        decode_tree::<C, Node>(d, ctx).map(|node| node.0)
    }
}

impl<C> Encode<C> for NativeScript {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        let mut pending = vec![self];
        while let Some(script) = pending.pop() {
            match script {
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
                    e.array(3)?.u8(3)?.u32(*n)?.array(xs.len() as u64)?;
                }
                Self::InvalidBefore(x) => {
                    e.array(2)?.u8(4)?.u64(*x)?;
                }
                Self::InvalidHereafter(x) => {
                    e.array(2)?.u8(5)?.u64(*x)?;
                }
            }
            pending.extend(script.children().iter().rev());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
