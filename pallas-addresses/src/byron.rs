use pallas_codec::{
    minicbor::{self, bytes::ByteVec, Decode, Encode},
    utils::{OrderPreservingProperties, TagWrap},
};

use pallas_crypto::hash::Hash;

use crate::Error;

pub type Blake2b224 = Hash<28>;

pub type AddressId = Blake2b224;
pub type StakeholderId = Blake2b224;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
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

#[derive(Debug, Encode, Decode, Clone, PartialEq, Eq, PartialOrd)]
pub struct AddressPayload {
    #[n(0)]
    pub root: AddressId,

    #[n(1)]
    pub attributes: AddrAttr,

    #[n(2)]
    pub addrtype: AddrType,
}

/// New type wrapping a Byron address primitive
#[derive(Debug, Encode, Decode, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ByronAddress {
    #[n(0)]
    payload: TagWrap<ByteVec, 24>,

    #[n(1)]
    crc: u64,
}

impl ByronAddress {
    pub fn new(payload: &[u8], crc: u64) -> Self {
        Self {
            payload: TagWrap(ByteVec::from(Vec::from(payload))),
            crc,
        }
    }

    pub fn from_bytes(value: &[u8]) -> Result<Self, Error> {
        pallas_codec::minicbor::decode(value).map_err(|_| Error::InvalidByronCbor)
    }

    // Tries to decode an address from its hex representation
    pub fn from_base58(value: &str) -> Result<Self, Error> {
        let bytes = base58::FromBase58::from_base58(value).map_err(Error::BadBase58)?;
        Self::from_bytes(&bytes)
    }

    /// Gets a numeric id describing the type of the address
    pub fn typeid(&self) -> u8 {
        0b1000
    }

    pub fn to_vec(&self) -> Vec<u8> {
        pallas_codec::minicbor::to_vec(&self).unwrap()
    }

    pub fn to_base58(&self) -> String {
        let bytes = self.to_vec();
        base58::ToBase58::to_base58(bytes.as_slice())
    }

    pub fn to_hex(&self) -> String {
        let bytes = self.to_vec();
        hex::encode(bytes)
    }

    pub fn decode(&self) -> Result<AddressPayload, Error> {
        minicbor::decode(&self.payload.0).map_err(|_| Error::InvalidByronCbor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_VECTOR: &str = "37btjrVyb4KDXBNC4haBVPCrro8AQPHwvCMp3RFhhSVWwfFmZ6wwzSK6JK1hY6wHNmtrpTf1kdbva8TCneM2YsiXT7mrzT21EacHnPpz5YyUdj64na";

    const ROOT_HASH: &str = "7e9ee4a9527dea9091e2d580edd6716888c42f75d96276290f98fe0b";

    #[test]
    fn roundtrip_base58() {
        let addr = ByronAddress::from_base58(TEST_VECTOR).unwrap();
        let ours = addr.to_base58();
        assert_eq!(TEST_VECTOR, ours);
    }

    #[test]
    fn payload_matches() {
        let addr = ByronAddress::from_base58(TEST_VECTOR).unwrap();
        let payload = addr.decode().unwrap();
        assert_eq!(payload.root.to_string(), ROOT_HASH);
    }
}
