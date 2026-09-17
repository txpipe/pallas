//! Dijkstra's native scripts share Alonzo's wire shape plus one leaf variant,
//! and take the same stack-safe lifecycle from [`crate::native_script`].

use pallas_codec::minicbor::{self, Decoder, Encoder};
use pallas_codec::tree::TreeNode;

use super::NativeScript;
use crate::native_script::{FlatScript, flat_script_impls};

impl TreeNode for NativeScript {
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

impl FlatScript for NativeScript {
    fn field_count(variant: i64) -> Option<u64> {
        match variant {
            0 | 1 | 2 | 4 | 5 | 6 => Some(2),
            3 => Some(3),
            _ => None,
        }
    }

    fn begin<'b, C>(
        variant: i64,
        d: &mut Decoder<'b>,
        ctx: &mut C,
    ) -> Result<Self, minicbor::decode::Error> {
        Ok(match variant {
            0 => Self::ScriptPubkey(d.decode_with(ctx)?),
            1 => Self::ScriptAll(Vec::new()),
            2 => Self::ScriptAny(Vec::new()),
            3 => Self::ScriptNOfK(d.i64()?, Vec::new()),
            4 => Self::InvalidBefore(d.u64()?),
            5 => Self::InvalidHereafter(d.u64()?),
            6 => Self::ScriptRequireGuard(d.decode_with(ctx)?),
            _ => unreachable!("field_count rejects unknown variants"),
        })
    }

    fn write_header<C, W: minicbor::encode::Write>(
        &self,
        e: &mut Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
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
            Self::ScriptRequireGuard(x) => {
                e.array(2)?.u8(6)?.encode_with(x, ctx)?;
            }
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
            Self::ScriptRequireGuard(x) => Self::ScriptRequireGuard(x.clone()),
        }
    }

    fn same_node(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::ScriptPubkey(a), Self::ScriptPubkey(b)) => a == b,
            (Self::ScriptAll(_), Self::ScriptAll(_)) | (Self::ScriptAny(_), Self::ScriptAny(_)) => {
                true
            }
            (Self::ScriptNOfK(a, _), Self::ScriptNOfK(b, _)) => a == b,
            (Self::InvalidBefore(a), Self::InvalidBefore(b))
            | (Self::InvalidHereafter(a), Self::InvalidHereafter(b)) => a == b,
            (Self::ScriptRequireGuard(a), Self::ScriptRequireGuard(b)) => a == b,
            _ => false,
        }
    }
}

flat_script_impls!(NativeScript);

#[cfg(test)]
mod tests;
