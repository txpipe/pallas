use futures::{Stream, StreamExt};
use std::{
    collections::HashSet,
    str::FromStr,
    sync::{atomic::AtomicBool, Arc},
    time::{Duration, Instant},
};

use tracing::{debug, info, instrument, trace};

use crate::miniprotocols::{
    blockfetch,
    chainsync::{self, HeaderContent},
    handshake, keepalive, peersharing,
    txsubmission::{self, EraTxBody, EraTxId},
};

pub mod behaviors;
pub mod netio;

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct PeerId {
    host: String,
    port: u16,
}

impl FromStr for PeerId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Split host:port
        let (host, port) = s
            .rsplit_once(':')
            .ok_or_else(|| "Missing port in address".to_string())?;

        let port: u16 = port.parse().map_err(|e| format!("Invalid port: {}", e))?;

        Ok(PeerId {
            host: host.to_string(),
            port,
        })
    }
}

impl std::fmt::Display for PeerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.host, self.port)
    }
}

#[derive(Clone, Debug)]
struct PeerHandle {
    id: PeerId,
    is_connected: bool,
    span: tracing::Span,
    tags: std::collections::HashSet<PeerTag>,
}

impl PeerHandle {
    fn new(pid: PeerId, tags: Vec<PeerTag>) -> Self {
        Self {
            id: pid.clone(),
            is_connected: false,
            tags: HashSet::from_iter(tags),
            span: tracing::info_span!("peer", pid = %pid),
        }
    }

    fn is_connected(&self) -> bool {
        self.is_connected
    }

    fn has_tag(&self, tag: impl Into<PeerTag>) -> bool {
        self.tags.contains(&tag.into())
    }

    fn add_tag(&mut self, tag: impl Into<PeerTag>) {
        self.tags.insert(tag.into());
    }

    fn remove_tag(&mut self, tag: impl Into<PeerTag>) {
        self.tags.remove(&tag.into());
    }

    fn switch_tag(&mut self, from: impl Into<PeerTag>, to: impl Into<PeerTag>) {
        self.tags.remove(&from.into());
        self.tags.insert(to.into());
    }
}

#[derive(Debug)]
pub enum InitiatorState {
    Handshake(handshake::State<handshake::n2n::VersionData>),
    PeerSharing(peersharing::State),
    KeepAlive(keepalive::State),
}

impl From<handshake::State<handshake::n2n::VersionData>> for InitiatorState {
    fn from(state: handshake::State<handshake::n2n::VersionData>) -> Self {
        InitiatorState::Handshake(state)
    }
}

impl From<peersharing::State> for InitiatorState {
    fn from(state: peersharing::State) -> Self {
        InitiatorState::PeerSharing(state)
    }
}

impl From<keepalive::State> for InitiatorState {
    fn from(state: keepalive::State) -> Self {
        InitiatorState::KeepAlive(state)
    }
}

#[derive(Debug)]
pub enum NetworkEvent {
    PeerDiscovered(PeerId),
    PeerConnected(PeerId),
    PeerDisconnected(PeerId),
    PeerTagged(PeerId, PeerTag),
    InitiatorStateChange(PeerId, InitiatorState),
    MessageSent(PeerId, AnyMessage),
    Error(Error),
}

#[derive(Debug)]
pub struct State {
    pub peers: dashmap::DashMap<PeerId, PeerHandle>,
}

impl State {
    fn new() -> Self {
        Self {
            peers: dashmap::DashMap::new(),
        }
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

type SharedState = std::sync::Arc<State>;

#[derive(Debug, Clone)]
pub enum AnyMessage {
    Handshake(
        crate::miniprotocols::handshake::Message<crate::miniprotocols::handshake::n2n::VersionData>,
    ),
    ChainSync(
        crate::miniprotocols::chainsync::Message<crate::miniprotocols::chainsync::HeaderContent>,
    ),
    BlockFetch(crate::miniprotocols::blockfetch::Message),
    TxSubmit(
        crate::miniprotocols::txsubmission::Message<
            crate::miniprotocols::txsubmission::EraTxId,
            crate::miniprotocols::txsubmission::EraTxBody,
        >,
    ),
    PeerSharing(crate::miniprotocols::peersharing::Message),
    KeepAlive(crate::miniprotocols::keepalive::Message),
}

pub type PeerVersion = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PeerTag {
    /// Peer is connected and actively exchanging messages
    Hot,

    /// Peer is connected but not actively exchanging messages
    Warm,

    /// Peer has no connections at all
    Cold,

    /// Peer is trusted
    Trusted,

    /// Peer accepted us
    Accepted(PeerVersion),

    /// Peer rejected us
    Rejected,

    /// Peer supports peer sharing
    PeerSharing,

    /// Peer seen alive at
    SeenAlive(Instant),
}

impl std::fmt::Display for PeerTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PeerTag::Hot => write!(f, "hot"),
            PeerTag::Warm => write!(f, "warm"),
            PeerTag::Cold => write!(f, "cold"),
            PeerTag::Trusted => write!(f, "trusted"),
            PeerTag::Accepted(version) => write!(f, "accepted({})", version),
            PeerTag::Rejected => write!(f, "rejected"),
            PeerTag::PeerSharing => write!(f, "peer-sharing"),
            PeerTag::SeenAlive(instant) => write!(f, "seen-alive"),
        }
    }
}

#[derive(Debug)]
pub enum IntrinsicCommand {
    TrackNewPeer(PeerId, Vec<PeerTag>),
    ConnectPeer(PeerId),
    SendMessage(PeerId, AnyMessage),
    AddPeerTag(PeerId, PeerTag),
    SwitchPeerTag(PeerId, PeerTag, PeerTag),
}

impl IntrinsicCommand {
    fn pid(&self) -> Option<PeerId> {
        match self {
            IntrinsicCommand::ConnectPeer(pid) => Some(pid.clone()),
            IntrinsicCommand::SendMessage(pid, _) => Some(pid.clone()),
            IntrinsicCommand::AddPeerTag(pid, _) => Some(pid.clone()),
            IntrinsicCommand::SwitchPeerTag(pid, _, _) => Some(pid.clone()),
            IntrinsicCommand::TrackNewPeer(pid, _) => Some(pid.clone()),
        }
    }
}

pub trait Behavior: Send + Sync {
    fn backoff(&self) -> Option<Duration>;
    fn next(&mut self, state: &State) -> impl Iterator<Item = IntrinsicCommand> + Send + Sync;
    fn handle(&mut self, event: &NetworkEvent);
}

const MAX_CONCURRENT_COMMANDS: usize = 10;

#[derive(Debug, thiserror::Error)]
pub enum AnyClientError {
    #[error("client error")]
    ClientError(#[from] crate::miniprotocols::Error),

    #[error("chainsync client error")]
    ChainSyncError(#[from] crate::miniprotocols::chainsync::ClientError),

    #[error("blockfetch client error")]
    BlockFetchError(#[from] crate::miniprotocols::blockfetch::ClientError),

    #[error("txsubmission client error")]
    TxSubmissionError(#[from] crate::miniprotocols::txsubmission::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("peer not found")]
    PeerNotFound,

    #[error("peer in use")]
    PeerInUse,

    #[error("peer not outbound")]
    PeerNotOutbound,

    #[error("peer not connected")]
    PeerNotConnected,

    #[error("address resolution failed for peer {0}")]
    AddressResolutionFailed(PeerId),

    #[error(transparent)]
    ClientError(#[from] AnyClientError),

    #[error("io error")]
    IoError(#[from] std::io::Error),
}

pub type InboxSend = tokio::sync::mpsc::Sender<NetworkEvent>;
pub type InboxRecv = tokio::sync::mpsc::Receiver<NetworkEvent>;

pub struct Actuator {
    state: SharedState,
    inbox: InboxSend,
    plexer: netio::MegaPlexer,
}

impl Actuator {
    fn new(state: SharedState, inbox: InboxSend) -> Self {
        Self {
            state,
            inbox: inbox.clone(),
            plexer: netio::MegaPlexer::new(inbox),
        }
    }

    async fn notify(&self, event: NetworkEvent) {
        trace!("notifying");
        self.inbox.send(event).await.unwrap();
    }

    pub async fn execute(&self, cmd: &IntrinsicCommand) -> Result<(), Error> {
        match cmd {
            IntrinsicCommand::TrackNewPeer(pid, tags) => {
                info!(pid = %pid, "tracking new peer");

                if self.state.peers.contains_key(&pid) {
                    info!(pid = %pid, "peer already tracked");
                    return Ok(());
                }

                self.state
                    .peers
                    .insert(pid.clone(), PeerHandle::new(pid.clone(), tags.clone()));

                self.notify(NetworkEvent::PeerDiscovered(pid.clone())).await;

                Ok(())
            }
            IntrinsicCommand::SwitchPeerTag(pid, from, to) => {
                info!(%pid, from = %from, to = %to, "switching tag");

                let mut peer = self.state.peers.get_mut(&pid).ok_or(Error::PeerNotFound)?;

                peer.switch_tag(*from, *to);

                self.notify(NetworkEvent::PeerTagged(pid.clone(), *to))
                    .await;

                Ok(())
            }
            IntrinsicCommand::AddPeerTag(pid, tag) => {
                info!(%pid, %tag, "adding tag to peer");

                let mut peer = self.state.peers.get_mut(&pid).ok_or(Error::PeerNotFound)?;

                if peer.has_tag(*tag) {
                    info!(%pid, %tag, "peer already has tag");
                    return Ok(());
                }

                peer.add_tag(*tag);

                self.notify(NetworkEvent::PeerTagged(pid.clone(), *tag))
                    .await;

                Ok(())
            }

            IntrinsicCommand::ConnectPeer(pid) => {
                info!(%pid, "connecting peer");

                let mut peer = self.state.peers.get_mut(&pid).ok_or(Error::PeerNotFound)?;

                if peer.is_connected() {
                    info!(%pid, "peer already connected");
                    return Ok(());
                }

                self.plexer.connect_peer(&pid).await?;

                peer.is_connected = true;

                self.notify(NetworkEvent::PeerConnected(pid.clone())).await;

                Ok(())
            }
            IntrinsicCommand::SendMessage(pid, any_message) => {
                info!(%pid, "sending message");

                self.plexer.send_message(&pid, any_message.clone()).await?;

                self.notify(NetworkEvent::MessageSent(pid.clone(), any_message.clone()))
                    .await;

                Ok(())
            }
            x => {
                dbg!(x);
                todo!()
            }
        }
    }

    #[instrument(skip_all)]
    pub async fn run_sprint(&self, cmds: Vec<IntrinsicCommand>) {
        futures::stream::iter(cmds.iter())
            .map(|cmd| self.execute(cmd))
            .buffer_unordered(MAX_CONCURRENT_COMMANDS)
            .collect::<Vec<_>>()
            .await;
    }
}

pub struct Manager<B: Behavior + Send + 'static> {
    behavior: B,
    state: SharedState,
    actuator: Actuator,
    inbox_recv: InboxRecv,
}

impl<B: Behavior + Send + 'static> Manager<B> {
    pub fn new(behavior: B) -> Self {
        let (inbox_send, inbox_recv) = tokio::sync::mpsc::channel(100);
        let state = SharedState::default();

        Self {
            actuator: Actuator::new(state.clone(), inbox_send),
            behavior,
            state,
            inbox_recv,
        }
    }

    async fn notify_loop(behavior: &mut B, inbox_recv: &mut InboxRecv) {
        loop {
            let event = inbox_recv.recv().await.unwrap();
            debug!("received event");
            behavior.handle(&event);
        }
    }

    pub async fn run(self) {
        let Self {
            mut behavior,
            state,
            actuator,
            mut inbox_recv,
        } = self;

        loop {
            let backoff = behavior.backoff();

            if let Some(backoff) = backoff {
                info!(millis = %backoff.as_millis(), "backing off");

                tokio::select! {
                    _ = tokio::time::sleep(backoff) => {
                        continue;
                    }
                    _ = Self::notify_loop(&mut behavior, &mut inbox_recv) => {
                        continue;
                    }
                }
            } else {
                let cmds = behavior.next(&state).collect::<Vec<_>>();

                debug!(cmds = %cmds.len(), "starting sprint");

                tokio::select! {
                    _ = actuator.run_sprint(cmds) => {
                        debug!("sprint completed");
                        continue;
                    }
                    _ = Self::notify_loop(&mut behavior, &mut inbox_recv) => {
                        continue;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn manager_smoke_test() {
        let behavior = behaviors::PeerPromotionBehavior::new(behaviors::PeerPromotionConfig {
            desired_hot_peers: (10, 10),
            desired_warm_peers: (10, 10),
            desired_cold_peers: (10, 10),
            trusted_peers: vec![PeerId::from_str("127.0.0.1:8080").unwrap()],
        });

        let manager = Manager::new(behavior);
        manager.run().await
    }
}
