use std::net::SocketAddr;
use std::path::Path;
use thiserror::Error;
use tracing::error;

use tokio::net::{TcpListener, ToSocketAddrs};

#[cfg(unix)]
use tokio::net::{unix::SocketAddr as UnixSocketAddr, UnixListener};

use crate::miniprotocols::handshake::{n2c, n2n, Confirmation, VersionNumber};

use crate::miniprotocols::{
    blockfetch, chainsync, handshake, keepalive, localstate, txsubmission, PROTOCOL_N2C_CHAIN_SYNC,
    PROTOCOL_N2C_HANDSHAKE, PROTOCOL_N2C_STATE_QUERY, PROTOCOL_N2N_BLOCK_FETCH,
    PROTOCOL_N2N_CHAIN_SYNC, PROTOCOL_N2N_HANDSHAKE, PROTOCOL_N2N_KEEP_ALIVE,
    PROTOCOL_N2N_TX_SUBMISSION,
};

use crate::multiplexer::{self, Bearer, RunningPlexer};

#[derive(Debug, Error)]
pub enum Error {
    #[error("error in multiplexer")]
    PlexerFailure(#[source] multiplexer::Error),

    #[error("error connecting bearer")]
    ConnectFailure(#[source] tokio::io::Error),

    #[error("handshake protocol error")]
    HandshakeProtocol(handshake::Error),

    #[error("handshake version not accepted")]
    IncompatibleVersion,
}

/// Client of N2N Ouroboros
pub struct PeerClient {
    pub plexer: RunningPlexer,
    pub handshake: handshake::N2NClient,
    pub chainsync: chainsync::N2NClient,
    pub blockfetch: blockfetch::Client,
    pub txsubmission: txsubmission::Client,
    pub keepalive: keepalive::Client,
}

impl PeerClient {
    pub fn new(bearer: Bearer) -> Self {
        let mut plexer = multiplexer::Plexer::new(bearer);

        let hs_channel = plexer.subscribe_client(PROTOCOL_N2N_HANDSHAKE);
        let cs_channel = plexer.subscribe_client(PROTOCOL_N2N_CHAIN_SYNC);
        let bf_channel = plexer.subscribe_client(PROTOCOL_N2N_BLOCK_FETCH);
        let txsub_channel = plexer.subscribe_client(PROTOCOL_N2N_TX_SUBMISSION);
        let keepalive_channel = plexer.subscribe_client(PROTOCOL_N2N_KEEP_ALIVE);

        let plexer = plexer.spawn();

        Self {
            plexer,
            handshake: handshake::Client::new(hs_channel),
            chainsync: chainsync::Client::new(cs_channel),
            blockfetch: blockfetch::Client::new(bf_channel),
            txsubmission: txsubmission::Client::new(txsub_channel),
            keepalive: keepalive::Client::new(keepalive_channel),
        }
    }

    pub async fn connect(addr: impl ToSocketAddrs, magic: u64) -> Result<Self, Error> {
        let bearer = Bearer::connect_tcp(addr)
            .await
            .map_err(Error::ConnectFailure)?;

        let mut client = Self::new(bearer);

        let versions = handshake::n2n::VersionTable::v7_and_above(magic);

        let handshake = client
            .handshake()
            .handshake(versions)
            .await
            .map_err(Error::HandshakeProtocol)?;

        if let handshake::Confirmation::Rejected(reason) = handshake {
            error!(?reason, "handshake refused");
            return Err(Error::IncompatibleVersion);
        }

        Ok(client)
    }

    pub fn handshake(&mut self) -> &mut handshake::N2NClient {
        &mut self.handshake
    }

    pub fn chainsync(&mut self) -> &mut chainsync::N2NClient {
        &mut self.chainsync
    }

    pub async fn with_chainsync<T, O, Fut>(&mut self, op: T) -> tokio::task::JoinHandle<O>
    where
        T: FnOnce(&mut chainsync::N2NClient) -> Fut,
        Fut: std::future::Future<Output = O> + Send + 'static,
        O: Send + 'static,
    {
        tokio::spawn(op(&mut self.chainsync))
    }

    pub fn blockfetch(&mut self) -> &mut blockfetch::Client {
        &mut self.blockfetch
    }

    pub fn txsubmission(&mut self) -> &mut txsubmission::Client {
        &mut self.txsubmission
    }

    pub fn keepalive(&mut self) -> &mut keepalive::Client {
        &mut self.keepalive
    }

    pub async fn abort(self) {
        self.plexer.abort().await
    }
}

/// Server of N2N Ouroboros
pub struct PeerServer {
    plexer: RunningPlexer,
    handshake: handshake::N2NServer,
    chainsync: chainsync::N2NServer,
    blockfetch: blockfetch::Server,
    txsubmission: txsubmission::Server,
    accepted_address: Option<SocketAddr>,
    accepted_version: Option<u64>,
}

impl PeerServer {
    pub fn new(bearer: Bearer) -> Self {
        let mut plexer = multiplexer::Plexer::new(bearer);

        let hs_channel = plexer.subscribe_server(PROTOCOL_N2N_HANDSHAKE);
        let cs_channel = plexer.subscribe_server(PROTOCOL_N2N_CHAIN_SYNC);
        let bf_channel = plexer.subscribe_server(PROTOCOL_N2N_BLOCK_FETCH);
        let txsub_channel = plexer.subscribe_server(PROTOCOL_N2N_TX_SUBMISSION);

        let hs = handshake::N2NServer::new(hs_channel);
        let cs = chainsync::N2NServer::new(cs_channel);
        let bf = blockfetch::Server::new(bf_channel);
        let txsub = txsubmission::Server::new(txsub_channel);

        let plexer = plexer.spawn();

        Self {
            plexer,
            handshake: hs,
            chainsync: cs,
            blockfetch: bf,
            txsubmission: txsub,
            accepted_address: None,
            accepted_version: None,
        }
    }

    pub async fn accept(listener: &TcpListener, magic: u64) -> Result<Self, Error> {
        let (bearer, address) = Bearer::accept_tcp(listener)
            .await
            .map_err(Error::ConnectFailure)?;

        let mut client = Self::new(bearer);

        let accepted_version = client
            .handshake()
            .handshake(n2n::VersionTable::v7_and_above(magic))
            .await
            .map_err(Error::HandshakeProtocol)?;

        if let Some((version, _)) = accepted_version {
            client.accepted_address = Some(address);
            client.accepted_version = Some(version);
            Ok(client)
        } else {
            client.abort().await;
            Err(Error::IncompatibleVersion)
        }
    }

    pub fn handshake(&mut self) -> &mut handshake::N2NServer {
        &mut self.handshake
    }

    pub fn chainsync(&mut self) -> &mut chainsync::N2NServer {
        &mut self.chainsync
    }

    pub fn blockfetch(&mut self) -> &mut blockfetch::Server {
        &mut self.blockfetch
    }

    pub fn txsubmission(&mut self) -> &mut txsubmission::Server {
        &mut self.txsubmission
    }

    pub async fn abort(self) {
        self.plexer.abort().await
    }
}

/// Client of N2C Ouroboros
pub struct NodeClient {
    plexer: RunningPlexer,
    handshake: handshake::N2CClient,
    chainsync: chainsync::N2CClient,
    statequery: localstate::Client,
}

impl NodeClient {
    pub fn new(bearer: Bearer) -> Self {
        let mut plexer = multiplexer::Plexer::new(bearer);

        let hs_channel = plexer.subscribe_client(PROTOCOL_N2C_HANDSHAKE);
        let cs_channel = plexer.subscribe_client(PROTOCOL_N2C_CHAIN_SYNC);
        let sq_channel = plexer.subscribe_client(PROTOCOL_N2C_STATE_QUERY);

        let plexer = plexer.spawn();

        Self {
            plexer,
            handshake: handshake::Client::new(hs_channel),
            chainsync: chainsync::Client::new(cs_channel),
            statequery: localstate::Client::new(sq_channel),
        }
    }

    #[cfg(unix)]
    pub async fn connect(path: impl AsRef<Path>, magic: u64) -> Result<Self, Error> {
        let bearer = Bearer::connect_unix(path)
            .await
            .map_err(Error::ConnectFailure)?;

        let mut client = Self::new(bearer);

        let versions = handshake::n2c::VersionTable::v10_and_above(magic);

        let handshake = client
            .handshake()
            .handshake(versions)
            .await
            .map_err(Error::HandshakeProtocol)?;

        if let handshake::Confirmation::Rejected(reason) = handshake {
            error!(?reason, "handshake refused");
            return Err(Error::IncompatibleVersion);
        }

        Ok(client)
    }

    #[cfg(windows)]
    pub async fn connect(
        pipe_name: impl AsRef<std::ffi::OsStr>,
        magic: u64,
    ) -> Result<Self, Error> {
        let pipe_name = pipe_name.as_ref().to_os_string();

        let bearer = tokio::task::spawn_blocking(move || Bearer::connect_named_pipe(pipe_name))
            .await
            .expect("can't join tokio thread")
            .map_err(Error::ConnectFailure)?;

        let mut client = Self::new(bearer);

        let versions = handshake::n2c::VersionTable::v10_and_above(magic);

        let handshake = client
            .handshake()
            .handshake(versions)
            .await
            .map_err(Error::HandshakeProtocol)?;

        if let handshake::Confirmation::Rejected(reason) = handshake {
            error!(?reason, "handshake refused");
            return Err(Error::IncompatibleVersion);
        }

        Ok(client)
    }

    #[cfg(unix)]
    pub async fn handshake_query(
        bearer: Bearer,
        magic: u64,
    ) -> Result<handshake::n2c::VersionTable, Error> {
        let mut plexer = multiplexer::Plexer::new(bearer);

        let hs_channel = plexer.subscribe_client(PROTOCOL_N2C_HANDSHAKE);

        let plexer = plexer.spawn();

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
                plexer.abort().await;
                Ok(version_table)
            }
        }
    }

    pub fn handshake(&mut self) -> &mut handshake::N2CClient {
        &mut self.handshake
    }

    pub fn chainsync(&mut self) -> &mut chainsync::N2CClient {
        &mut self.chainsync
    }

    pub fn statequery(&mut self) -> &mut localstate::Client {
        &mut self.statequery
    }

    pub async fn abort(self) {
        self.plexer.abort().await
    }
}

/// Server of N2C Ouroboros.
#[cfg(unix)]
pub struct NodeServer {
    plexer: RunningPlexer,
    handshake: handshake::N2CServer,
    chainsync: chainsync::N2CServer,
    statequery: localstate::Server,
    accepted_address: Option<UnixSocketAddr>,
    accpeted_version: Option<(VersionNumber, n2c::VersionData)>,
}

#[cfg(unix)]
impl NodeServer {
    pub async fn new(bearer: Bearer) -> Self {
        let mut plexer = multiplexer::Plexer::new(bearer);

        let hs_channel = plexer.subscribe_server(PROTOCOL_N2C_HANDSHAKE);
        let cs_channel = plexer.subscribe_server(PROTOCOL_N2C_CHAIN_SYNC);
        let sq_channel = plexer.subscribe_server(PROTOCOL_N2C_STATE_QUERY);

        let server_hs = handshake::Server::<n2c::VersionData>::new(hs_channel);
        let server_cs = chainsync::N2CServer::new(cs_channel);
        let server_sq = localstate::Server::new(sq_channel);

        let plexer = plexer.spawn();

        Self {
            plexer,
            handshake: server_hs,
            chainsync: server_cs,
            statequery: server_sq,
            accepted_address: None,
            accpeted_version: None,
        }
    }

    pub async fn accept(listener: &UnixListener, magic: u64) -> Result<Self, Error> {
        let (bearer, address) = Bearer::accept_unix(listener)
            .await
            .map_err(Error::ConnectFailure)?;

        let mut client = Self::new(bearer).await;

        let accepted_version = client
            .handshake()
            .handshake(n2c::VersionTable::v10_and_above(magic))
            .await
            .map_err(Error::HandshakeProtocol)?;

        if let Some(version) = accepted_version {
            client.accepted_address = Some(address);
            client.accpeted_version = Some(version);
            Ok(client)
        } else {
            client.abort().await;
            Err(Error::IncompatibleVersion)
        }
    }

    pub fn handshake(&mut self) -> &mut handshake::N2CServer {
        &mut self.handshake
    }

    pub fn chainsync(&mut self) -> &mut chainsync::N2CServer {
        &mut self.chainsync
    }

    pub fn statequery(&mut self) -> &mut localstate::Server {
        &mut self.statequery
    }

    pub async fn abort(self) {
        self.plexer.abort().await
    }
}
