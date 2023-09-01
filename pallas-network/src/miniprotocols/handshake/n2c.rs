use std::collections::HashMap;

use pallas_codec::minicbor::data::Type;
use pallas_codec::minicbor::{decode, encode, Decode, Decoder, Encode, Encoder};

use super::protocol::NetworkMagic;

pub type VersionTable = super::protocol::VersionTable<VersionData>;

const PROTOCOL_V1: u64 = 1;
const PROTOCOL_V2: u64 = 32770;
const PROTOCOL_V3: u64 = 32771;
const PROTOCOL_V4: u64 = 32772;
const PROTOCOL_V5: u64 = 32773;
const PROTOCOL_V6: u64 = 32774;
const PROTOCOL_V7: u64 = 32775;
const PROTOCOL_V8: u64 = 32776;
const PROTOCOL_V9: u64 = 32777;
const PROTOCOL_V10: u64 = 32778;
const PROTOCOL_V11: u64 = 32779;
const PROTOCOL_V12: u64 = 32780;
const PROTOCOL_V13: u64 = 32781;
const PROTOCOL_V14: u64 = 32782;
const PROTOCOL_V15: u64 = 32783;

impl VersionTable {
    pub fn v1_and_above(network_magic: u64) -> VersionTable {
        let values = vec![
            (PROTOCOL_V1, VersionData(network_magic, None)),
            (PROTOCOL_V2, VersionData(network_magic, None)),
            (PROTOCOL_V3, VersionData(network_magic, None)),
            (PROTOCOL_V4, VersionData(network_magic, None)),
            (PROTOCOL_V5, VersionData(network_magic, None)),
            (PROTOCOL_V6, VersionData(network_magic, None)),
            (PROTOCOL_V7, VersionData(network_magic, None)),
            (PROTOCOL_V8, VersionData(network_magic, None)),
            (PROTOCOL_V9, VersionData(network_magic, None)),
            (PROTOCOL_V10, VersionData(network_magic, None)),
            (PROTOCOL_V11, VersionData(network_magic, None)),
            (PROTOCOL_V12, VersionData(network_magic, None)),
            (PROTOCOL_V13, VersionData(network_magic, None)),
            (PROTOCOL_V14, VersionData(network_magic, None)),
            (PROTOCOL_V15, VersionData(network_magic, Some(false))),
        ]
        .into_iter()
        .collect::<HashMap<u64, VersionData>>();

        VersionTable { values }
    }

    pub fn only_v10(network_magic: u64) -> VersionTable {
        let values = vec![(PROTOCOL_V10, VersionData(network_magic, None))]
            .into_iter()
            .collect::<HashMap<u64, VersionData>>();

        VersionTable { values }
    }

    pub fn v10_and_above(network_magic: u64) -> VersionTable {
        let values = vec![
            (PROTOCOL_V10, VersionData(network_magic, None)),
            (PROTOCOL_V11, VersionData(network_magic, None)),
            (PROTOCOL_V12, VersionData(network_magic, None)),
            (PROTOCOL_V13, VersionData(network_magic, None)),
            (PROTOCOL_V14, VersionData(network_magic, None)),
            (PROTOCOL_V15, VersionData(network_magic, Some(false))),
        ]
        .into_iter()
        .collect::<HashMap<u64, VersionData>>();

        VersionTable { values }
    }

    pub fn v15_with_query(network_magic: u64) -> VersionTable {
        let values = vec![(PROTOCOL_V15, VersionData(network_magic, Some(true)))]
            .into_iter()
            .collect::<HashMap<u64, VersionData>>();

        VersionTable { values }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VersionData(NetworkMagic, Option<bool>);

impl Encode<()> for VersionData {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        match self.1 {
            None => {
                e.u64(self.0)?;
            }
            Some(is_query) => {
                e.array(2)?;
                e.u64(self.0)?;
                e.bool(is_query)?;
            }
        }

        Ok(())
    }
}

impl<'b> Decode<'b, ()> for VersionData {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        match d.datatype()? {
            Type::U8 | Type::U16 | Type::U32 | Type::U64 => {
                let network_magic = d.u64()?;
                Ok(Self(network_magic, None))
            }
            Type::Array => {
                d.array()?;
                let network_magic = d.u64()?;
                let is_query = d.bool()?;
                Ok(Self(network_magic, Some(is_query)))
            }
            _ => Err(decode::Error::message("unknown type for VersionData")),
        }
    }
}
