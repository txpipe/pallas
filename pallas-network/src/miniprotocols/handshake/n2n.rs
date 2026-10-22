use std::collections::HashMap;

use pallas_codec::minicbor::{Decode, Decoder, Encode, Encoder, decode, encode};

/// N2N-specific instantiation of the generic handshake version table.
pub type VersionTable = super::protocol::VersionTable<VersionData>;

const PROTOCOL_V7: u64 = 7;
const PROTOCOL_V8: u64 = 8;
const PROTOCOL_V9: u64 = 9;
const PROTOCOL_V10: u64 = 10;
const PROTOCOL_V11: u64 = 11;
const PROTOCOL_V12: u64 = 12;
const PROTOCOL_V13: u64 = 13;
const PROTOCOL_V14: u64 = 14;

const PEER_SHARING_DISABLED: u8 = 0;

impl VersionTable {
    /// Deprecated: forwards to [`Self::v7_and_above`] for backward compatibility.
    #[deprecated(note = "no longer supported by spec")]
    pub fn v4_and_above(network_magic: u64) -> VersionTable {
        // Older versions are not supported anymore (removed from network-spec.pdf).
        // Try not to break compatibility with older pallas users.
        Self::v7_and_above(network_magic)
    }

    /// Deprecated: forwards to [`Self::v7_and_above`] for backward compatibility.
    #[deprecated(note = "no longer supported by spec")]
    pub fn v6_and_above(network_magic: u64) -> VersionTable {
        // Older versions are not supported anymore (removed from network-spec.pdf).
        // Try not to break compatibility with older pallas users.
        Self::v7_and_above(network_magic)
    }

    /// Build a version table offering N2N protocol versions 7 through 10.
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

    /// Build a version table offering N2N versions 7 and up with `query=false`.
    pub fn v7_and_above(network_magic: u64) -> VersionTable {
        Self::v7_and_above_with_query(network_magic, false)
    }

    /// Build a version table offering N2N versions 7 and up with the given query flag.
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
        ]
        .into_iter()
        .collect::<HashMap<u64, VersionData>>();

        VersionTable { values }
    }

    /// Build a version table offering N2N versions 11 and up with `query=false`.
    pub fn v11_and_above(network_magic: u64) -> VersionTable {
        Self::v11_and_above_with_query(network_magic, false)
    }

    /// Build a version table offering N2N versions 11 and up with the given query flag.
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
        ]
        .into_iter()
        .collect::<HashMap<u64, VersionData>>();

        VersionTable { values }
    }
}

/// Per-version payload for the N2N handshake.
#[derive(Debug, Clone, PartialEq)]
pub struct VersionData {
    /// Network identifier (mainnet / testnet / …).
    pub network_magic: u64,
    /// Whether the peer is initiator-only (no responder duties).
    pub initiator_only_diffusion_mode: bool,
    /// Peer-sharing capability flag, when negotiated.
    pub peer_sharing: Option<u8>,
    /// Whether this exchange is a query-mode handshake.
    pub query: Option<bool>,
}

impl VersionData {
    /// Build a [`VersionData`] from its individual fields.
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
        }
    }
}

/// Mirrors `acceptableVersion` for `NodeToNodeVersionData` in ouroboros-network:
/// the network magic must match; the diffusion mode is initiator-only if either
/// side asks for it; peer sharing is enabled only if both sides enable it; query
/// mode is requested if either side requests it.
impl super::protocol::NegotiateVersionData for VersionData {
    /// Negotiate our payload against the proposed one, per the rules above.
    fn negotiate(&self, proposed: &Self) -> Result<Self, String> {
        if self.network_magic != proposed.network_magic {
            return Err(format!(
                "network magic mismatch: ours {}, proposed {}",
                self.network_magic, proposed.network_magic
            ));
        }

        // Versions 11 and up carry both peer sharing and query; earlier ones carry
        // neither. The two sides must use the same payload shape for a version.
        let (peer_sharing, query) = match (
            (self.peer_sharing, self.query),
            (proposed.peer_sharing, proposed.query),
        ) {
            ((Some(ours), Some(our_query)), (Some(theirs), Some(their_query))) => {
                (Some(ours.min(theirs)), Some(our_query || their_query))
            }
            ((None, None), (None, None)) => (None, None),
            _ => {
                return Err(format!(
                    "version data shape mismatch: ours {self:?}, proposed {proposed:?}"
                ));
            }
        };

        Ok(VersionData {
            network_magic: self.network_magic,
            initiator_only_diffusion_mode: self.initiator_only_diffusion_mode
                || proposed.initiator_only_diffusion_mode,
            peer_sharing,
            query,
        })
    }
}

impl Encode<()> for VersionData {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        match (self.peer_sharing, self.query) {
            (Some(peer_sharing), Some(query)) => {
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
            _ => None,
        };
        let query = match len {
            Some(4) => Some(d.bool()?),
            _ => None,
        };

        Ok(Self {
            network_magic,
            initiator_only_diffusion_mode,
            peer_sharing,
            query,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::protocol::NegotiateVersionData;
    use super::*;

    const MAINNET: u64 = 764824073;

    /// A mainnet cardano-node proposal is accepted as initiator-only with peer sharing disabled.
    #[test]
    fn accepts_a_duplex_peer_sharing_node_as_initiator_only() {
        // What cardano-node proposes for v14 on mainnet.
        let node = VersionData::new(MAINNET, false, Some(1), Some(false));
        let ours = VersionData::new(MAINNET, true, Some(PEER_SHARING_DISABLED), Some(false));

        let accepted = ours.negotiate(&node).unwrap();

        assert_eq!(
            accepted,
            VersionData::new(MAINNET, true, Some(PEER_SHARING_DISABLED), Some(false))
        );
    }

    /// Duplex diffusion and peer sharing survive when both sides offer them.
    #[test]
    fn keeps_duplex_and_peer_sharing_when_both_sides_offer_them() {
        let node = VersionData::new(MAINNET, false, Some(1), Some(false));
        let ours = VersionData::new(MAINNET, false, Some(1), Some(false));

        assert_eq!(ours.negotiate(&node).unwrap(), node);
    }

    /// The connection is initiator-only when either side asks for it.
    #[test]
    fn initiator_only_if_either_side_asks() {
        let ours = VersionData::new(MAINNET, false, Some(0), Some(false));
        let theirs = VersionData::new(MAINNET, true, Some(0), Some(false));

        assert!(
            ours.negotiate(&theirs)
                .unwrap()
                .initiator_only_diffusion_mode
        );
        assert!(
            theirs
                .negotiate(&ours)
                .unwrap()
                .initiator_only_diffusion_mode
        );
    }

    /// Query mode is requested when either side requests it.
    #[test]
    fn query_if_either_side_asks() {
        let ours = VersionData::new(MAINNET, true, Some(0), Some(false));
        let theirs = VersionData::new(MAINNET, true, Some(0), Some(true));

        assert_eq!(ours.negotiate(&theirs).unwrap().query, Some(true));
    }

    /// Pre-v11 payloads carry neither peer sharing nor query and stay that way.
    #[test]
    fn versions_before_11_carry_no_peer_sharing_or_query() {
        let ours = VersionData::new(MAINNET, true, None, None);
        let theirs = VersionData::new(MAINNET, false, None, None);

        assert_eq!(
            ours.negotiate(&theirs).unwrap(),
            VersionData::new(MAINNET, true, None, None)
        );
    }

    /// A legacy payload for a modern version entry is refused in both directions.
    #[test]
    fn refuses_a_legacy_payload_for_a_modern_entry() {
        let ours = VersionData::new(MAINNET, true, Some(0), Some(false));
        let theirs = VersionData::new(MAINNET, false, None, None);

        assert!(ours.negotiate(&theirs).is_err());
        assert!(theirs.negotiate(&ours).is_err());
    }

    /// A payload carrying only one of peer sharing and query is refused.
    #[test]
    fn refuses_a_partial_payload() {
        let ours = VersionData::new(MAINNET, true, Some(0), Some(false));
        let partial = VersionData::new(MAINNET, false, Some(1), None);

        assert!(ours.negotiate(&partial).is_err());
        assert!(partial.negotiate(&partial).is_err());
    }

    /// Payloads for different networks are refused.
    #[test]
    fn refuses_a_different_network_magic() {
        let ours = VersionData::new(MAINNET, true, Some(0), Some(false));
        let theirs = VersionData::new(1, false, Some(1), Some(false));

        assert!(ours.negotiate(&theirs).is_err());
    }
}
