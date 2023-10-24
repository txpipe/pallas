use serde::{Deserialize, Serialize};
use pallas_codec::minicbor;
use pallas_crypto::hash::Hash;
use crate::alonzo::{AddrKeyhash, Scripthash, StakeCredential};

#[derive(Serialize, Deserialize, Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
pub enum DRep {
    Key(AddrKeyhash),
    Script(Scripthash),
    Abstain,
    NoConfidence,
}

impl<'b, C> minicbor::decode::Decode<'b, C> for DRep {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        d.array()?;
        let variant = d.u16()?;

        match variant {
            0 => Ok(DRep::Key(d.decode_with(ctx)?)),
            1 => Ok(DRep::Script(d.decode_with(ctx)?)),
            2 => Ok(DRep::Abstain),
            3 => Ok(DRep::NoConfidence),
            _ => Err(minicbor::decode::Error::message(
                "invalid variant id for DRep",
            )),
        }
    }
}

impl<C> minicbor::encode::Encode<C> for DRep {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            DRep::Key(h) => {
                e.array(2)?;
                e.encode_with(0, ctx)?;
                e.encode_with(h, ctx)?;

                Ok(())
            }
            DRep::Script(h) => {
                e.array(2)?;
                e.encode_with(1, ctx)?;
                e.encode_with(h, ctx)?;

                Ok(())
            }
            DRep::Abstain => {
                e.array(1)?;
                e.encode_with(2, ctx)?;

                Ok(())
            }
            DRep::NoConfidence => {
                e.array(1)?;
                e.encode_with(3, ctx)?;

                Ok(())
            }
        }
    }
}

pub type DRepCredential = StakeCredential;

pub type CommitteeColdCredential = StakeCredential;

pub type CommitteeHotCredential = StakeCredential;

#[derive(Serialize, Deserialize, Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
pub struct Anchor(pub String, pub Hash<32>);

impl<'b, C> minicbor::Decode<'b, C> for Anchor {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        d.array()?;

        Ok(Self(d.decode_with(ctx)?, d.decode_with(ctx)?))
    }
}

impl<C> minicbor::Encode<C> for Anchor {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.array(2)?;

        e.encode_with(&self.0, ctx)?;
        e.encode_with(&self.1, ctx)?;

        Ok(())
    }
}