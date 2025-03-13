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
    SingleKeyDistribution(StakeholderId),
    BootstrapEraDistribution,
}

impl<'b, C> minicbor::Decode<'b, C> for AddrDistr {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        d.array()?;
        let variant = d.u32()?;

        match variant {
            0 => Ok(AddrDistr::SingleKeyDistribution(d.decode_with(ctx)?)),
            1 => Ok(AddrDistr::BootstrapEraDistribution),
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
            AddrDistr::SingleKeyDistribution(x) => {
                e.array(2)?;
                e.u32(0)?;
                e.encode(x)?;

                Ok(())
            }
            AddrDistr::BootstrapEraDistribution => {
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
    Other(u32),
}

impl<'b, C> minicbor::Decode<'b, C> for AddrType {
    fn decode(
        d: &mut minicbor::Decoder<'b>,
        _ctx: &mut C,
    ) -> Result<Self, minicbor::decode::Error> {
        let variant = d.u32()?;

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
            AddrType::PubKey => e.u32(0)?,
            AddrType::Script => e.u32(1)?,
            AddrType::Redeem => e.u32(2)?,
            AddrType::Other(x) => e.u32(*x)?,
        };

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum AddrAttrProperty {
    AddrDistr(AddrDistr),
    DerivationPath(ByteVec),
    NetworkTag(ByteVec),
}

impl<'b, C> minicbor::Decode<'b, C> for AddrAttrProperty {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let key = d.u8()?;

        match key {
            0 => Ok(AddrAttrProperty::AddrDistr(d.decode_with(ctx)?)),
            1 => Ok(AddrAttrProperty::DerivationPath(d.decode_with(ctx)?)),
            2 => Ok(AddrAttrProperty::NetworkTag(d.decode_with(ctx)?)),
            _ => Err(minicbor::decode::Error::message(
                "unknown tag for address attribute",
            )),
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
                e.u8(0)?;
                e.encode(x)?;

                Ok(())
            }
            AddrAttrProperty::DerivationPath(x) => {
                e.u8(1)?;
                e.encode(x)?;

                Ok(())
            }
            AddrAttrProperty::NetworkTag(b) => {
                e.u8(2)?;
                e.encode(b)?;

                Ok(())
            }
        }
    }
}

#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq, PartialOrd)]
#[cbor(flat)]
pub enum SpendingData {
    #[n(0)]
    PubKey(#[n(0)] ByteVec),
    #[n(1)]
    Script(#[n(0)] ByteVec),
    #[n(2)]
    Redeem(#[n(0)] ByteVec),
}

pub type AddrAttrs = OrderPreservingProperties<AddrAttrProperty>;

#[derive(Debug, Encode, Decode, Clone, PartialEq, Eq, PartialOrd)]
pub struct AddressPayload {
    #[n(0)]
    pub root: AddressId,

    #[n(1)]
    pub attributes: AddrAttrs,

    #[n(2)]
    pub addrtype: AddrType,
}

use cryptoxide::hashing::sha3_256;
impl AddressPayload {
    pub fn hash_address_id(
        addrtype: &AddrType,
        spending_data: &SpendingData,
        attributes: &AddrAttrs,
    ) -> Hash<28> {
        let parts = (addrtype, spending_data, attributes);
        let buf = minicbor::to_vec(parts).unwrap();

        pallas_crypto::hash::Hasher::<224>::hash(&sha3_256(&buf))
    }

    pub fn new(addrtype: AddrType, spending_data: SpendingData, attributes: AddrAttrs) -> Self {
        AddressPayload {
            root: Self::hash_address_id(&addrtype, &spending_data, &attributes),
            attributes,
            addrtype,
        }
    }

    // bootstrap era + no hdpayload address
    pub fn new_redeem(
        pubkey: pallas_crypto::key::ed25519::PublicKey,
        network_tag: Option<Vec<u8>>,
    ) -> Self {
        let spending_data = SpendingData::Redeem(ByteVec::from(Vec::from(pubkey.as_ref())));

        let attributes = match network_tag {
            Some(x) => vec![
                //AddrAttrProperty::DerivationPath(ByteVec::from(vec![])),
                //AddrAttrProperty::AddrDistr(AddrDistr::BootstrapEraDistribution),
                AddrAttrProperty::NetworkTag(x.into()),
            ]
            .into(),
            None => vec![
                //AddrAttrProperty::DerivationPath(ByteVec::from(vec![])),
                //AddrAttrProperty::AddrDistr(AddrDistr::BootstrapEraDistribution),
            ]
            .into(),
        };

        Self::new(AddrType::Redeem, spending_data, attributes)
    }
}

impl From<AddressPayload> for ByronAddress {
    fn from(value: AddressPayload) -> Self {
        ByronAddress::from_decoded(value)
    }
}

/// New type wrapping a Byron address primitive
#[derive(Debug, Encode, Decode, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ByronAddress {
    #[n(0)]
    pub payload: TagWrap<ByteVec, 24>,

    #[n(1)]
    pub crc: u32,
}

const CRC: crc::Crc<u32> = crc::Crc::<u32>::new(&crc::CRC_32_ISO_HDLC);

impl ByronAddress {
    pub fn new(payload: &[u8], crc: u32) -> Self {
        Self {
            payload: TagWrap(ByteVec::from(Vec::from(payload))),
            crc,
        }
    }

    pub fn from_decoded(payload: AddressPayload) -> Self {
        let payload = minicbor::to_vec(payload).unwrap();
        let c = CRC.checksum(&payload);
        ByronAddress::new(&payload, c)
    }

    pub fn from_bytes(value: &[u8]) -> Result<Self, Error> {
        pallas_codec::minicbor::decode(value).map_err(Error::InvalidByronCbor)
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
        pallas_codec::minicbor::to_vec(self).unwrap()
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
        minicbor::decode(&self.payload.0).map_err(Error::InvalidByronCbor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_VECTORS: [&str; 3] = [
        "37btjrVyb4KDXBNC4haBVPCrro8AQPHwvCMp3RFhhSVWwfFmZ6wwzSK6JK1hY6wHNmtrpTf1kdbva8TCneM2YsiXT7mrzT21EacHnPpz5YyUdj64na",
        "DdzFFzCqrht7PQiAhzrn6rNNoADJieTWBt8KeK9BZdUsGyX9ooYD9NpMCTGjQoUKcHN47g8JMXhvKogsGpQHtiQ65fZwiypjrC6d3a4Q",
        "Ae2tdPwUPEZLs4HtbuNey7tK4hTKrwNwYtGqp7bDfCy2WdR3P6735W5Yfpe",
    ];

    #[test]
    fn roundtrip_base58() {
        for vector in TEST_VECTORS {
            let addr = ByronAddress::from_base58(vector).unwrap();
            let ours = addr.to_base58();
            assert_eq!(vector, ours);
        }
    }

    #[test]
    fn roundtrip_cbor() {
        for vector in TEST_VECTORS {
            let addr = ByronAddress::from_base58(vector).unwrap();
            let addr = addr.decode().unwrap();
            let addr = ByronAddress::from_decoded(addr);
            let ours = addr.to_base58();
            assert_eq!(vector, ours);
        }
    }

    #[test]
    fn payload_crc_matches() {
        for vector in TEST_VECTORS {
            let addr = ByronAddress::from_base58(vector).unwrap();
            let crc2 = CRC.checksum(addr.payload.as_ref());
            assert_eq!(crc2, addr.crc);
        }
    }
}
