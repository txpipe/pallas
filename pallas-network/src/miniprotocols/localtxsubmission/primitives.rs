// Material brought from `pallas-primitives`
// TODO: Refactor in order to avoid repetition.
pub use pallas_codec::utils::KeyValuePairs;
pub use pallas_crypto::hash::Hash;
use crate::miniprotocols::localstate::queries_v16::{Coin, PolicyId, AssetName};

use pallas_codec::minicbor;

pub type Multiasset<A> = KeyValuePairs<PolicyId, KeyValuePairs<AssetName, A>>;

pub type Mint = Multiasset<i64>;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Value {
    Coin(Coin),
    Multiasset(Coin, Multiasset<Coin>),
}

impl<'b, C> minicbor::decode::Decode<'b, C> for Value {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        match d.datatype()? {
            minicbor::data::Type::U8 => Ok(Value::Coin(d.decode_with(ctx)?)),
            minicbor::data::Type::U16 => Ok(Value::Coin(d.decode_with(ctx)?)),
            minicbor::data::Type::U32 => Ok(Value::Coin(d.decode_with(ctx)?)),
            minicbor::data::Type::U64 => Ok(Value::Coin(d.decode_with(ctx)?)),
            minicbor::data::Type::Array => {
                d.array()?;
                let coin = d.decode_with(ctx)?;
                let multiasset = d.decode_with(ctx)?;
                Ok(Value::Multiasset(coin, multiasset))
            }
            _ => Err(minicbor::decode::Error::message(
                "unknown cbor data type for Alonzo Value enum",
            )),
        }
    }
}

impl<C> minicbor::encode::Encode<C> for Value {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        // TODO: check how to deal with uint variants (u32 vs u64)
        match self {
            Value::Coin(coin) => {
                e.encode_with(coin, ctx)?;
            }
            Value::Multiasset(coin, other) => {
                e.array(2)?;
                e.encode_with(coin, ctx)?;
                e.encode_with(other, ctx)?;
            }
        };

        Ok(())
    }
}
