use std::path::Path;

use thiserror::Error;
use tokio::task::JoinHandle;
use tracing::{debug, error};

use crate::{
    miniprotocols::{
        blockfetch, chainsync, handshake, localstate, PROTOCOL_N2C_CHAIN_SYNC,
        PROTOCOL_N2C_HANDSHAKE, PROTOCOL_N2C_STATE_QUERY,
    },
    multiplexer::{self, Bearer},
};

#[derive(Debug, Error)]
pub enum Error {
    #[error("error connecting bearer")]
    ConnectFailure(#[source] tokio::io::Error),

    #[error("handshake protocol error")]
    HandshakeProtocol(handshake::Error),

    #[error("handshake version not accepted")]
    IncompatibleVersion,
}

pub struct PeerClient {
    plexer_handle: JoinHandle<Result<(), crate::multiplexer::Error>>,
    pub handshake: handshake::Confirmation<handshake::n2n::VersionData>,
    chainsync: chainsync::N2NClient,
    blockfetch: blockfetch::Client,
}

impl PeerClient {
    pub async fn connect(address: &str, magic: u64) -> Result<Self, Error> {
        debug!("connecting");
        let bearer = Bearer::connect_tcp(address)
            .await
            .map_err(Error::ConnectFailure)?;

        let mut plexer = multiplexer::Plexer::new(bearer);

        let channel0 = plexer.subscribe_client(0);
        let channel2 = plexer.subscribe_client(2);
        let channel3 = plexer.subscribe_client(3);

        let plexer_handle = tokio::spawn(async move { plexer.run().await });

        let versions = handshake::n2n::VersionTable::v7_and_above(magic);
        let mut client = handshake::Client::new(channel0);

        let handshake = client
            .handshake(versions)
            .await
            .map_err(Error::HandshakeProtocol)?;

        if let handshake::Confirmation::Rejected(reason) = handshake {
            error!(?reason, "handshake refused");
            return Err(Error::IncompatibleVersion);
        }

        Ok(Self {
            plexer_handle,
            handshake,
            chainsync: chainsync::Client::new(channel2),
            blockfetch: blockfetch::Client::new(channel3),
        })
    }

    pub fn chainsync(&mut self) -> &mut chainsync::N2NClient {
        &mut self.chainsync
    }

    pub fn blockfetch(&mut self) -> &mut blockfetch::Client {
        &mut self.blockfetch
    }

    pub fn abort(&mut self) {
        self.plexer_handle.abort();
    }
}

pub struct NodeClient {
    plexer_handle: JoinHandle<Result<(), crate::multiplexer::Error>>,
    pub handshake: handshake::Confirmation<handshake::n2c::VersionData>,
    chainsync: chainsync::N2CClient,
    statequery: localstate::ClientV10,
}

impl NodeClient {
    pub async fn connect(path: impl AsRef<Path>, magic: u64) -> Result<Self, Error> {
        debug!("connecting");

        let bearer = Bearer::connect_unix(path)
            .await
            .map_err(Error::ConnectFailure)?;

        let mut plexer = multiplexer::Plexer::new(bearer);

        let hs_channel = plexer.subscribe_client(PROTOCOL_N2C_HANDSHAKE);
        let cs_channel = plexer.subscribe_client(PROTOCOL_N2C_CHAIN_SYNC);
        let sq_channel = plexer.subscribe_client(PROTOCOL_N2C_STATE_QUERY);

        let plexer_handle = tokio::spawn(async move { plexer.run().await });

        let versions = handshake::n2c::VersionTable::v10_and_above(magic);
        let mut client = handshake::Client::new(hs_channel);

        let handshake = client
            .handshake(versions)
            .await
            .map_err(Error::HandshakeProtocol)?;

        if let handshake::Confirmation::Rejected(reason) = handshake {
            error!(?reason, "handshake refused");
            return Err(Error::IncompatibleVersion);
        }

        Ok(Self {
            plexer_handle,
            handshake,
            chainsync: chainsync::Client::new(cs_channel),
            statequery: localstate::Client::new(sq_channel),
        })
    }

    pub fn chainsync(&mut self) -> &mut chainsync::N2CClient {
        &mut self.chainsync
    }

    pub fn statequery(&mut self) -> &mut localstate::ClientV10 {
        &mut self.statequery
    }

    pub fn abort(&mut self) {
        self.plexer_handle.abort();
    }
}
