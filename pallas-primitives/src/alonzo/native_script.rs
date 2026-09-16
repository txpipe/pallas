//! Native scripts may be arbitrarily deep even within a small transaction.
//! Use heap-backed work lists throughout their CBOR lifecycle: fixing only
//! decoding would leave cloning, encoding or destruction able to overflow.

use pallas_codec::minicbor::{self, Decode, Decoder, Encode, Encoder, data::Type};

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

struct Frame {
    script: NativeScript,
    // None: leaf; Some(None): indefinite child array; Some(Some(n)): n children left.
    children_left: Option<Option<u64>>,
    trailing_fields: u64,
}

impl Frame {
    fn decode<'b, C>(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
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
        let children_left = if matches!(variant, 1..=3) {
            Some(d.array()?)
        } else {
            None
        };
        // Do not reserve from an untrusted CBOR length. Memory grows only as
        // actual nodes are consumed from the input.
        Ok(Self {
            script,
            children_left,
            trailing_fields: len - required,
        })
    }

    fn needs_child(&mut self, d: &mut Decoder<'_>) -> Result<bool, minicbor::decode::Error> {
        match self.children_left {
            None | Some(Some(0)) => Ok(false),
            Some(Some(_)) => Ok(true),
            Some(None) if d.datatype()? == Type::Break => {
                d.skip()?;
                self.children_left = None;
                Ok(false)
            }
            Some(None) => Ok(true),
        }
    }
}

impl<'b, C> Decode<'b, C> for NativeScript {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let mut parents: Vec<Frame> = Vec::new();
        let mut current = Frame::decode(d, ctx)?;
        loop {
            if current.needs_child(d)? {
                parents.push(current);
                current = Frame::decode(d, ctx)?;
                continue;
            }
            for _ in 0..current.trailing_fields {
                d.skip()?;
            }
            let Some(mut parent) = parents.pop() else {
                return Ok(current.script);
            };
            parent.script.children_mut().unwrap().push(current.script);
            if let Some(Some(remaining)) = &mut parent.children_left {
                *remaining -= 1;
            }
            current = parent;
        }
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
