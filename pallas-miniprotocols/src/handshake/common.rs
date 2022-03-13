use itertools::Itertools;
use pallas_codec::minicbor::{decode, encode, Decode, Decoder, Encode, Encoder};
use std::{collections::HashMap, fmt::Debug};

#[derive(Debug, Clone)]
pub struct VersionTable<T>
where
    T: Debug + Clone,
{
    pub values: HashMap<u64, T>,
}

impl<'b, T> Encode for VersionTable<T>
where
    T: Debug + Clone + Encode + Decode<'b>,
{
    fn encode<W: encode::Write>(&self, e: &mut Encoder<W>) -> Result<(), encode::Error<W::Error>> {
        e.map(self.values.len() as u64)?;

        for key in self.values.keys().sorted() {
            e.u64(*key)?;
            self.values[key].encode(e)?;
        }

        Ok(())
    }
}

pub type NetworkMagic = u64;

pub type VersionNumber = u64;

#[derive(Debug)]
pub enum RefuseReason {
    VersionMismatch(Vec<VersionNumber>),
    HandshakeDecodeError(VersionNumber, String),
    Refused(VersionNumber, String),
}

impl Encode for RefuseReason {
    fn encode<W: encode::Write>(&self, e: &mut Encoder<W>) -> Result<(), encode::Error<W::Error>> {
        match self {
            RefuseReason::VersionMismatch(versions) => {
                e.array(2)?;
                e.u16(0)?;
                e.array(versions.len() as u64)?;
                for v in versions.iter() {
                    e.u64(*v)?;
                }

                Ok(())
            }
            RefuseReason::HandshakeDecodeError(version, msg) => {
                e.array(3)?;
                e.u16(1)?;
                e.u64(*version)?;
                e.str(msg)?;

                Ok(())
            }
            RefuseReason::Refused(version, msg) => {
                e.array(3)?;
                e.u16(2)?;
                e.u64(*version)?;
                e.str(msg)?;

                Ok(())
            }
        }
    }
}

impl<'b> Decode<'b> for RefuseReason {
    fn decode(d: &mut Decoder<'b>) -> Result<Self, decode::Error> {
        d.array()?;

        match d.u16()? {
            0 => {
                let versions = d.array_iter::<u64>()?;
                let versions = versions.try_collect()?;
                Ok(RefuseReason::VersionMismatch(versions))
            }
            1 => {
                let version = d.u64()?;
                let msg = d.str()?;

                Ok(RefuseReason::HandshakeDecodeError(version, msg.to_string()))
            }
            2 => {
                let version = d.u64()?;
                let msg = d.str()?;

                Ok(RefuseReason::Refused(version, msg.to_string()))
            }
            _ => Err(decode::Error::message("unknown variant for refusereason")),
        }
    }
}
