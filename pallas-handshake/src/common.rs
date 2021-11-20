use itertools::Itertools;
use pallas_machines::{DecodePayload, EncodePayload, PayloadEncoder};
use std::{collections::HashMap, fmt::Debug};

pub const TESTNET_MAGIC: u64 = 1097911063;
pub const MAINNET_MAGIC: u64 = 764824073;

#[derive(Debug, Clone)]
pub struct VersionTable<T>
where
    T: Debug + Clone + EncodePayload + DecodePayload,
{
    pub values: HashMap<u64, T>,
}

impl<T> EncodePayload for VersionTable<T>
where
    T: Debug + Clone + EncodePayload + DecodePayload,
{
    fn encode_payload(
        &self,
        e: &mut PayloadEncoder,
    ) -> Result<(), Box<dyn std::error::Error>> {
        e.map(self.values.len() as u64)?;

        for key in self.values.keys().sorted() {
            e.u64(*key)?;
            self.values[key].encode_payload(e)?;
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

impl EncodePayload for RefuseReason {
    fn encode_payload(
        &self,
        e: &mut PayloadEncoder,
    ) -> Result<(), Box<dyn std::error::Error>> {
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
                e.u16(1)?;
                e.u64(*version)?;
                e.str(msg)?;

                Ok(())
            }
        }
    }
}
