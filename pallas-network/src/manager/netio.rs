use std::sync::Arc;
use std::{fmt::Debug, time::Duration};
use tracing::{debug, info, instrument, trace, warn};

use crate::{
    miniprotocols::{handshake, keepalive, peersharing, Agent, PlexerAdapter},
    multiplexer::{Plexer, RunningPlexer},
};

use super::{AnyMessage, Error, InboxSend, InitiatorState, NetworkEvent, PeerId};

pub type OutboxRecv<T> = tokio::sync::mpsc::Receiver<T>;
pub type OutboxSend<T> = tokio::sync::mpsc::Sender<T>;

pub type HsMsg = handshake::Message<handshake::n2n::VersionData>;

async fn link_agent_io<A>(
    pid: PeerId,
    mut agent: PlexerAdapter<A>,
    inbox: InboxSend,
    mut outbox: OutboxRecv<A::Message>,
    span: tracing::Span,
) where
    A: Agent,
    A::State: Clone + Debug,
    A::Message: Debug,
    InitiatorState: From<A::State>,
{
    let _span = span.enter();

    inbox
        .send(NetworkEvent::InitiatorStateChange(
            pid.clone(),
            InitiatorState::from(agent.state().clone()),
        ))
        .await
        .unwrap();

    loop {
        if agent.is_done() {
            let _span = span.enter();
            debug!(%pid, "agent loop complete");
            break;
        } else if !agent.has_agency() {
            let _span = span.enter();
            warn!("they have agency, waiting for peer message");

            if let Err(e) = agent.recv().await {
                warn!(%pid, "agent recv error: {:?}", e);
                break;
            }

            let state = agent.state();

            inbox
                .send(NetworkEvent::InitiatorStateChange(
                    pid.clone(),
                    InitiatorState::from(state.clone()),
                ))
                .await
                .unwrap();
        } else {
            let _span = span.enter();

            trace!("we have agency, waiting for intrinsic message");
            let msg = outbox.recv().await.unwrap();

            trace!("sending agent message");
            agent.send(&msg).await.unwrap();
        }
    }
}

pub struct AgentIO<A: Agent> {
    _io: tokio::task::JoinHandle<()>,
    outbox: OutboxSend<A::Message>,
}

impl<A: Agent> AgentIO<A> {
    pub fn new(
        pid: PeerId,
        plexer: &mut Plexer,
        protocol_id: u16,
        manager_inbox: &InboxSend,
    ) -> Self
    where
        A: Agent + Default + Send + Sync + 'static,
        A::State: Clone + Debug + Send + Sync + 'static,
        A::Message: Debug + Send + Sync + 'static,
        InitiatorState: From<A::State>,
    {
        let plexer_channel = plexer.subscribe_client(protocol_id);
        let inbox = manager_inbox.clone();

        let (outbox_send, outbox_recv) = tokio::sync::mpsc::channel(50);

        let _io = tokio::spawn(async move {
            let agent = A::default();
            let plexer = PlexerAdapter::new(agent, plexer_channel);
            let span = tracing::span!(tracing::Level::TRACE, "agent io", pid = %pid, protocol = %protocol_id);

            link_agent_io(pid, plexer, inbox, outbox_recv, span).await;
        });

        Self {
            _io,
            outbox: outbox_send,
        }
    }
}

pub type HsAgent = handshake::Client<handshake::n2n::VersionData>;
pub type PsAgent = peersharing::Client;
pub type KaAgent = keepalive::Client;

pub struct PeerIO {
    _plexer: RunningPlexer,
    hs_io: AgentIO<HsAgent>,
    ka_io: AgentIO<KaAgent>,
    ps_io: AgentIO<PsAgent>,
}

impl PeerIO {
    pub fn new(pid: PeerId, mut plexer: Plexer, manager_inbox: &InboxSend) -> Self {
        let hs_io = AgentIO::new(
            pid.clone(),
            &mut plexer,
            crate::miniprotocols::PROTOCOL_N2N_HANDSHAKE,
            manager_inbox,
        );

        let ka_io = AgentIO::new(
            pid.clone(),
            &mut plexer,
            crate::miniprotocols::PROTOCOL_N2N_KEEP_ALIVE,
            manager_inbox,
        );

        let ps_io = AgentIO::new(
            pid.clone(),
            &mut plexer,
            crate::miniprotocols::PROTOCOL_N2N_PEER_SHARING,
            manager_inbox,
        );

        let _plexer = plexer.spawn();

        Self {
            _plexer,
            hs_io,
            ka_io,
            ps_io,
        }
    }
}

#[instrument(skip_all)]
async fn connect_peer(pid: PeerId, inbox: &InboxSend) -> Result<PeerIO, Error> {
    let value = format!("{}:{}", pid.host, pid.port);

    let address = tokio::net::lookup_host(value)
        .await?
        .next()
        .ok_or(Error::AddressResolutionFailed(pid.clone()))?;

    info!("resolved address: {}", address);

    let bearer =
        crate::multiplexer::Bearer::connect_tcp_timeout(address, Duration::from_secs(10)).await?;

    info!("tcp bearer connected");

    let plexer = crate::multiplexer::Plexer::new(bearer);

    let peer_io = PeerIO::new(pid, plexer, inbox);

    info!("peer IO spawned");

    Ok(peer_io)
}

#[derive(Clone)]
pub struct MegaPlexer {
    peers: papaya::HashMap<PeerId, Arc<PeerIO>>,
    inbox: InboxSend,
}

impl MegaPlexer {
    pub fn new(inbox: InboxSend) -> Self {
        Self {
            peers: papaya::HashMap::new(),
            inbox,
        }
    }

    pub async fn connect_peer(&self, pid: &PeerId) -> Result<(), Error> {
        let peers = self.peers.pin();

        if peers.contains_key(pid) {
            info!(pid = %pid, "peer already connected");
            return Ok(());
        }

        info!(pid = %pid, "connecting peer");
        let peer_io = connect_peer(pid.clone(), &self.inbox).await?;
        peers.insert(pid.clone(), Arc::new(peer_io));

        Ok(())
    }

    pub async fn disconnect_peer(&self, pid: &PeerId) -> Result<(), Error> {
        let peers = self.peers.pin();

        let _peer_io = peers.remove(pid).ok_or(Error::PeerNotFound)?;

        todo!("abort peer io");
        //peer_io.plexer.abort().await;
    }

    pub async fn send_message(&self, pid: &PeerId, msg: AnyMessage) -> Result<(), Error> {
        let peers = self.peers.pin();

        let peer_io = peers.get(pid).ok_or(Error::PeerNotFound)?;

        match msg {
            AnyMessage::Handshake(msg) => {
                peer_io.hs_io.outbox.send(msg).await.unwrap();
            }
            AnyMessage::PeerSharing(msg) => {
                peer_io.ps_io.outbox.send(msg).await.unwrap();
            }
            AnyMessage::KeepAlive(msg) => {
                peer_io.ka_io.outbox.send(msg).await.unwrap();
            }
            _ => todo!(),
        }

        Ok(())
    }
}
