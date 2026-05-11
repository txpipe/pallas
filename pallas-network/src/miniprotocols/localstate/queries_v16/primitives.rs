// Material brought from `pallas-primitives`
// TODO: Refactor in order to avoid repetition.
pub use pallas_codec::utils::{Bytes, Nullable};
pub use pallas_crypto::hash::Hash;

use pallas_codec::minicbor::{self, Decode, Encode};

/// Stake pool metadata reference: URL plus the hash of the pointed-to JSON.
#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub struct PoolMetadata {
    /// URL serving the pool metadata JSON.
    #[n(0)]
    pub url: String,

    /// Hash of the JSON document served at `url`.
    #[n(1)]
    pub hash: PoolMetadataHash,
}

/// Hash of stake pool metadata (Blake2b-256).
pub type PoolMetadataHash = Bytes;

/// TCP/UDP port number.
pub type Port = u32;

/// IPv4 address bytes (4 bytes, big-endian).
pub type IPv4 = Bytes;

/// IPv6 address bytes (16 bytes, big-endian).
pub type IPv6 = Bytes;

/// DNS name (A or SRV record) used in relay declarations.
pub type DnsName = String;

/// Network endpoint declared by a stake pool's relay.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Relay {
    /// IP-based relay with optional port and either or both IPv4/IPv6.
    SingleHostAddr(Nullable<Port>, Nullable<IPv4>, Nullable<IPv6>),
    /// DNS A-record relay with optional port and a hostname.
    SingleHostName(Nullable<Port>, DnsName),
    /// DNS SRV-record relay (port and host both come from the SRV record).
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
