// Material brought from `pallas-primitives`
// TODO: Refactor in order to avoid repetition.
pub use pallas_codec::utils::{Bytes, Nullable};
pub use pallas_crypto::hash::Hash;

use pallas_codec::minicbor::{self, Decode, Encode};

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub struct PoolMetadata {
    #[n(0)]
    pub url: String,

    #[n(1)]
    pub hash: PoolMetadataHash,
}

pub type PoolMetadataHash = Hash<32>;

pub type Port = u32;

pub type IPv4 = Bytes;

pub type IPv6 = Bytes;

pub type DnsName = String;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Relay {
    SingleHostAddr(Nullable<Port>, Nullable<IPv4>, Nullable<IPv6>),
    SingleHostName(Nullable<Port>, DnsName),
    MultiHostName(DnsName),
}

// Move to `codec.rs`?
impl<'b, C> minicbor::decode::Decode<'b, C> for Relay {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        d.array()?;
        let variant = d.u16()?;

        match variant {
            0 => Ok(Relay::SingleHostAddr(
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
            )),
            1 => Ok(Relay::SingleHostName(
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
            )),
            2 => Ok(Relay::MultiHostName(d.decode_with(ctx)?)),
            _ => Err(minicbor::decode::Error::message(
                "invalid variant id for Relay",
            )),
        }
    }
}

impl<C> minicbor::encode::Encode<C> for Relay {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            Relay::SingleHostAddr(a, b, c) => {
                e.array(4)?;
                e.encode_with(0, ctx)?;
                e.encode_with(a, ctx)?;
                e.encode_with(b, ctx)?;
                e.encode_with(c, ctx)?;

                Ok(())
            }
            Relay::SingleHostName(a, b) => {
                e.array(3)?;
                e.encode_with(1, ctx)?;
                e.encode_with(a, ctx)?;
                e.encode_with(b, ctx)?;

                Ok(())
            }
            Relay::MultiHostName(a) => {
                e.array(2)?;
                e.encode_with(2, ctx)?;
                e.encode_with(a, ctx)?;

                Ok(())
            }
        }
    }
}
