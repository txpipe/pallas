use std::collections::HashMap;

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

impl VersionTable {
    pub fn v1_and_above(network_magic: u64) -> VersionTable {
        let values = vec![
            (PROTOCOL_V1, VersionData(network_magic)),
            (PROTOCOL_V2, VersionData(network_magic)),
            (PROTOCOL_V3, VersionData(network_magic)),
            (PROTOCOL_V4, VersionData(network_magic)),
            (PROTOCOL_V5, VersionData(network_magic)),
            (PROTOCOL_V6, VersionData(network_magic)),
            (PROTOCOL_V7, VersionData(network_magic)),
            (PROTOCOL_V8, VersionData(network_magic)),
            (PROTOCOL_V9, VersionData(network_magic)),
            (PROTOCOL_V10, VersionData(network_magic)),
        ]
        .into_iter()
        .collect::<HashMap<u64, VersionData>>();

        VersionTable { values }
    }

    pub fn only_v10(network_magic: u64) -> VersionTable {
        let values = vec![(PROTOCOL_V10, VersionData(network_magic))]
            .into_iter()
            .collect::<HashMap<u64, VersionData>>();

        VersionTable { values }
    }
}

#[derive(Debug, Clone)]
pub struct VersionData(NetworkMagic);

impl Encode<()> for VersionData {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        e.u64(self.0)?;

        Ok(())
    }
}

impl<'b> Decode<'b, ()> for VersionData {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        let network_magic = d.u64()?;

        Ok(Self(network_magic))
    }
}
