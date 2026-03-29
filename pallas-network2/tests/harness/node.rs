use std::net::SocketAddr;
use std::time::Duration;

use tokio::task::JoinHandle;

use pallas_network2::behavior::responder::{ResponderBehavior, ResponderCommand, ResponderEvent};
use pallas_network2::behavior::{AnyMessage, InitiatorBehavior, InitiatorCommand, InitiatorEvent};
use pallas_network2::interface::{TcpInterface, TcpListenerInterface};
use pallas_network2::protocol::Point;
use pallas_network2::{Manager, PeerId};

use super::MockChain;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
const HOUSEKEEPING_INTERVAL: Duration = Duration::from_millis(50);

/// A test responder node that listens for inbound connections and responds to
/// protocol events using a [`MockChain`].
pub struct ResponderNode {
    listener: tokio::net::TcpListener,
    addr: SocketAddr,
}

impl ResponderNode {
    /// Binds to a random port on localhost.
    pub async fn bind() -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("failed to bind");
        let addr = listener.local_addr().unwrap();

        Self { listener, addr }
    }

    /// Returns the address this node is listening on.
    #[allow(dead_code)]
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Spawns the node's event loop as a background task.
    ///
    /// The responder automatically handles protocol events using synthetic
    /// chain data from [`MockChain`]. Returns the listening address and a
    /// join handle (abort it to stop the node).
    pub fn spawn(self) -> (SocketAddr, JoinHandle<Vec<ResponderEvent>>) {
        let addr = self.addr;

        let handle = tokio::spawn(async move {
            let interface = TcpListenerInterface::new(self.listener);
            let behavior = ResponderBehavior::default();
            let mut manager = Manager::new(interface, behavior);
            let mut chain = MockChain::new();
            let mut events = Vec::new();

            loop {
                let event = manager.poll_next().await;

                let Some(event) = event else {
                    continue;
                };

                match &event {
                    ResponderEvent::PeerInitialized(pid, _) => {
                        tracing::info!(%pid, "responder: peer initialized");
                        manager.execute(ResponderCommand::Housekeeping);
                    }
                    ResponderEvent::PeerDisconnected(pid) => {
                        tracing::info!(%pid, "responder: peer disconnected");
                    }
                    ResponderEvent::IntersectionRequested(pid, _) => {
                        manager.execute(ResponderCommand::ProvideIntersection(
                            pid.clone(),
                            Point::Origin,
                            chain.tip(),
                        ));
                    }
                    ResponderEvent::NextHeaderRequested(pid) => {
                        let (header, tip) = chain.next_header();
                        manager.execute(ResponderCommand::ProvideHeader(pid.clone(), header, tip));
                    }
                    ResponderEvent::BlockRangeRequested(pid, _) => {
                        manager.execute(ResponderCommand::ProvideBlocks(
                            pid.clone(),
                            chain.blocks(2),
                        ));
                    }
                    ResponderEvent::PeersRequested(pid, _) => {
                        manager.execute(ResponderCommand::ProvidePeers(pid.clone(), vec![]));
                    }
                    ResponderEvent::TxReceived(pid, _) => {
                        tracing::info!(%pid, "responder: tx received");
                    }
                }

                events.push(event);
            }
        });

        (addr, handle)
    }
}

/// A test initiator node that connects to one or more responders.
pub struct InitiatorNode {
    manager: Manager<TcpInterface<AnyMessage>, InitiatorBehavior, AnyMessage>,
    peer_id: PeerId,
}

impl InitiatorNode {
    /// Creates a new initiator that connects to the given address.
    ///
    /// The peer is included and an initial housekeeping pass is triggered
    /// to start the connection + promotion flow.
    pub fn connect_to(addr: SocketAddr) -> Self {
        let interface = TcpInterface::new();
        let behavior = InitiatorBehavior::default();
        let mut manager = Manager::new(interface, behavior);

        let peer_id = PeerId {
            host: addr.ip().to_string(),
            port: addr.port(),
        };

        manager.execute(InitiatorCommand::IncludePeer(peer_id.clone()));
        manager.execute(InitiatorCommand::Housekeeping);

        Self { manager, peer_id }
    }

    /// Returns the PeerId of the responder this node is connected to.
    pub fn peer_id(&self) -> PeerId {
        self.peer_id.clone()
    }

    /// Executes a command on the underlying manager.
    pub fn execute(&mut self, cmd: InitiatorCommand) {
        self.manager.execute(cmd);
    }

    /// Runs the poll loop until the predicate is satisfied, with automatic
    /// housekeeping. Panics on timeout with collected events for debugging.
    pub async fn run_until<F>(&mut self, timeout: Duration, mut done: F) -> Vec<InitiatorEvent>
    where
        F: FnMut(&[InitiatorEvent]) -> bool,
    {
        let mut events = Vec::new();
        let mut housekeeping_interval = tokio::time::interval(HOUSEKEEPING_INTERVAL);

        let result = tokio::time::timeout(timeout, async {
            loop {
                tokio::select! {
                    event = self.manager.poll_next() => {
                        if let Some(event) = event {
                            events.push(event);
                            if done(&events) {
                                return;
                            }
                        }
                    }
                    _ = housekeeping_interval.tick() => {
                        self.manager.execute(InitiatorCommand::Housekeeping);
                    }
                }
            }
        })
        .await;

        if result.is_err() {
            panic!(
                "test timed out after {:?}. Events collected: {:?}",
                timeout, events
            );
        }

        events
    }

    /// Waits for the handshake to complete (PeerInitialized event).
    pub async fn wait_for_handshake(&mut self) -> Vec<InitiatorEvent> {
        self.run_until(DEFAULT_TIMEOUT, |events| {
            events
                .iter()
                .any(|e| matches!(e, InitiatorEvent::PeerInitialized(..)))
        })
        .await
    }

    /// Waits for an intersection to be found (IntersectionFound event).
    pub async fn wait_for_intersection(&mut self) -> Vec<InitiatorEvent> {
        self.run_until(DEFAULT_TIMEOUT, |events| {
            events
                .iter()
                .any(|e| matches!(e, InitiatorEvent::IntersectionFound(..)))
        })
        .await
    }

    /// Waits for a block header to be received (BlockHeaderReceived event).
    pub async fn wait_for_header(&mut self) -> Vec<InitiatorEvent> {
        self.run_until(DEFAULT_TIMEOUT, |events| {
            events
                .iter()
                .any(|e| matches!(e, InitiatorEvent::BlockHeaderReceived(..)))
        })
        .await
    }

    /// Keeps the node running for the given duration, polling events and
    /// executing periodic housekeeping. Returns all events collected.
    /// Does NOT panic on timeout — the duration is the desired run time.
    pub async fn sustain(&mut self, duration: Duration) -> Vec<InitiatorEvent> {
        let mut events = Vec::new();
        let mut housekeeping_interval = tokio::time::interval(HOUSEKEEPING_INTERVAL);

        let _ = tokio::time::timeout(duration, async {
            loop {
                tokio::select! {
                    event = self.manager.poll_next() => {
                        if let Some(event) = event {
                            events.push(event);
                        }
                    }
                    _ = housekeeping_interval.tick() => {
                        self.manager.execute(InitiatorCommand::Housekeeping);
                    }
                }
            }
        })
        .await;

        events
    }

    /// Waits for a block body to be received (BlockBodyReceived event).
    pub async fn wait_for_block(&mut self) -> Vec<InitiatorEvent> {
        self.run_until(DEFAULT_TIMEOUT, |events| {
            events
                .iter()
                .any(|e| matches!(e, InitiatorEvent::BlockBodyReceived(..)))
        })
        .await
    }
}
