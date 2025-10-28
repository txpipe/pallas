use std::net::SocketAddr;
use std::path::Path;
use std::time::Duration;
use thiserror::Error;
use tracing::{debug, error};

use tokio::net::{TcpListener, ToSocketAddrs};

#[cfg(unix)]
use tokio::net::{unix::SocketAddr as UnixSocketAddr, UnixListener};

use crate::miniprotocols::handshake::n2n::VersionData;
use crate::miniprotocols::handshake::{n2c, n2n, Confirmation, VersionNumber, VersionTable};

use crate::miniprotocols::{
    blockfetch, chainsync, handshake, keepalive, localmsgnotification, localmsgsubmission,
    localstate, localtxsubmission, peersharing, txmonitor, txsubmission, PROTOCOL_N2C_CHAIN_SYNC,
    PROTOCOL_N2C_HANDSHAKE, PROTOCOL_N2C_MSG_NOTIFICATION, PROTOCOL_N2C_MSG_SUBMISSION,
    PROTOCOL_N2C_STATE_QUERY, PROTOCOL_N2C_TX_MONITOR, PROTOCOL_N2C_TX_SUBMISSION,
    PROTOCOL_N2N_BLOCK_FETCH, PROTOCOL_N2N_CHAIN_SYNC, PROTOCOL_N2N_HANDSHAKE,
    PROTOCOL_N2N_KEEP_ALIVE, PROTOCOL_N2N_PEER_SHARING, PROTOCOL_N2N_TX_SUBMISSION,
};

#[cfg(feature = "leios")]
use crate::miniprotocols::{
    leiosfetch, leiosnotify, PROTOCOL_N2N_LEIOS_FETCH, PROTOCOL_N2N_LEIOS_NOTIFY,
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

    #[error("keepalive client loop error")]
    KeepAliveClientLoop(keepalive::ClientError),

    #[error("keepalive server loop error")]
    KeepAliveServerLoop(keepalive::ServerError),

    #[error("handshake version not accepted")]
    IncompatibleVersion,
}

pub const DEFAULT_KEEP_ALIVE_INTERVAL_SEC: u64 = 20;

pub type KeepAliveHandle = tokio::task::JoinHandle<Result<(), Error>>;

pub enum KeepAliveLoop {
    Client(keepalive::Client, Duration),
    Server(keepalive::Server),
}

impl KeepAliveLoop {
    pub fn client(client: keepalive::Client, interval: Duration) -> Self {
        Self::Client(client, interval)
    }

    pub fn server(server: keepalive::Server) -> Self {
        Self::Server(server)
    }

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

    pub async fn run_server(mut server: keepalive::Server) -> Result<(), Error> {
        loop {
            debug!("waiting keepalive request");

            server
                .keepalive_roundtrip()
                .await
                .map_err(Error::KeepAliveServerLoop)?;
        }
    }

    pub fn spawn(self) -> KeepAliveHandle {
        match self {
            KeepAliveLoop::Client(client, interval) => {
                tokio::spawn(Self::run_client(client, interval))
            }
            KeepAliveLoop::Server(server) => tokio::spawn(Self::run_server(server)),
        }
    }
}

/// Client of N2N Ouroboros
pub struct PeerClient {
    pub plexer: RunningPlexer,
    pub keepalive: KeepAliveHandle,
    pub chainsync: chainsync::N2NClient,
    pub blockfetch: blockfetch::Client,
    pub txsubmission: txsubmission::Client,
    pub peersharing: peersharing::Client,
    #[cfg(feature = "leios")]
    pub leiosnotify: leiosnotify::Client,
    #[cfg(feature = "leios")]
    pub leiosfetch: leiosfetch::Client,
}

impl PeerClient {
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
        #[cfg(feature = "leios")]
        let leiosnotify_channel = plexer.subscribe_client(PROTOCOL_N2N_LEIOS_NOTIFY);
        #[cfg(feature = "leios")]
        let leiosfetch_channel = plexer.subscribe_client(PROTOCOL_N2N_LEIOS_FETCH);

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
            #[cfg(feature = "leios")]
            leiosnotify: leiosnotify::Client::new(leiosnotify_channel),
            #[cfg(feature = "leios")]
            leiosfetch: leiosfetch::Client::new(leiosfetch_channel),
        };

        Ok(client)
    }

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

    pub fn peersharing(&mut self) -> &mut peersharing::Client {
        &mut self.peersharing
    }

    #[cfg(feature = "leios")]
    pub fn leiosnotify(&mut self) -> &mut leiosnotify::Client {
        &mut self.leiosnotify
    }

    #[cfg(feature = "leios")]
    pub fn leiosfetch(&mut self) -> &mut leiosfetch::Client {
        &mut self.leiosfetch
    }

    pub async fn abort(self) {
        self.plexer.abort().await
    }
}

/// Server of N2N Ouroboros
pub struct PeerServer {
    pub plexer: RunningPlexer,
    pub handshake: handshake::N2NServer,
    pub chainsync: chainsync::N2NServer,
    pub blockfetch: blockfetch::Server,
    pub txsubmission: txsubmission::Server,
    pub keepalive: keepalive::Server,
    pub peersharing: peersharing::Server,
    #[cfg(feature = "leios")]
    pub leiosnotify: leiosnotify::Server,
    #[cfg(feature = "leios")]
    pub leiosfetch: leiosfetch::Server,
    accepted_address: Option<SocketAddr>,
    accepted_version: Option<(u64, n2n::VersionData)>,
}

impl PeerServer {
    pub fn new(bearer: Bearer) -> Self {
        let mut plexer = multiplexer::Plexer::new(bearer);

        let hs_channel = plexer.subscribe_server(PROTOCOL_N2N_HANDSHAKE);
        let cs_channel = plexer.subscribe_server(PROTOCOL_N2N_CHAIN_SYNC);
        let bf_channel = plexer.subscribe_server(PROTOCOL_N2N_BLOCK_FETCH);
        let txsub_channel = plexer.subscribe_server(PROTOCOL_N2N_TX_SUBMISSION);
        let keepalive_channel = plexer.subscribe_server(PROTOCOL_N2N_KEEP_ALIVE);
        let peersharing_channel = plexer.subscribe_server(PROTOCOL_N2N_PEER_SHARING);
        #[cfg(feature = "leios")]
        let leiosnotify_channel = plexer.subscribe_server(PROTOCOL_N2N_LEIOS_NOTIFY);
        #[cfg(feature = "leios")]
        let leiosfetch_channel = plexer.subscribe_server(PROTOCOL_N2N_LEIOS_FETCH);

        let hs = handshake::N2NServer::new(hs_channel);
        let cs = chainsync::N2NServer::new(cs_channel);
        let bf = blockfetch::Server::new(bf_channel);
        let txsub = txsubmission::Server::new(txsub_channel);
        let keepalive = keepalive::Server::new(keepalive_channel);
        let peersharing = peersharing::Server::new(peersharing_channel);
        #[cfg(feature = "leios")]
        let leiosnotify = leiosnotify::Server::new(leiosnotify_channel);
        #[cfg(feature = "leios")]
        let leiosfetch = leiosfetch::Server::new(leiosfetch_channel);

        let plexer = plexer.spawn();

        Self {
            plexer,
            handshake: hs,
            chainsync: cs,
            blockfetch: bf,
            txsubmission: txsub,
            keepalive,
            peersharing,
            #[cfg(feature = "leios")]
            leiosnotify,
            #[cfg(feature = "leios")]
            leiosfetch,
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

        if let Some((version, data)) = accepted_version {
            client.accepted_address = Some(address);
            client.accepted_version = Some((version, data));
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

    pub fn keepalive(&mut self) -> &mut keepalive::Server {
        &mut self.keepalive
    }

    pub fn peersharing(&mut self) -> &mut peersharing::Server {
        &mut self.peersharing
    }

    #[cfg(feature = "leios")]
    pub fn leiosnotify(&mut self) -> &mut leiosnotify::Server {
        &mut self.leiosnotify
    }

    #[cfg(feature = "leios")]
    pub fn leiosfetch(&mut self) -> &mut leiosfetch::Server {
        &mut self.leiosfetch
    }

    pub fn accepted_address(&self) -> Option<&SocketAddr> {
        self.accepted_address.as_ref()
    }

    pub fn accepted_version(&self) -> Option<&(u64, n2n::VersionData)> {
        self.accepted_version.as_ref()
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
    submission: localtxsubmission::Client,
    monitor: txmonitor::Client,
}

impl NodeClient {
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

    pub fn submission(&mut self) -> &mut localtxsubmission::Client {
        &mut self.submission
    }

    pub fn monitor(&mut self) -> &mut txmonitor::Client {
        &mut self.monitor
    }

    pub async fn abort(self) {
        self.plexer.abort().await
    }
}

/// Server of N2C Ouroboros.
#[cfg(unix)]
pub struct NodeServer {
    pub plexer: RunningPlexer,
    pub handshake: handshake::N2CServer,
    pub chainsync: chainsync::N2CServer,
    pub statequery: localstate::Server,
    pub localtxsubmission: localtxsubmission::Server,
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

    pub fn localtxsubmission(&mut self) -> &mut localtxsubmission::Server {
        &mut self.localtxsubmission
    }

    pub fn accepted_address(&self) -> Option<&UnixSocketAddr> {
        self.accepted_address.as_ref()
    }

    pub fn accepted_version(&self) -> Option<&(u64, n2c::VersionData)> {
        self.accpeted_version.as_ref()
    }

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

    pub fn handshake(&mut self) -> &mut handshake::N2CClient {
        &mut self.handshake
    }

    pub fn msg_submission(&mut self) -> &mut localmsgsubmission::Client {
        &mut self.msg_submission
    }

    pub fn msg_notification(&mut self) -> &mut localmsgnotification::Client {
        &mut self.msg_notification
    }

    pub async fn abort(self) {
        self.plexer.abort().await
    }
}

/// Server of N2C DMQ (Decentralized Message Queue)
///
/// Described in [CIP-0137](https://github.com/cardano-foundation/CIPs/tree/master/CIP-0137)
#[cfg(unix)]
pub struct DmqServer {
    pub plexer: RunningPlexer,
    pub handshake: handshake::N2CServer,
    pub msg_notification: localmsgnotification::Server,
    pub msg_submission: localmsgsubmission::Server,
    accepted_address: Option<UnixSocketAddr>,
    accpeted_version: Option<(VersionNumber, n2c::VersionData)>,
}

#[cfg(unix)]
impl DmqServer {
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

    pub fn handshake(&mut self) -> &mut handshake::N2CServer {
        &mut self.handshake
    }

    pub fn msg_notification(&mut self) -> &mut localmsgnotification::Server {
        &mut self.msg_notification
    }

    pub fn msg_submission(&mut self) -> &mut localmsgsubmission::Server {
        &mut self.msg_submission
    }

    pub fn accepted_address(&self) -> Option<&UnixSocketAddr> {
        self.accepted_address.as_ref()
    }

    pub fn accepted_version(&self) -> Option<&(u64, n2c::VersionData)> {
        self.accpeted_version.as_ref()
    }

    pub async fn abort(self) {
        self.plexer.abort().await
    }
}
