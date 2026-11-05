use std::collections::HashMap;

use pallas_codec::minicbor::{Decode, Decoder, Encode, Encoder, decode, encode};

pub type VersionTable = super::VersionTable<VersionData>;

const PROTOCOL_V7: u64 = 7;
const PROTOCOL_V8: u64 = 8;
const PROTOCOL_V9: u64 = 9;
const PROTOCOL_V10: u64 = 10;
const PROTOCOL_V11: u64 = 11;
const PROTOCOL_V12: u64 = 12;
const PROTOCOL_V13: u64 = 13;
const PROTOCOL_V14: u64 = 14;
const PROTOCOL_V15: u64 = 15;
const PROTOCOL_V16: u64 = 16;

const PEER_SHARING_DISABLED: u8 = 0;
#[cfg(feature = "unstable")]
const PERAS_UNSUPPORTED: bool = false;

/// Minimum N2N protocol version that carries the Leios overlay. The Haskell node
/// runs the leios-notify / leios-fetch mini-protocols only on connections that
/// negotiated version 16 or higher; version 15 is mainline's SRV support and has
/// no Leios mini-protocols, so opening them on a v15 connection gets it reset.
/// From version 16 the version data carries a Peras support flag.
pub const LEIOS_MIN_VERSION: u64 = PROTOCOL_V16;

impl VersionTable {
    /// Builds a version table for v4 and above (deprecated, delegates to v7).
    #[deprecated(note = "no longer supported by spec")]
    pub fn v4_and_above(network_magic: u64) -> VersionTable {
        // Older versions are not supported anymore (removed from network-spec.pdf).
        // Try not to break compatibility with older pallas users.
        Self::v7_and_above(network_magic)
    }

    /// Builds a version table for v6 and above (deprecated, delegates to v7).
    #[deprecated(note = "no longer supported by spec")]
    pub fn v6_and_above(network_magic: u64) -> VersionTable {
        // Older versions are not supported anymore (removed from network-spec.pdf).
        // Try not to break compatibility with older pallas users.
        Self::v7_and_above(network_magic)
    }

    /// Builds a version table containing only versions 7 through 10.
    pub fn v7_to_v10(network_magic: u64) -> VersionTable {
        let values = vec![
            (
                PROTOCOL_V7,
                VersionData::new(network_magic, true, None, None),
            ),
            (
                PROTOCOL_V8,
                VersionData::new(network_magic, true, None, None),
            ),
            (
                PROTOCOL_V9,
                VersionData::new(network_magic, true, None, None),
            ),
            (
                PROTOCOL_V10,
                VersionData::new(network_magic, true, None, None),
            ),
        ]
        .into_iter()
        .collect::<HashMap<u64, VersionData>>();

        VersionTable { values }
    }

    /// Builds a version table for v7 and above with query mode disabled.
    pub fn v7_and_above(network_magic: u64) -> VersionTable {
        Self::v7_and_above_with_query(network_magic, false)
    }

    /// Builds a version table for v7 and above, optionally enabling query mode.
    pub fn v7_and_above_with_query(network_magic: u64, query: bool) -> VersionTable {
        let values = vec![
            (
                PROTOCOL_V7,
                VersionData::new(network_magic, true, None, None),
            ),
            (
                PROTOCOL_V8,
                VersionData::new(network_magic, true, None, None),
            ),
            (
                PROTOCOL_V9,
                VersionData::new(network_magic, true, None, None),
            ),
            (
                PROTOCOL_V10,
                VersionData::new(network_magic, true, None, None),
            ),
            (
                PROTOCOL_V11,
                VersionData::new(
                    network_magic,
                    true,
                    Some(PEER_SHARING_DISABLED),
                    Some(query),
                ),
            ),
            (
                PROTOCOL_V12,
                VersionData::new(
                    network_magic,
                    true,
                    Some(PEER_SHARING_DISABLED),
                    Some(query),
                ),
            ),
            (
                PROTOCOL_V13,
                VersionData::new(
                    network_magic,
                    true,
                    Some(PEER_SHARING_DISABLED),
                    Some(query),
                ),
            ),
            (
                PROTOCOL_V14,
                VersionData::new(
                    network_magic,
                    true,
                    Some(PEER_SHARING_DISABLED),
                    Some(query),
                ),
            ),
            (
                PROTOCOL_V15,
                VersionData::new(
                    network_magic,
                    true,
                    Some(PEER_SHARING_DISABLED),
                    Some(query),
                ),
            ),
            #[cfg(feature = "unstable")]
            (
                PROTOCOL_V16,
                VersionData::new(
                    network_magic,
                    true,
                    Some(PEER_SHARING_DISABLED),
                    Some(query),
                )
                .with_peras_support(PERAS_UNSUPPORTED),
            ),
        ]
        .into_iter()
        .collect::<HashMap<u64, VersionData>>();

        VersionTable { values }
    }

    /// Builds a version table for v11 and above with query mode disabled.
    pub fn v11_and_above(network_magic: u64) -> VersionTable {
        Self::v11_and_above_with_query(network_magic, false)
    }

    /// Builds a version table for v11 and above, optionally enabling query mode.
    pub fn v11_and_above_with_query(network_magic: u64, query: bool) -> VersionTable {
        let values = vec![
            (
                PROTOCOL_V11,
                VersionData::new(
                    network_magic,
                    true,
                    Some(PEER_SHARING_DISABLED),
                    Some(query),
                ),
            ),
            (
                PROTOCOL_V12,
                VersionData::new(
                    network_magic,
                    true,
                    Some(PEER_SHARING_DISABLED),
                    Some(query),
                ),
            ),
            (
                PROTOCOL_V13,
                VersionData::new(
                    network_magic,
                    true,
                    Some(PEER_SHARING_DISABLED),
                    Some(query),
                ),
            ),
            (
                PROTOCOL_V14,
                VersionData::new(
                    network_magic,
                    true,
                    Some(PEER_SHARING_DISABLED),
                    Some(query),
                ),
            ),
            (
                PROTOCOL_V15,
                VersionData::new(
                    network_magic,
                    true,
                    Some(PEER_SHARING_DISABLED),
                    Some(query),
                ),
            ),
            #[cfg(feature = "unstable")]
            (
                PROTOCOL_V16,
                VersionData::new(
                    network_magic,
                    true,
                    Some(PEER_SHARING_DISABLED),
                    Some(query),
                )
                .with_peras_support(PERAS_UNSUPPORTED),
            ),
        ]
        .into_iter()
        .collect::<HashMap<u64, VersionData>>();

        VersionTable { values }
    }

    /// Builds a version table for v15 and above with query mode disabled.
    pub fn v15_and_above(network_magic: u64) -> VersionTable {
        Self::v15_and_above_with_query(network_magic, false)
    }

    /// Builds a version table for v15 and above, optionally enabling query mode.
    pub fn v15_and_above_with_query(network_magic: u64, query: bool) -> VersionTable {
        let values = vec![
            (
                PROTOCOL_V15,
                VersionData::new(
                    network_magic,
                    true,
                    Some(PEER_SHARING_DISABLED),
                    Some(query),
                ),
            ),
            #[cfg(feature = "unstable")]
            (
                PROTOCOL_V16,
                VersionData::new(
                    network_magic,
                    true,
                    Some(PEER_SHARING_DISABLED),
                    Some(query),
                )
                .with_peras_support(PERAS_UNSUPPORTED),
            ),
        ]
        .into_iter()
        .collect::<HashMap<u64, VersionData>>();

        VersionTable { values }
    }
}

/// Version-specific data exchanged during a node-to-node handshake.
#[derive(Debug, Clone, PartialEq)]
pub struct VersionData {
    /// The network magic identifying the Cardano network.
    pub network_magic: u64,
    /// Whether the initiator operates in diffusion-only mode.
    pub initiator_only_diffusion_mode: bool,
    /// Peer sharing configuration (available from protocol v11+).
    pub peer_sharing: Option<u8>,
    /// Whether this is a query-only connection (available from protocol v11+).
    pub query: Option<bool>,
    /// Whether the node runs the Peras mini-protocols (required from protocol
    /// v16, rejected by the Haskell node below it).
    #[cfg(feature = "unstable")]
    pub peras_support: Option<bool>,
}

impl VersionData {
    /// Creates new version data with the given parameters.
    ///
    /// Peras support is left unset (`None`), which is the shape of every version
    /// below v16. Use [`VersionData::with_peras_support`] to set it for v16+.
    pub fn new(
        network_magic: u64,
        initiator_only_diffusion_mode: bool,
        peer_sharing: Option<u8>,
        query: Option<bool>,
    ) -> Self {
        VersionData {
            network_magic,
            initiator_only_diffusion_mode,
            peer_sharing,
            query,
            #[cfg(feature = "unstable")]
            peras_support: None,
        }
    }

    /// Sets the Peras support flag, required from protocol v16 (encoded only
    /// when `peer_sharing` and `query` are also set).
    #[cfg(feature = "unstable")]
    pub fn with_peras_support(mut self, peras_support: bool) -> Self {
        self.peras_support = Some(peras_support);
        self
    }
}

impl Encode<()> for VersionData {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        #[cfg(feature = "unstable")]
        let peras_support = self.peras_support;
        #[cfg(not(feature = "unstable"))]
        let peras_support: Option<bool> = None;

        match (self.peer_sharing, self.query, peras_support) {
            #[cfg(feature = "unstable")]
            (Some(peer_sharing), Some(query), Some(peras_support)) => {
                e.array(5)?
                    .u64(self.network_magic)?
                    .bool(self.initiator_only_diffusion_mode)?
                    .u8(peer_sharing)?
                    .bool(query)?
                    .bool(peras_support)?;
            }
            (Some(peer_sharing), Some(query), None) => {
                e.array(4)?
                    .u64(self.network_magic)?
                    .bool(self.initiator_only_diffusion_mode)?
                    .u8(peer_sharing)?
                    .bool(query)?;
            }
            _ => {
                e.array(2)?
                    .u64(self.network_magic)?
                    .bool(self.initiator_only_diffusion_mode)?;
            }
        };

        Ok(())
    }
}

impl<'b> Decode<'b, ()> for VersionData {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        let len = d.array()?;
        let network_magic = d.u64()?;
        let initiator_only_diffusion_mode = d.bool()?;
        let peer_sharing = match len {
            Some(4) => Some(d.u8()?),
            #[cfg(feature = "unstable")]
            Some(5) => Some(d.u8()?),
            _ => None,
        };
        let query = match len {
            Some(4) => Some(d.bool()?),
            #[cfg(feature = "unstable")]
            Some(5) => Some(d.bool()?),
            _ => None,
        };
        #[cfg(feature = "unstable")]
        let peras_support = match len {
            Some(5) => Some(d.bool()?),
            _ => None,
        };

        Ok(Self {
            network_magic,
            initiator_only_diffusion_mode,
            peer_sharing,
            query,
            #[cfg(feature = "unstable")]
            peras_support,
        })
    }
}
