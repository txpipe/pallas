use pallas_codec::{
    minicbor::{self, bytes::ByteVec, Decode, Encode},
    utils::OrderPreservingProperties,
};

use pallas_crypto::hash::Hash;

pub type Blake2b224 = Hash<28>;

pub type AddressId = Blake2b224;
pub type StakeholderId = Blake2b224;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum AddrDistr {
    Variant0(StakeholderId),
    Variant1,
}

impl<'b, C> minicbor::Decode<'b, C> for AddrDistr {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        d.array()?;
        let variant = d.u32()?;

        match variant {
            0 => Ok(AddrDistr::Variant0(d.decode_with(ctx)?)),
            1 => Ok(AddrDistr::Variant1),
            _ => Err(minicbor::decode::Error::message(
                "invalid variant for addrdstr",
            )),
        }
    }
}

impl minicbor::Encode<()> for AddrDistr {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            AddrDistr::Variant0(x) => {
                e.array(2)?;
                e.u32(0)?;
                e.encode(x)?;

                Ok(())
            }
            AddrDistr::Variant1 => {
                e.array(1)?;
                e.u32(1)?;

                Ok(())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum AddrType {
    PubKey,
    Script,
    Redeem,
    Other(u64),
}

impl<'b, C> minicbor::Decode<'b, C> for AddrType {
    fn decode(
        d: &mut minicbor::Decoder<'b>,
        _ctx: &mut C,
    ) -> Result<Self, minicbor::decode::Error> {
        let variant = d.u64()?;

        match variant {
            0 => Ok(AddrType::PubKey),
            1 => Ok(AddrType::Script),
            2 => Ok(AddrType::Redeem),
            x => Ok(AddrType::Other(x)),
        }
    }
}

impl<C> minicbor::Encode<C> for AddrType {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        _ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            AddrType::PubKey => e.u64(0)?,
            AddrType::Script => e.u64(1)?,
            AddrType::Redeem => e.u64(2)?,
            AddrType::Other(x) => e.u64(*x)?,
        };

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum AddrAttrProperty {
    AddrDistr(AddrDistr),
    Bytes(ByteVec),
    Unparsed(u8, ByteVec),
}

impl<'b, C> minicbor::Decode<'b, C> for AddrAttrProperty {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let key = d.u8()?;

        match key {
            0 => Ok(AddrAttrProperty::AddrDistr(d.decode_with(ctx)?)),
            1 => Ok(AddrAttrProperty::Bytes(d.decode_with(ctx)?)),
            x => Ok(AddrAttrProperty::Unparsed(x, d.decode_with(ctx)?)),
        }
    }
}

impl<C> minicbor::Encode<C> for AddrAttrProperty {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        _ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            AddrAttrProperty::AddrDistr(x) => {
                e.u32(0)?;
                e.encode(x)?;

                Ok(())
            }
            AddrAttrProperty::Bytes(x) => {
                e.u32(1)?;
                e.encode(x)?;

                Ok(())
            }
            AddrAttrProperty::Unparsed(a, b) => {
                e.encode(a)?;
                e.encode(b)?;

                Ok(())
            }
        }
    }
}

pub type AddrAttr = OrderPreservingProperties<AddrAttrProperty>;

#[derive(Debug, Encode, Decode, Clone, PartialEq, PartialOrd)]
pub struct AddressPayload {
    #[n(0)]
    pub root: AddressId,

    #[n(1)]
    pub attributes: AddrAttr,

    #[n(2)]
    pub addrtype: AddrType,
}
