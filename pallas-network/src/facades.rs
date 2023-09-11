use std::path::Path;

use thiserror::Error;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tracing::{debug, error};

#[cfg(not(target_os = "windows"))]
use tokio::net::UnixListener;

use crate::miniprotocols::handshake::{n2c, n2n, Confirmation, VersionNumber};
use crate::miniprotocols::PROTOCOL_N2N_HANDSHAKE;
use crate::{
    miniprotocols::{
        blockfetch, chainsync, handshake, localstate, PROTOCOL_N2C_CHAIN_SYNC,
        PROTOCOL_N2C_HANDSHAKE, PROTOCOL_N2C_STATE_QUERY, PROTOCOL_N2N_BLOCK_FETCH,
        PROTOCOL_N2N_CHAIN_SYNC,
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

/// Client of N2N Ouroboros
pub struct PeerClient {
    pub plexer_handle: JoinHandle<Result<(), crate::multiplexer::Error>>,
    pub handshake: handshake::Confirmation<handshake::n2n::VersionData>,
    pub chainsync: chainsync::N2NClient,
    pub blockfetch: blockfetch::Client,
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

/// Server of N2N Ouroboros
pub struct PeerServer {
    pub plexer_handle: JoinHandle<Result<(), crate::multiplexer::Error>>,
    pub version: (VersionNumber, n2n::VersionData),
    pub chainsync: chainsync::N2NServer,
    pub blockfetch: blockfetch::Server,
}

impl PeerServer {
    pub async fn accept(listener: &TcpListener, magic: u64) -> Result<Self, Error> {
        let (bearer, _) = Bearer::accept_tcp(listener)
            .await
            .map_err(Error::ConnectFailure)?;

        let mut server_plexer = multiplexer::Plexer::new(bearer);

        let hs_channel = server_plexer.subscribe_server(PROTOCOL_N2N_HANDSHAKE);
        let cs_channel = server_plexer.subscribe_server(PROTOCOL_N2N_CHAIN_SYNC);
        let bf_channel = server_plexer.subscribe_server(PROTOCOL_N2N_BLOCK_FETCH);

        let mut server_hs: handshake::Server<n2n::VersionData> = handshake::Server::new(hs_channel);
        let server_cs = chainsync::N2NServer::new(cs_channel);
        let server_bf = blockfetch::Server::new(bf_channel);

        let plexer_handle = tokio::spawn(async move { server_plexer.run().await });

        let accepted_version = server_hs
            .handshake(n2n::VersionTable::v7_and_above(magic))
            .await
            .map_err(Error::HandshakeProtocol)?;

        if let Some(ver) = accepted_version {
            Ok(Self {
                plexer_handle,
                version: ver,
                chainsync: server_cs,
                blockfetch: server_bf,
            })
        } else {
            plexer_handle.abort();
            Err(Error::IncompatibleVersion)
        }
    }

    pub fn chainsync(&mut self) -> &mut chainsync::N2NServer {
        &mut self.chainsync
    }

    pub fn blockfetch(&mut self) -> &mut blockfetch::Server {
        &mut self.blockfetch
    }

    pub fn abort(&mut self) {
        self.plexer_handle.abort();
    }
}

/// Client of N2C Ouroboros
pub struct NodeClient {
    pub plexer_handle: JoinHandle<Result<(), crate::multiplexer::Error>>,
    pub handshake: handshake::Confirmation<handshake::n2c::VersionData>,
    pub chainsync: chainsync::N2CClient,
    pub statequery: localstate::ClientV10,
}

impl NodeClient {
    #[cfg(not(target_os = "windows"))]
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

    #[cfg(not(target_os = "windows"))]
    pub async fn handshake_query(
        path: impl AsRef<Path>,
        magic: u64,
    ) -> Result<handshake::n2c::VersionTable, Error> {
        debug!("connecting");

        let bearer = Bearer::connect_unix(path)
            .await
            .map_err(Error::ConnectFailure)?;

        let mut plexer = multiplexer::Plexer::new(bearer);

        let hs_channel = plexer.subscribe_client(PROTOCOL_N2C_HANDSHAKE);

        let plexer_handle = tokio::spawn(async move { plexer.run().await });

        let versions = handshake::n2c::VersionTable::v15_with_query(magic);
        let mut client = handshake::Client::new(hs_channel);

        let handshake = client
            .handshake(versions)
            .await
            .map_err(Error::HandshakeProtocol)?;

        match handshake {
            Confirmation::Accepted(_, _) => {
                error!("handshake accepted when we expected query reply");
                Err(Error::HandshakeProtocol(handshake::Error::InvalidInbound))
            }
            Confirmation::Rejected(reason) => {
                error!(?reason, "handshake refused");
                Err(Error::IncompatibleVersion)
            }
            Confirmation::QueryReply(version_table) => {
                plexer_handle.abort();
                Ok(version_table)
            }
        }
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

/// Server of N2C Ouroboros
pub struct NodeServer {
    pub plexer_handle: JoinHandle<Result<(), crate::multiplexer::Error>>,
    pub version: (VersionNumber, n2c::VersionData),
    pub chainsync: chainsync::N2CServer,
    // statequery: localstate::Server,
}

#[cfg(not(target_os = "windows"))]
impl NodeServer {
    pub async fn accept(listener: &UnixListener, magic: u64) -> Result<Self, Error> {
        let (bearer, _) = Bearer::accept_unix(listener)
            .await
            .map_err(Error::ConnectFailure)?;

        let mut server_plexer = multiplexer::Plexer::new(bearer);

        let hs_channel = server_plexer.subscribe_server(PROTOCOL_N2C_HANDSHAKE);
        let cs_channel = server_plexer.subscribe_server(PROTOCOL_N2C_CHAIN_SYNC);
        // let sq_channel = server_plexer.subscribe_server(PROTOCOL_N2C_STATE_QUERY);

        let mut server_hs: handshake::Server<n2c::VersionData> = handshake::Server::new(hs_channel);
        let server_cs = chainsync::N2CServer::new(cs_channel);
        // let server_sq = localstate::Server::new(sq_channel);

        let plexer_handle = tokio::spawn(async move { server_plexer.run().await });

        let accepted_version = server_hs
            .handshake(n2c::VersionTable::v10_and_above(magic))
            .await
            .map_err(Error::HandshakeProtocol)?;

        if let Some(ver) = accepted_version {
            Ok(Self {
                plexer_handle,
                version: ver,
                chainsync: server_cs,
                // statequery: server_sq
            })
        } else {
            plexer_handle.abort();
            Err(Error::IncompatibleVersion)
        }
    }

    pub fn chainsync(&mut self) -> &mut chainsync::N2CServer {
        &mut self.chainsync
    }

    // pub fn statequery(&mut self) -> &mut localstate::Server {
    //     &mut self.statequery
    // }

    pub fn abort(&mut self) {
        self.plexer_handle.abort();
    }
}
