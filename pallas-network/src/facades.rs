use std::net::SocketAddr;
use std::path::Path;
use std::time::Duration;
use thiserror::Error;
use tracing::{debug, error};

use tokio::net::{TcpListener, ToSocketAddrs};

#[cfg(unix)]
use tokio::net::{UnixListener, unix::SocketAddr as UnixSocketAddr};

use crate::miniprotocols::handshake::n2n::VersionData;
use crate::miniprotocols::handshake::{Confirmation, VersionNumber, VersionTable, n2c, n2n};

use crate::miniprotocols::{
    PROTOCOL_N2C_CHAIN_SYNC, PROTOCOL_N2C_HANDSHAKE, PROTOCOL_N2C_MSG_NOTIFICATION,
    PROTOCOL_N2C_MSG_SUBMISSION, PROTOCOL_N2C_STATE_QUERY, PROTOCOL_N2C_TX_MONITOR,
    PROTOCOL_N2C_TX_SUBMISSION, PROTOCOL_N2N_BLOCK_FETCH, PROTOCOL_N2N_CHAIN_SYNC,
    PROTOCOL_N2N_HANDSHAKE, PROTOCOL_N2N_KEEP_ALIVE, PROTOCOL_N2N_PEER_SHARING,
    PROTOCOL_N2N_TX_SUBMISSION, blockfetch, chainsync, handshake, keepalive, localmsgnotification,
    localmsgsubmission, localstate, localtxsubmission, peersharing, txmonitor, txsubmission,
};

use crate::multiplexer::{self, Bearer, RunningPlexer};

/// Errors produced by the high-level peer/node facades.
#[derive(Debug, Error)]
pub enum Error {
    /// Underlying multiplexer failure.
    #[error("error in multiplexer")]
    PlexerFailure(#[source] multiplexer::Error),

    /// Failed to open or accept the bearer connection.
    #[error("error connecting bearer")]
    ConnectFailure(#[source] tokio::io::Error),

    /// Handshake mini-protocol error.
    #[error("handshake protocol error")]
    HandshakeProtocol(handshake::Error),

    /// Keep-alive loop reported a client-side error.
    #[error("keepalive client loop error")]
    KeepAliveClientLoop(keepalive::ClientError),

    /// Keep-alive loop reported a server-side error.
    #[error("keepalive server loop error")]
    KeepAliveServerLoop(keepalive::ServerError),

    /// The remote rejected every version offered in the handshake.
    #[error("handshake version not accepted")]
    IncompatibleVersion,
}

/// Default interval between keep-alive pings.
pub const DEFAULT_KEEP_ALIVE_INTERVAL_SEC: u64 = 20;

/// Handle to a spawned keep-alive loop.
pub type KeepAliveHandle = tokio::task::JoinHandle<Result<(), Error>>;

/// A keep-alive loop on either side of the connection.
pub enum KeepAliveLoop {
    /// Client-side loop: ping the peer at the given interval.
    Client(keepalive::Client, Duration),
    /// Server-side loop: respond to pings from the peer.
    Server(keepalive::Server),
}

impl KeepAliveLoop {
    /// Build a client-side loop that pings every `interval`.
    pub fn client(client: keepalive::Client, interval: Duration) -> Self {
        Self::Client(client, interval)
    }

    /// Build a server-side loop that responds to incoming pings.
    pub fn server(server: keepalive::Server) -> Self {
        Self::Server(server)
    }

    /// Drive a client-side keep-alive loop until it errors.
    pub async fn run_client(
        mut client: keepalive::Client,
        interval: Duration,
    ) -> Result<(), Error> {
        let mut interval = tokio::time::interval(interval);

        loop {
            interval.tick().await;
            debug!("sending keepalive request");

            client
                .keepalive_roundtrip()
                .await
                .map_err(Error::KeepAliveClientLoop)?;
        }
    }

    /// Drive a server-side keep-alive loop until it errors.
    pub async fn run_server(mut server: keepalive::Server) -> Result<(), Error> {
        loop {
            debug!("waiting keepalive request");

            server
                .keepalive_roundtrip()
                .await
                .map_err(Error::KeepAliveServerLoop)?;
        }
    }

    /// Spawn the loop on the current Tokio runtime.
    pub fn spawn(self) -> KeepAliveHandle {
        match self {
            KeepAliveLoop::Client(client, interval) => {
                tokio::spawn(Self::run_client(client, interval))
            }
            KeepAliveLoop::Server(server) => tokio::spawn(Self::run_server(server)),
        }
    }
}

/// Node-to-node Ouroboros client. Bundles the chain-sync, block-fetch,
/// tx-submission, peer-sharing, and keep-alive protocols over a single bearer.
pub struct PeerClient {
    /// Underlying running multiplexer.
    pub plexer: RunningPlexer,
    /// Handle to the spawned keep-alive task.
    pub keepalive: KeepAliveHandle,
    /// Chain-sync client (node-to-node).
    pub chainsync: chainsync::N2NClient,
    /// Block-fetch client.
    pub blockfetch: blockfetch::Client,
    /// Tx-submission client.
    pub txsubmission: txsubmission::Client,
    /// Peer-sharing client.
    pub peersharing: peersharing::Client,
}

impl PeerClient {
    /// Connect to `addr` and perform the N2N handshake using the given network magic.
    pub async fn connect(addr: impl ToSocketAddrs, magic: u64) -> Result<Self, Error> {
        let bearer = Bearer::connect_tcp(addr)
            .await
            .map_err(Error::ConnectFailure)?;

        let mut plexer = multiplexer::Plexer::new(bearer);

        let channel = plexer.subscribe_client(PROTOCOL_N2N_HANDSHAKE);
        let mut handshake = handshake::Client::new(channel);

        let cs_channel = plexer.subscribe_client(PROTOCOL_N2N_CHAIN_SYNC);
        let bf_channel = plexer.subscribe_client(PROTOCOL_N2N_BLOCK_FETCH);
        let txsub_channel = plexer.subscribe_client(PROTOCOL_N2N_TX_SUBMISSION);
        let peersharing_channel = plexer.subscribe_client(PROTOCOL_N2N_PEER_SHARING);

        let channel = plexer.subscribe_client(PROTOCOL_N2N_KEEP_ALIVE);
        let keepalive = keepalive::Client::new(channel);

        let plexer = plexer.spawn();

        let versions = handshake::n2n::VersionTable::v7_and_above(magic);

        let handshake = handshake
            .handshake(versions)
            .await
            .map_err(Error::HandshakeProtocol)?;

        if let handshake::Confirmation::Rejected(reason) = handshake {
            error!(?reason, "handshake refused");
            return Err(Error::IncompatibleVersion);
        }

        let keepalive = KeepAliveLoop::client(
            keepalive,
            Duration::from_secs(DEFAULT_KEEP_ALIVE_INTERVAL_SEC),
        )
        .spawn();

        let client = Self {
            plexer,
            keepalive,
            chainsync: chainsync::Client::new(cs_channel),
            blockfetch: blockfetch::Client::new(bf_channel),
            txsubmission: txsubmission::Client::new(txsub_channel),
            peersharing: peersharing::Client::new(peersharing_channel),
        };

        Ok(client)
    }

    /// Connect, issue a query-mode handshake, and return the peer's advertised
    /// version table without keeping the connection alive.
    pub async fn handshake_query(
        addr: impl ToSocketAddrs,
        magic: u64,
    ) -> Result<VersionTable<VersionData>, Error> {
        let bearer = Bearer::connect_tcp(addr)
            .await
            .map_err(Error::ConnectFailure)?;

        let mut plexer = multiplexer::Plexer::new(bearer);

        let channel = plexer.subscribe_client(PROTOCOL_N2N_HANDSHAKE);
        let mut handshake = handshake::Client::new(channel);

        let _plexer = plexer.spawn();

        let versions = handshake::n2n::VersionTable::v7_and_above_with_query(magic, true);

        let handshake = handshake
            .handshake(versions)
            .await
            .map_err(Error::HandshakeProtocol)?;

        let version_table = match handshake {
            handshake::Confirmation::QueryReply(version_table) => {
                debug!("handshake query reply received");
                version_table
            }
            handshake::Confirmation::Accepted(_, _) => {
                error!("handshake accepted when we expected query reply");
                return Err(Error::HandshakeProtocol(handshake::Error::InvalidInbound));
            }
            handshake::Confirmation::Rejected(reason) => {
                error!(?reason, "handshake refused");
                return Err(Error::IncompatibleVersion);
            }
        };

        Ok(version_table)
    }

    /// Get mutable access to the chain-sync client.
    pub fn chainsync(&mut self) -> &mut chainsync::N2NClient {
        &mut self.chainsync
    }

    /// Run an operation against the chain-sync client on a background task.
    pub async fn with_chainsync<T, O, Fut>(&mut self, op: T) -> tokio::task::JoinHandle<O>
    where
        T: FnOnce(&mut chainsync::N2NClient) -> Fut,
        Fut: std::future::Future<Output = O> + Send + 'static,
        O: Send + 'static,
    {
        tokio::spawn(op(&mut self.chainsync))
    }

    /// Get mutable access to the block-fetch client.
    pub fn blockfetch(&mut self) -> &mut blockfetch::Client {
        &mut self.blockfetch
    }

    /// Get mutable access to the tx-submission client.
    pub fn txsubmission(&mut self) -> &mut txsubmission::Client {
        &mut self.txsubmission
    }

    /// Get mutable access to the peer-sharing client.
    pub fn peersharing(&mut self) -> &mut peersharing::Client {
        &mut self.peersharing
    }

    /// Tear down the underlying multiplexer and abort all spawned tasks.
    pub async fn abort(self) {
        self.plexer.abort().await
    }
}

/// Node-to-node Ouroboros server. Accepts a peer connection and exposes the
/// server side of each mini-protocol carried over the bearer.
pub struct PeerServer {
    /// Underlying running multiplexer.
    pub plexer: RunningPlexer,
    /// Handshake server.
    pub handshake: handshake::N2NServer,
    /// Chain-sync server.
    pub chainsync: chainsync::N2NServer,
    /// Block-fetch server.
    pub blockfetch: blockfetch::Server,
    /// Tx-submission server.
    pub txsubmission: txsubmission::Server,
    /// Keep-alive server.
    pub keepalive: keepalive::Server,
    /// Peer-sharing server.
    pub peersharing: peersharing::Server,
    accepted_address: Option<SocketAddr>,
    accepted_version: Option<(u64, n2n::VersionData)>,
}

impl PeerServer {
    /// Build a server over an already-accepted bearer.
    pub fn new(bearer: Bearer) -> Self {
        let mut plexer = multiplexer::Plexer::new(bearer);

        let hs_channel = plexer.subscribe_server(PROTOCOL_N2N_HANDSHAKE);
        let cs_channel = plexer.subscribe_server(PROTOCOL_N2N_CHAIN_SYNC);
        let bf_channel = plexer.subscribe_server(PROTOCOL_N2N_BLOCK_FETCH);
        let txsub_channel = plexer.subscribe_server(PROTOCOL_N2N_TX_SUBMISSION);
        let keepalive_channel = plexer.subscribe_server(PROTOCOL_N2N_KEEP_ALIVE);
        let peersharing_channel = plexer.subscribe_server(PROTOCOL_N2N_PEER_SHARING);

        let hs = handshake::N2NServer::new(hs_channel);
        let cs = chainsync::N2NServer::new(cs_channel);
        let bf = blockfetch::Server::new(bf_channel);
        let txsub = txsubmission::Server::new(txsub_channel);
        let keepalive = keepalive::Server::new(keepalive_channel);
        let peersharing = peersharing::Server::new(peersharing_channel);

        let plexer = plexer.spawn();

        Self {
            plexer,
            handshake: hs,
            chainsync: cs,
            blockfetch: bf,
            txsubmission: txsub,
            keepalive,
            peersharing,
            accepted_address: None,
            accepted_version: None,
        }
    }

    /// Accept the next connection from `listener` and complete the N2N handshake.
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

        if let Some((version, data)) = accepted_version {
            client.accepted_address = Some(address);
            client.accepted_version = Some((version, data));
            Ok(client)
        } else {
            client.abort().await;
            Err(Error::IncompatibleVersion)
        }
    }

    /// Get mutable access to the handshake server.
    pub fn handshake(&mut self) -> &mut handshake::N2NServer {
        &mut self.handshake
    }

    /// Get mutable access to the chain-sync server.
    pub fn chainsync(&mut self) -> &mut chainsync::N2NServer {
        &mut self.chainsync
    }

    /// Get mutable access to the block-fetch server.
    pub fn blockfetch(&mut self) -> &mut blockfetch::Server {
        &mut self.blockfetch
    }

    /// Get mutable access to the tx-submission server.
    pub fn txsubmission(&mut self) -> &mut txsubmission::Server {
        &mut self.txsubmission
    }

    /// Get mutable access to the keep-alive server.
    pub fn keepalive(&mut self) -> &mut keepalive::Server {
        &mut self.keepalive
    }

    /// Get mutable access to the peer-sharing server.
    pub fn peersharing(&mut self) -> &mut peersharing::Server {
        &mut self.peersharing
    }

    /// Remote socket address of the accepted peer.
    pub fn accepted_address(&self) -> Option<&SocketAddr> {
        self.accepted_address.as_ref()
    }

    /// Version number negotiated with the accepted peer plus its version data.
    pub fn accepted_version(&self) -> Option<&(u64, n2n::VersionData)> {
        self.accepted_version.as_ref()
    }

    /// Tear down the underlying multiplexer.
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
    submission: localtxsubmission::Client,
    monitor: txmonitor::Client,
}

impl NodeClient {
    /// Build a client over an already-opened bearer (does not perform the handshake).
    pub fn new(bearer: Bearer) -> Self {
        let mut plexer = multiplexer::Plexer::new(bearer);

        let hs_channel = plexer.subscribe_client(PROTOCOL_N2C_HANDSHAKE);
        let cs_channel = plexer.subscribe_client(PROTOCOL_N2C_CHAIN_SYNC);
        let sq_channel = plexer.subscribe_client(PROTOCOL_N2C_STATE_QUERY);
        let tx_channel = plexer.subscribe_client(PROTOCOL_N2C_TX_SUBMISSION);
        let mo_channel = plexer.subscribe_client(PROTOCOL_N2C_TX_MONITOR);

        let plexer = plexer.spawn();

        Self {
            plexer,
            handshake: handshake::Client::new(hs_channel),
            chainsync: chainsync::Client::new(cs_channel),
            statequery: localstate::Client::new(sq_channel),
            submission: localtxsubmission::Client::new(tx_channel),
            monitor: txmonitor::Client::new(mo_channel),
        }
    }

    /// Connect to a Unix-domain node socket at `path` and perform the N2C handshake.
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

    /// Connect to a Windows named-pipe node socket and perform the N2C handshake.
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

    /// Issue a query-mode handshake over `bearer` and return the node's
    /// advertised version table.
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

    /// Get mutable access to the handshake client.
    pub fn handshake(&mut self) -> &mut handshake::N2CClient {
        &mut self.handshake
    }

    /// Get mutable access to the chain-sync client (N2C).
    pub fn chainsync(&mut self) -> &mut chainsync::N2CClient {
        &mut self.chainsync
    }

    /// Get mutable access to the local-state-query client.
    pub fn statequery(&mut self) -> &mut localstate::Client {
        &mut self.statequery
    }

    /// Get mutable access to the local-tx-submission client.
    pub fn submission(&mut self) -> &mut localtxsubmission::Client {
        &mut self.submission
    }

    /// Get mutable access to the tx-monitor client.
    pub fn monitor(&mut self) -> &mut txmonitor::Client {
        &mut self.monitor
    }

    /// Tear down the underlying multiplexer.
    pub async fn abort(self) {
        self.plexer.abort().await
    }
}

/// Node-to-client Ouroboros server (the node side of a local connection).
#[cfg(unix)]
pub struct NodeServer {
    /// Underlying running multiplexer.
    pub plexer: RunningPlexer,
    /// Handshake server.
    pub handshake: handshake::N2CServer,
    /// Chain-sync server (N2C).
    pub chainsync: chainsync::N2CServer,
    /// Local-state-query server.
    pub statequery: localstate::Server,
    /// Local-tx-submission server.
    pub localtxsubmission: localtxsubmission::Server,
    accepted_address: Option<UnixSocketAddr>,
    accpeted_version: Option<(VersionNumber, n2c::VersionData)>,
}

#[cfg(unix)]
impl NodeServer {
    /// Build a server over an already-accepted bearer.
    pub async fn new(bearer: Bearer) -> Self {
        let mut plexer = multiplexer::Plexer::new(bearer);

        let hs_channel = plexer.subscribe_server(PROTOCOL_N2C_HANDSHAKE);
        let cs_channel = plexer.subscribe_server(PROTOCOL_N2C_CHAIN_SYNC);
        let sq_channel = plexer.subscribe_server(PROTOCOL_N2C_STATE_QUERY);
        let localtx_channel = plexer.subscribe_server(PROTOCOL_N2C_TX_SUBMISSION);

        let server_hs = handshake::Server::<n2c::VersionData>::new(hs_channel);
        let server_cs = chainsync::N2CServer::new(cs_channel);
        let server_sq = localstate::Server::new(sq_channel);
        let server_localtx = localtxsubmission::Server::new(localtx_channel);

        let plexer = plexer.spawn();

        Self {
            plexer,
            handshake: server_hs,
            chainsync: server_cs,
            statequery: server_sq,
            localtxsubmission: server_localtx,
            accepted_address: None,
            accpeted_version: None,
        }
    }

    /// Accept the next Unix-domain connection from `listener` and complete the N2C handshake.
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

    /// Get mutable access to the handshake server.
    pub fn handshake(&mut self) -> &mut handshake::N2CServer {
        &mut self.handshake
    }

    /// Get mutable access to the chain-sync server (N2C).
    pub fn chainsync(&mut self) -> &mut chainsync::N2CServer {
        &mut self.chainsync
    }

    /// Get mutable access to the local-state-query server.
    pub fn statequery(&mut self) -> &mut localstate::Server {
        &mut self.statequery
    }

    /// Get mutable access to the local-tx-submission server.
    pub fn localtxsubmission(&mut self) -> &mut localtxsubmission::Server {
        &mut self.localtxsubmission
    }

    /// Remote address of the accepted local client.
    pub fn accepted_address(&self) -> Option<&UnixSocketAddr> {
        self.accepted_address.as_ref()
    }

    /// Version negotiated with the accepted local client.
    pub fn accepted_version(&self) -> Option<&(u64, n2c::VersionData)> {
        self.accpeted_version.as_ref()
    }

    /// Tear down the underlying multiplexer.
    pub async fn abort(self) {
        self.plexer.abort().await
    }
}

/// Client of N2C DMQ (Decentralized Message Queue)
///
/// Described in [CIP-0137](https://github.com/cardano-foundation/CIPs/tree/master/CIP-0137)
pub struct DmqClient {
    plexer: RunningPlexer,
    handshake: handshake::N2CClient,
    msg_submission: localmsgsubmission::Client,
    msg_notification: localmsgnotification::Client,
}

impl DmqClient {
    /// Build a DMQ client over an already-opened bearer.
    pub fn new(bearer: Bearer) -> Self {
        let mut plexer = multiplexer::Plexer::new(bearer);

        let hs_channel = plexer.subscribe_client(PROTOCOL_N2C_HANDSHAKE);
        let msg_submission_channel = plexer.subscribe_client(PROTOCOL_N2C_MSG_SUBMISSION);
        let msg_notification_channel = plexer.subscribe_client(PROTOCOL_N2C_MSG_NOTIFICATION);

        let plexer = plexer.spawn();

        Self {
            plexer,
            handshake: handshake::Client::new(hs_channel),
            msg_submission: localmsgsubmission::Client::new(msg_submission_channel),
            msg_notification: localmsgnotification::Client::new(msg_notification_channel),
        }
    }

    /// Connect to a DMQ node socket at `path` and perform the DMQ handshake.
    #[cfg(unix)]
    pub async fn connect(path: impl AsRef<Path>, magic: u64) -> Result<Self, Error> {
        let bearer = Bearer::connect_unix(path)
            .await
            .map_err(Error::ConnectFailure)?;

        let mut client = Self::new(bearer);

        let versions = handshake::n2c::VersionTable::dmq(magic);

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

    /// Connect to a DMQ node over a Windows named pipe and perform the handshake.
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

    /// Issue a DMQ query-mode handshake and return the node's advertised version table.
    #[cfg(unix)]
    pub async fn handshake_query(
        bearer: Bearer,
        magic: u64,
    ) -> Result<handshake::n2c::VersionTable, Error> {
        let mut plexer = multiplexer::Plexer::new(bearer);

        let hs_channel = plexer.subscribe_client(PROTOCOL_N2C_HANDSHAKE);

        let plexer = plexer.spawn();

        let versions = handshake::n2c::VersionTable::dmq(magic);
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

    /// Get mutable access to the handshake client.
    pub fn handshake(&mut self) -> &mut handshake::N2CClient {
        &mut self.handshake
    }

    /// Get mutable access to the DMQ message-submission client.
    pub fn msg_submission(&mut self) -> &mut localmsgsubmission::Client {
        &mut self.msg_submission
    }

    /// Get mutable access to the DMQ message-notification client.
    pub fn msg_notification(&mut self) -> &mut localmsgnotification::Client {
        &mut self.msg_notification
    }

    /// Tear down the underlying multiplexer.
    pub async fn abort(self) {
        self.plexer.abort().await
    }
}

/// Server of N2C DMQ (Decentralized Message Queue)
///
/// Described in [CIP-0137](https://github.com/cardano-foundation/CIPs/tree/master/CIP-0137)
#[cfg(unix)]
pub struct DmqServer {
    /// Underlying running multiplexer.
    pub plexer: RunningPlexer,
    /// Handshake server.
    pub handshake: handshake::N2CServer,
    /// DMQ message-notification server.
    pub msg_notification: localmsgnotification::Server,
    /// DMQ message-submission server.
    pub msg_submission: localmsgsubmission::Server,
    accepted_address: Option<UnixSocketAddr>,
    accpeted_version: Option<(VersionNumber, n2c::VersionData)>,
}

#[cfg(unix)]
impl DmqServer {
    /// Build a DMQ server over an already-accepted bearer.
    pub async fn new(bearer: Bearer) -> Self {
        let mut plexer = multiplexer::Plexer::new(bearer);

        let hs_channel = plexer.subscribe_server(PROTOCOL_N2C_HANDSHAKE);
        let msg_notification_channel = plexer.subscribe_server(PROTOCOL_N2C_MSG_NOTIFICATION);
        let msg_submission_channel = plexer.subscribe_server(PROTOCOL_N2C_MSG_SUBMISSION);

        let server_hs = handshake::Server::<n2c::VersionData>::new(hs_channel);
        let server_msg_notification = localmsgnotification::Server::new(msg_notification_channel);
        let server_msg_submission = localmsgsubmission::Server::new(msg_submission_channel);

        let plexer = plexer.spawn();

        Self {
            plexer,
            handshake: server_hs,
            msg_notification: server_msg_notification,
            msg_submission: server_msg_submission,
            accepted_address: None,
            accpeted_version: None,
        }
    }

    /// Accept the next DMQ connection from `listener` and complete the handshake.
    pub async fn accept(listener: &UnixListener, magic: u64) -> Result<Self, Error> {
        let (bearer, address) = Bearer::accept_unix(listener)
            .await
            .map_err(Error::ConnectFailure)?;

        let mut client = Self::new(bearer).await;

        let accepted_version = client
            .handshake()
            .handshake(n2c::VersionTable::dmq(magic))
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

    /// Get mutable access to the handshake server.
    pub fn handshake(&mut self) -> &mut handshake::N2CServer {
        &mut self.handshake
    }

    /// Get mutable access to the DMQ message-notification server.
    pub fn msg_notification(&mut self) -> &mut localmsgnotification::Server {
        &mut self.msg_notification
    }

    /// Get mutable access to the DMQ message-submission server.
    pub fn msg_submission(&mut self) -> &mut localmsgsubmission::Server {
        &mut self.msg_submission
    }

    /// Remote address of the accepted DMQ client.
    pub fn accepted_address(&self) -> Option<&UnixSocketAddr> {
        self.accepted_address.as_ref()
    }

    /// Version negotiated with the accepted DMQ client.
    pub fn accepted_version(&self) -> Option<&(u64, n2c::VersionData)> {
        self.accpeted_version.as_ref()
    }

    /// Tear down the underlying multiplexer.
    pub async fn abort(self) {
        self.plexer.abort().await
    }
}
