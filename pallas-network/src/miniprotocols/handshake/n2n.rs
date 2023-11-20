use std::collections::HashMap;

use pallas_codec::minicbor::{decode, encode, Decode, Decoder, Encode, Encoder};

pub type VersionTable = super::protocol::VersionTable<VersionData>;

const PROTOCOL_V4: u64 = 4;
const PROTOCOL_V5: u64 = 5;
const PROTOCOL_V6: u64 = 6;
const PROTOCOL_V7: u64 = 7;
const PROTOCOL_V8: u64 = 8;
const PROTOCOL_V9: u64 = 9;
const PROTOCOL_V10: u64 = 10;

impl VersionTable {
    pub fn v4_and_above(network_magic: u64) -> VersionTable {
        let values = vec![
            (PROTOCOL_V4, VersionData::new(network_magic, false)),
            (PROTOCOL_V5, VersionData::new(network_magic, false)),
            (PROTOCOL_V6, VersionData::new(network_magic, false)),
            (PROTOCOL_V7, VersionData::new(network_magic, false)),
            (PROTOCOL_V8, VersionData::new(network_magic, false)),
            (PROTOCOL_V9, VersionData::new(network_magic, false)),
            (PROTOCOL_V10, VersionData::new(network_magic, false)),
        ]
        .into_iter()
        .collect::<HashMap<u64, VersionData>>();

        VersionTable { values }
    }

    pub fn v6_and_above(network_magic: u64) -> VersionTable {
        let values = vec![
            (PROTOCOL_V6, VersionData::new(network_magic, false)),
            (PROTOCOL_V7, VersionData::new(network_magic, false)),
            (PROTOCOL_V8, VersionData::new(network_magic, false)),
            (PROTOCOL_V9, VersionData::new(network_magic, false)),
            (PROTOCOL_V10, VersionData::new(network_magic, false)),
        ]
        .into_iter()
        .collect::<HashMap<u64, VersionData>>();

        VersionTable { values }
    }

    pub fn v7_and_above(network_magic: u64) -> VersionTable {
        let values = vec![
            (PROTOCOL_V7, VersionData::new(network_magic, false)),
            (PROTOCOL_V8, VersionData::new(network_magic, false)),
            (PROTOCOL_V9, VersionData::new(network_magic, false)),
            (PROTOCOL_V10, VersionData::new(network_magic, false)),
        ]
        .into_iter()
        .collect::<HashMap<u64, VersionData>>();

        VersionTable { values }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VersionData {
    network_magic: u64,
    initiator_and_responder_diffusion_mode: bool,
}

impl VersionData {
    pub fn new(network_magic: u64, initiator_and_responder_diffusion_mode: bool) -> Self {
        VersionData {
            network_magic,
            initiator_and_responder_diffusion_mode,
        }
    }
}

impl Encode<()> for VersionData {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        e.array(2)?
            .u64(self.network_magic)?
            .bool(self.initiator_and_responder_diffusion_mode)?;

        Ok(())
    }
}

impl<'b> Decode<'b, ()> for VersionData {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        d.array()?;
        let network_magic = d.u64()?;
        let initiator_and_responder_diffusion_mode = d.bool()?;

        Ok(Self {
            network_magic,
            initiator_and_responder_diffusion_mode,
        })
    }
}
