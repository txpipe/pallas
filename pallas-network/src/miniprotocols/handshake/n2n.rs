use std::collections::HashMap;

use pallas_codec::minicbor::{decode, Decode, Decoder, encode, Encode, Encoder};

pub type VersionTable = super::protocol::VersionTable<VersionData>;

const PROTOCOL_V7: u64 = 7;
const PROTOCOL_V8: u64 = 8;
const PROTOCOL_V9: u64 = 9;
const PROTOCOL_V10: u64 = 10;
const PROTOCOL_V11: u64 = 11;
const PROTOCOL_V12: u64 = 12;
const PROTOCOL_V13: u64 = 13;

const PEER_SHARING_NONE: u8 = 0;

impl VersionTable {
    pub fn v4_and_above(network_magic: u64) -> VersionTable {
        // Older versions are not supported anymore (removed from network-spec.pdf).
        // Try not to break compatibility with older pallas users.
        return Self::v7_and_above(network_magic);
    }

    pub fn v6_and_above(network_magic: u64) -> VersionTable {
        // Older versions are not supported anymore (removed from network-spec.pdf).
        // Try not to break compatibility with older pallas users.
        return Self::v7_and_above(network_magic);
    }

    pub fn v7_to_v10(network_magic: u64) -> VersionTable {
        let values = vec![
            (PROTOCOL_V7, VersionData::new(network_magic, false, None, None)),
            (PROTOCOL_V8, VersionData::new(network_magic, false, None, None)),
            (PROTOCOL_V9, VersionData::new(network_magic, false, None, None)),
            (PROTOCOL_V10, VersionData::new(network_magic, false, None, None)),
        ]
            .into_iter()
            .collect::<HashMap<u64, VersionData>>();

        VersionTable { values }
    }

    pub fn v7_and_above(network_magic: u64) -> VersionTable {
        let values = vec![
            (PROTOCOL_V7, VersionData::new(network_magic, false, None, None)),
            (PROTOCOL_V8, VersionData::new(network_magic, false, None, None)),
            (PROTOCOL_V9, VersionData::new(network_magic, false, None, None)),
            (PROTOCOL_V10, VersionData::new(network_magic, false, None, None)),
            (PROTOCOL_V11, VersionData::new(network_magic, false, Some(PEER_SHARING_NONE), Some(false))),
            (PROTOCOL_V12, VersionData::new(network_magic, false, Some(PEER_SHARING_NONE), Some(false))),
            (PROTOCOL_V13, VersionData::new(network_magic, false, Some(PEER_SHARING_NONE), Some(false))),
        ]
            .into_iter()
            .collect::<HashMap<u64, VersionData>>();

        VersionTable { values }
    }

    pub fn v11_and_above(network_magic: u64) -> VersionTable {
        let values = vec![
            (PROTOCOL_V11, VersionData::new(network_magic, false, Some(PEER_SHARING_NONE), Some(false))),
            (PROTOCOL_V12, VersionData::new(network_magic, false, Some(PEER_SHARING_NONE), Some(false))),
            (PROTOCOL_V13, VersionData::new(network_magic, false, Some(PEER_SHARING_NONE), Some(false))),
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
    peer_sharing: Option<u8>,
    query: Option<bool>,
}

impl VersionData {
    pub fn new(network_magic: u64, initiator_and_responder_diffusion_mode: bool, peer_sharing: Option<u8>, query: Option<bool>) -> Self {
        VersionData {
            network_magic,
            initiator_and_responder_diffusion_mode,
            peer_sharing,
            query,
        }
    }
}

impl Encode<()> for VersionData {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        match (self.peer_sharing, self.query) {
            (Some(_), Some(_)) => {
                e.array(4)?
                    .u64(self.network_magic)?
                    .bool(self.initiator_and_responder_diffusion_mode)?
                    .u8(self.peer_sharing.unwrap())?
                    .bool(self.query.unwrap())?;
            },
            _ => {
                e.array(2)?
                    .u64(self.network_magic)?
                    .bool(self.initiator_and_responder_diffusion_mode)?;
            },
        };

        Ok(())
    }
}

impl<'b> Decode<'b, ()> for VersionData {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        let len = d.array()?;
        let network_magic = d.u64()?;
        let initiator_and_responder_diffusion_mode = d.bool()?;
        let peer_sharing = match len {
            Some(4) => Some(d.u8()?),
            _ => None,
        };
        let query = match len {
            Some(4) => Some(d.bool()?),
            _ => None,
        };

        Ok(Self {
            network_magic,
            initiator_and_responder_diffusion_mode,
            peer_sharing,
            query,
        })
    }
}
