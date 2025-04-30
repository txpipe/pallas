use futures::{future, StreamExt};
use itertools::Itertools;
use tracing::warn;

use crate::miniprotocols::{
    handshake,
    peersharing::{self, IdleState},
};

use super::*;

pub struct ChainSyncBehavior {
    intersect: Vec<crate::miniprotocols::Point>,
}

#[derive(Debug)]
pub struct PeerPromotionConfig {
    pub desired_hot_peers: (usize, usize),
    pub desired_warm_peers: (usize, usize),
    pub desired_cold_peers: (usize, usize),
    pub trusted_peers: Vec<PeerId>,
}

#[derive(Debug)]
pub struct PeerPromotionBehavior {
    config: PeerPromotionConfig,
    span: tracing::Span,
    backoff: Option<Duration>,
}

impl PeerPromotionBehavior {
    pub fn new(config: PeerPromotionConfig) -> Self {
        Self {
            config,
            span: tracing::info_span!("peer_promotion"),
            backoff: None,
        }
    }

    fn missing_trusted(&self, state: &State) -> Vec<IntrinsicCommand> {
        let peers = state.peers.pin();

        let missing = self
            .config
            .trusted_peers
            .iter()
            .filter(|pid| !peers.contains_key(*pid))
            .collect::<Vec<_>>();

        missing
            .into_iter()
            .map(|pid| {
                IntrinsicCommand::TrackNewPeer(
                    pid.clone(),
                    vec![PeerTag::Cold.into(), PeerTag::Trusted.into()],
                )
            })
            .collect()
    }

    fn warm_promotions(&self, state: &State) -> Vec<IntrinsicCommand> {
        let desired = self.config.desired_hot_peers.0;

        let current = state
            .peers
            .pin()
            .iter()
            .filter(|(_, p)| p.has_tag(PeerTag::Hot))
            .count();

        if current >= desired {
            return vec![];
        }

        info!(desired, current, "hot peers below low water mark");

        let required = desired - current;

        let peers = state.peers.pin();

        let warm = peers
            .iter()
            .filter(|(_, p)| p.has_tag(PeerTag::Warm))
            .filter(|(_, p)| p.is_connected());

        let candidates: Vec<_> = warm
            .take(required)
            .map(|(pid, _)| {
                IntrinsicCommand::SwitchPeerTag(
                    pid.clone(),
                    PeerTag::Warm.into(),
                    PeerTag::Hot.into(),
                )
            })
            .collect();

        info!(candidate = candidates.len(), "found warm peer candidates");

        candidates
    }

    fn cold_promotions(&self, state: &State) -> Vec<IntrinsicCommand> {
        let desired = self.config.desired_warm_peers.0;

        let current = state
            .peers
            .pin()
            .iter()
            .filter(|(_, p)| p.has_tag(PeerTag::Warm))
            .count();

        if current >= desired {
            return vec![];
        }

        info!(desired, current, "warm peers below low water mark");

        let required = desired - current;

        let peers = state.peers.pin();

        let cold = peers.iter().filter(|(_, p)| p.has_tag(PeerTag::Cold));

        let candidates: Vec<_> = cold
            .take(required)
            .map(|(pid, _)| {
                IntrinsicCommand::SwitchPeerTag(
                    pid.clone(),
                    PeerTag::Cold.into(),
                    PeerTag::Warm.into(),
                )
            })
            .collect();

        info!(candidate = candidates.len(), "found cold peer candidates");

        candidates
    }
}

impl Behavior for PeerPromotionBehavior {
    fn backoff(&self) -> Option<Duration> {
        self.backoff.clone()
    }

    fn next(&mut self, state: &State) -> impl Iterator<Item = IntrinsicCommand> {
        let _span = self.span.enter();

        let mut all = vec![];

        all.extend(self.missing_trusted(state));
        all.extend(self.warm_promotions(state));
        all.extend(self.cold_promotions(state));

        // TODO
        //let hot_demotions = self.hot_demotions(state);
        //let warm_demotions = self.warm_demotions(state);
        //let cold_demotions = self.cold_demotions(state);

        if all.is_empty() {
            self.backoff = Some(Duration::from_secs(10));
        }

        all.into_iter()
    }

    fn handle(&mut self, event: &NetworkEvent) {
        let _span = self.span.enter();

        match event {
            NetworkEvent::PeerDiscovered(_) => {
                debug!("peer discovered, clearing backoff");
                self.backoff = None;
            }
            NetworkEvent::PeerTagged(..) => {
                debug!("peer tagged, clearing backoff");
                self.backoff = None;
            }
            _ => (),
        }
    }
}

#[derive(Debug)]
pub struct ConnectPeersConfig {}

pub struct ConnectPeersBehavior {
    backoff: Option<Duration>,
    failed: HashSet<PeerId>,
    span: tracing::Span,
}

impl ConnectPeersBehavior {
    pub fn new(_: ConnectPeersConfig) -> Self {
        Self {
            backoff: None,
            failed: HashSet::new(),
            span: tracing::info_span!("connect_peers"),
        }
    }
}

impl Behavior for ConnectPeersBehavior {
    fn backoff(&self) -> Option<Duration> {
        self.backoff.clone()
    }

    fn next(&mut self, state: &State) -> impl Iterator<Item = IntrinsicCommand> {
        let _span = self.span.enter();

        let mut commands = state
            .peers
            .pin()
            .iter()
            .filter(|(_, p)| !p.is_connected())
            .filter(|(_, p)| p.has_tag(PeerTag::Warm) || p.has_tag(PeerTag::Hot))
            .filter(|(_, p)| !self.failed.contains(&p.id))
            .map(|(pid, _)| IntrinsicCommand::ConnectPeer(pid.clone()))
            .collect::<Vec<_>>();

        info!(
            disconnected = commands.len(),
            "found disconnected warm peers"
        );

        for failed in self.failed.drain() {
            commands.push(IntrinsicCommand::SwitchPeerTag(
                failed,
                PeerTag::Warm.into(),
                PeerTag::Banned.into(),
            ));
        }

        if commands.is_empty() {
            self.backoff = Some(Duration::from_secs(10));
        }

        commands.into_iter()
    }

    fn handle(&mut self, event: &NetworkEvent) {
        let _span = self.span.enter();

        match event {
            NetworkEvent::PeerTagged(_, _) => {
                self.backoff = None;
            }
            NetworkEvent::PeerConnectFailed(pid) => {
                debug!(%pid, "tracking failed peer");
                self.failed.insert(pid.clone());
                self.backoff = None;
            }
            _ => (),
        }
    }
}

#[derive(Debug, Default)]
pub struct CommandBuffer(Vec<IntrinsicCommand>);

impl CommandBuffer {
    pub fn push(&mut self, command: IntrinsicCommand) {
        self.0.push(command);
    }

    pub fn extend(&mut self, iter: impl Iterator<Item = IntrinsicCommand>) {
        self.0.extend(iter);
    }

    pub fn send_message(&mut self, pid: &PeerId, msg: AnyMessage) {
        self.0.push(IntrinsicCommand::SendMessage(pid.clone(), msg));
    }

    pub fn track_peer(&mut self, pid: &PeerId, tags: Vec<PeerTag>) {
        self.0
            .push(IntrinsicCommand::TrackNewPeer(pid.clone(), tags));
    }

    pub fn tag_peer(&mut self, pid: &PeerId, tag: PeerTag) {
        self.0
            .push(IntrinsicCommand::AddPeerTag(pid.clone(), tag.into()));
    }

    pub fn drain(&mut self) -> Vec<IntrinsicCommand> {
        self.0.drain(..).collect()
    }

    pub fn drain_if(&mut self, f: impl Fn(&IntrinsicCommand) -> bool) -> Vec<IntrinsicCommand> {
        let mut drained = vec![];
        let mut retained = vec![];

        for cmd in self.0.drain(..) {
            if f(&cmd) {
                drained.push(cmd);
            } else {
                retained.push(cmd);
            }
        }

        self.0 = retained;

        drained
    }

    pub fn backoff(&self) -> Option<Duration> {
        if self.0.is_empty() {
            Some(Duration::from_secs(2))
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct HandshakeConfig {
    pub handshake: crate::miniprotocols::handshake::n2n::VersionTable,
}

pub struct HandshakeBehavior {
    buffer: CommandBuffer,
    config: HandshakeConfig,
    span: tracing::Span,
}

impl HandshakeBehavior {
    pub fn new(config: HandshakeConfig) -> Self {
        Self {
            buffer: CommandBuffer::default(),
            config,
            span: tracing::info_span!("handshake"),
        }
    }

    fn send_message(&mut self, pid: &PeerId, msg: handshake::Message<handshake::n2n::VersionData>) {
        self.buffer.send_message(pid, AnyMessage::Handshake(msg));
    }

    fn propose(&mut self, pid: &PeerId) {
        debug!(%pid, "sending version proposal");

        let versions = self.config.handshake.clone();
        self.send_message(pid, handshake::Message::Propose(versions));
    }

    fn update(&mut self, pid: &PeerId, done: &handshake::DoneState<handshake::n2n::VersionData>) {
        match done {
            handshake::DoneState::Accepted(x, y) => {
                debug!(%pid, "peer accepted us");
                self.buffer.tag_peer(pid, PeerTag::Accepted(*x as u32));

                if y.peer_sharing == Some(1) {
                    debug!(%pid, "peer supports peer sharing");
                    self.buffer.tag_peer(pid, PeerTag::PeerSharing);
                } else {
                    warn!(%pid, "peer does not support peer sharing");
                }
            }
            handshake::DoneState::Rejected(x) => {
                warn!(%pid, "handshake rejected");
                self.buffer.tag_peer(pid, PeerTag::Rejected);
            }
            _ => (),
        }
    }

    #[instrument(skip_all, parent = self.span.clone())]
    fn handle_state_change(
        &mut self,
        pid: &PeerId,
        state: &handshake::State<handshake::n2n::VersionData>,
    ) {
        match state {
            handshake::State::Propose => self.propose(pid),
            handshake::State::Done(x) => self.update(pid, x),
            _ => (),
        }
    }
}

impl Behavior for HandshakeBehavior {
    fn backoff(&self) -> Option<Duration> {
        self.buffer.backoff()
    }

    fn next(&mut self, _state: &State) -> impl Iterator<Item = IntrinsicCommand> {
        self.buffer.drain().into_iter()
    }

    fn handle(&mut self, event: &NetworkEvent) {
        match event {
            NetworkEvent::InitiatorStateChange(pid, InitiatorState::Handshake(state)) => {
                self.handle_state_change(pid, state);
            }
            _ => (),
        }
    }
}

#[derive(Debug)]
pub struct PeerDiscoveryConfig {
    pub desired_peers: usize,
}

pub struct PeerDiscoveryBehavior {
    buffer: CommandBuffer,
    config: PeerDiscoveryConfig,
    span: tracing::Span,
}

impl PeerDiscoveryBehavior {
    pub fn new(config: PeerDiscoveryConfig) -> Self {
        Self {
            config,
            buffer: CommandBuffer::default(),
            span: tracing::info_span!("peer_sharing"),
        }
    }

    fn send_message(&mut self, pid: &PeerId, msg: peersharing::Message) {
        self.buffer.send_message(pid, AnyMessage::PeerSharing(msg));
    }

    fn request(&mut self, pid: &PeerId) {
        debug!(%pid, "requesting peers");

        self.send_message(
            pid,
            peersharing::Message::ShareRequest(self.config.desired_peers as u8),
        );
    }

    fn track(&mut self, pid: &PeerId, response: &[peersharing::PeerAddress]) {
        debug!(%pid, "tracking shared peers");

        for addr in response {
            let pid = match addr {
                peersharing::PeerAddress::V4(ip, port) => {
                    let host = format!("{}:{}", ip, port);
                    PeerId::from_str(&host).unwrap()
                }
                peersharing::PeerAddress::V6(ip, port) => {
                    let host = format!("[{}]:{}", ip, port);
                    PeerId::from_str(&host).unwrap()
                }
            };

            self.buffer.track_peer(&pid, vec![PeerTag::Cold.into()]);
        }

        self.buffer
            .tag_peer(pid, PeerTag::SharedPeers(response.len()));
    }

    fn handle_state_change(
        &mut self,
        pid: &PeerId,
        state: &crate::miniprotocols::peersharing::State,
    ) {
        use peersharing::*;

        match state {
            State::Idle(IdleState::Response(response)) => self.track(pid, response),
            _ => (),
        }
    }

    fn count_not_banned_peers(&self, state: &State) -> usize {
        let peers = state.peers.pin();

        peers
            .iter()
            .filter(|(_, p)| !p.has_tag(PeerTag::Banned))
            .count()
    }

    fn filter_sharing_peers(&self, state: &State) -> Vec<PeerId> {
        let peers = state.peers.pin();

        peers
            .iter()
            .filter(|(_, p)| p.has_tag(PeerTag::Hot) && p.has_tag(PeerTag::PeerSharing))
            .filter(|(_, p)| !p.has_tag(PeerTag::SharedPeers(0)))
            .map(|(pid, _)| pid.clone())
            .collect()
    }
}

impl Behavior for PeerDiscoveryBehavior {
    fn backoff(&self) -> Option<Duration> {
        self.buffer.backoff()
    }

    fn next(&mut self, state: &State) -> impl Iterator<Item = IntrinsicCommand> {
        let not_banned = self.count_not_banned_peers(&state);

        if not_banned < self.config.desired_peers {
            info!(
                not_banned = not_banned,
                desired = self.config.desired_peers,
                "need more peers, requesting"
            );

            let to_ask = self.filter_sharing_peers(&state);

            info!(to_ask = to_ask.len(), "found peers with peer sharing");

            for pid in to_ask {
                self.request(&pid);
            }
        }

        self.buffer.drain().into_iter()
    }

    fn handle(&mut self, event: &NetworkEvent) {
        match event {
            NetworkEvent::InitiatorStateChange(pid, InitiatorState::PeerSharing(state)) => {
                self.handle_state_change(pid, state);
            }
            _ => (),
        }
    }
}

#[derive(Debug)]
pub struct KeepAliveConfig {
    pub interval: Duration,
}

pub struct KeepAliveBehavior {
    buffer: CommandBuffer,
    config: KeepAliveConfig,
    span: tracing::Span,
}

impl KeepAliveBehavior {
    pub fn new(config: KeepAliveConfig) -> Self {
        Self {
            config,
            buffer: CommandBuffer::default(),
            span: tracing::info_span!("keep_alive"),
        }
    }

    fn send_message(&mut self, pid: &PeerId, msg: keepalive::Message) {
        self.buffer.send_message(pid, AnyMessage::KeepAlive(msg));
    }

    fn request(&mut self, pid: &PeerId) {
        debug!(%pid, "requesting peers");

        self.send_message(pid, keepalive::Message::KeepAlive(111));
    }

    fn update(&mut self, pid: &PeerId) {
        debug!(%pid, "updating peer seen alive");

        self.buffer
            .tag_peer(pid, PeerTag::SeenAlive(Instant::now()));
    }

    fn handle_state_change(
        &mut self,
        pid: &PeerId,
        state: &crate::miniprotocols::keepalive::State,
    ) {
        use keepalive::*;

        match state {
            State::Client(ClientState::Response(response)) => self.update(pid),
            _ => (),
        }
    }

    fn handle_accepted(&mut self, pid: &PeerId) {
        debug!(%pid, "peer accepted, requesting keep-alive");

        self.request(pid);
    }
}

impl Behavior for KeepAliveBehavior {
    fn backoff(&self) -> Option<Duration> {
        self.buffer.backoff()
    }

    fn next(&mut self, state: &State) -> impl Iterator<Item = IntrinsicCommand> {
        let _span = self.span.clone();
        let _span = _span.enter();

        let peers = state.peers.pin();

        let max_interval = self.config.interval;
        let aging_peers: Vec<_> = peers
            .iter()
            .filter(|(_, p)| p.is_connected())
            .filter(|(_, p)| p.has_tag(PeerTag::Warm) || p.has_tag(PeerTag::Hot))
            .filter(|(_, p)| {
                p.tags.iter().any(|t| match t {
                    PeerTag::SeenAlive(instant) => instant.elapsed() >= max_interval,
                    _ => false,
                })
            })
            .collect();

        for (pid, _) in aging_peers {
            self.request(pid);
        }

        self.buffer.drain().into_iter()
    }

    fn handle(&mut self, event: &NetworkEvent) {
        match event {
            NetworkEvent::PeerTagged(pid, PeerTag::Accepted(_)) => {
                self.handle_accepted(pid);
            }
            NetworkEvent::InitiatorStateChange(pid, InitiatorState::KeepAlive(state)) => {
                self.handle_state_change(pid, state);
            }
            _ => (),
        }
    }
}

pub struct InterleaveBehavior<A: Behavior, B: Behavior> {
    a: A,
    b: B,
}

impl<A: Behavior, B: Behavior> InterleaveBehavior<A, B> {
    pub fn new(a: A, b: B) -> Self {
        Self { a, b }
    }
}

impl<A: Behavior, B: Behavior> Behavior for InterleaveBehavior<A, B> {
    fn backoff(&self) -> Option<Duration> {
        match (self.a.backoff(), self.b.backoff()) {
            (Some(a), Some(b)) => Some(a.min(b)),
            _ => None,
        }
    }

    fn next(&mut self, state: &State) -> impl Iterator<Item = IntrinsicCommand> {
        let a = self.a.next(state);
        let b = self.b.next(state);
        a.interleave(b)
    }

    fn handle(&mut self, event: &NetworkEvent) {
        self.a.handle(event);
        self.b.handle(event);
    }
}
