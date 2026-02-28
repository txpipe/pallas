use std::{collections::HashMap, future::Future, pin::Pin, task::Poll, time::Duration};

use futures::{
    Stream, StreamExt,
    stream::{FusedStream, FuturesUnordered},
};
use rand::Rng;

use crate::{
    Interface, InterfaceCommand, InterfaceEvent, PeerId,
    behavior::AnyMessage,
    protocol::{
        self as proto, MAINNET_MAGIC, blockfetch, chainsync, handshake, keepalive, peersharing,
    },
};

type MockFuture = Pin<Box<dyn Future<Output = InterfaceEvent<AnyMessage>> + Send>>;

/// Tracks how far each simulated initiator peer has progressed.
#[derive(Default)]
struct PeerProgress {
    headers_sent: u32,
}

/// A mock interface that simulates inbound initiator peers connecting to our
/// responder node. Each peer follows a scripted protocol flow: handshake →
/// chainsync → blockfetch → keepalive → disconnect.
pub struct MockInitiatorInterface {
    pending: FuturesUnordered<MockFuture>,
    progress: HashMap<PeerId, PeerProgress>,
    max_headers_per_peer: u32,
}

impl MockInitiatorInterface {
    pub fn new(num_peers: u16, max_headers_per_peer: u32) -> Self {
        let pending = FuturesUnordered::new();

        // Schedule each peer to connect with some jitter
        for i in 0..num_peers {
            let pid = PeerId {
                host: format!("10.0.0.{}", i + 1),
                port: 3000 + i,
            };

            let jitter = Duration::from_millis(rand::rng().random_range(100..1500));
            let future: MockFuture = Box::pin(async move {
                tokio::time::sleep(jitter).await;
                tracing::info!(%pid, "simulated initiator connecting");
                InterfaceEvent::Connected(pid)
            });

            pending.push(future);
        }

        Self {
            pending,
            progress: HashMap::new(),
            max_headers_per_peer,
        }
    }

    fn jitter(&self) -> Duration {
        Duration::from_millis(rand::rng().random_range(50..500))
    }

    fn push_jittered_msg(&mut self, pid: PeerId, msg: AnyMessage, jitter: Duration) {
        let future: MockFuture = Box::pin(async move {
            tokio::time::sleep(jitter).await;
            InterfaceEvent::Recv(pid, vec![msg])
        });
        self.pending.push(future);
    }

    fn push_jittered_disconnect(&mut self, pid: PeerId, jitter: Duration) {
        let future: MockFuture = Box::pin(async move {
            tokio::time::sleep(jitter).await;
            tracing::info!(%pid, "simulated initiator disconnecting");
            InterfaceEvent::Disconnected(pid)
        });
        self.pending.push(future);
    }

    /// Given a message we sent to the peer, determine what the simulated
    /// initiator peer does next and queue it.
    fn on_sent(&mut self, pid: PeerId, msg: AnyMessage) {
        let jitter = self.jitter();

        match msg {
            // After responder accepts handshake → initiator sends FindIntersect
            AnyMessage::Handshake(handshake::Message::Accept(..)) => {
                tracing::debug!(%pid, "peer received handshake accept, will send FindIntersect");
                let find = chainsync::Message::FindIntersect(vec![proto::Point::Origin]);
                self.push_jittered_msg(pid, AnyMessage::ChainSync(find), jitter);
            }

            // After responder provides intersection → initiator sends RequestNext
            AnyMessage::ChainSync(chainsync::Message::IntersectFound(..)) => {
                tracing::debug!(%pid, "peer received intersection, will send RequestNext");
                let req = chainsync::Message::RequestNext;
                self.push_jittered_msg(pid, AnyMessage::ChainSync(req), jitter);
            }

            // After responder provides a header → initiator sends RequestNext or switches to blockfetch
            AnyMessage::ChainSync(chainsync::Message::RollForward(..)) => {
                let progress = self.progress.entry(pid.clone()).or_default();
                progress.headers_sent += 1;

                if progress.headers_sent >= self.max_headers_per_peer {
                    tracing::debug!(%pid, headers = progress.headers_sent, "peer got enough headers, requesting blocks");

                    let range = (
                        proto::Point::Specific(1, vec![0xAA; 32]),
                        proto::Point::Specific(progress.headers_sent as u64, vec![0xBB; 32]),
                    );
                    let req = blockfetch::Message::RequestRange(range);
                    self.push_jittered_msg(pid, AnyMessage::BlockFetch(req), jitter);
                } else {
                    tracing::debug!(%pid, headers = progress.headers_sent, "peer requesting next header");
                    let req = chainsync::Message::RequestNext;
                    self.push_jittered_msg(pid, AnyMessage::ChainSync(req), jitter);
                }
            }

            // After responder sends BatchDone → initiator sends KeepAlive
            AnyMessage::BlockFetch(blockfetch::Message::BatchDone) => {
                tracing::debug!(%pid, "peer received all blocks, sending keepalive");
                let cookie = rand::rng().random::<u16>();
                let msg = keepalive::Message::KeepAlive(cookie);
                self.push_jittered_msg(pid, AnyMessage::KeepAlive(msg), jitter);
            }

            // After responder sends keepalive response → initiator requests peers then disconnects
            AnyMessage::KeepAlive(keepalive::Message::ResponseKeepAlive(..)) => {
                tracing::debug!(%pid, "peer received keepalive response, requesting peers");
                let msg = peersharing::Message::ShareRequest(3);
                self.push_jittered_msg(pid, AnyMessage::PeerSharing(msg), jitter);
            }

            // After responder shares peers → initiator disconnects
            AnyMessage::PeerSharing(peersharing::Message::SharePeers(..)) => {
                tracing::debug!(%pid, "peer received peers, disconnecting");
                self.push_jittered_disconnect(pid, jitter);
            }

            // Ignore other messages (StartBatch, Block, etc.)
            _ => {}
        }
    }

    /// When a peer connects, they immediately send a handshake proposal.
    fn on_connected(&mut self, pid: &PeerId) {
        let jitter = self.jitter();

        let version_data = handshake::n2n::VersionData {
            network_magic: MAINNET_MAGIC,
            initiator_only_diffusion_mode: false,
            peer_sharing: Some(1),
            query: Some(false),
        };

        let mut values = std::collections::HashMap::new();
        values.insert(13, version_data);

        let propose = handshake::Message::Propose(handshake::VersionTable { values });

        tracing::debug!(%pid, "simulated initiator sending handshake propose");
        self.push_jittered_msg(pid.clone(), AnyMessage::Handshake(propose), jitter);
    }
}

impl Interface<AnyMessage> for MockInitiatorInterface {
    fn dispatch(&mut self, cmd: InterfaceCommand<AnyMessage>) {
        match cmd {
            InterfaceCommand::Connect(pid) => {
                // Responder doesn't initiate connections, but handle it gracefully
                let jitter = self.jitter();
                let future: MockFuture = Box::pin(async move {
                    tokio::time::sleep(jitter).await;
                    InterfaceEvent::Connected(pid)
                });
                self.pending.push(future);
            }
            InterfaceCommand::Send(pid, msg) => {
                // Emit Sent event immediately
                let pid2 = pid.clone();
                let msg2 = msg.clone();
                let future: MockFuture = Box::pin(async move { InterfaceEvent::Sent(pid2, msg2) });
                self.pending.push(future);

                // Then queue the peer's response
                self.on_sent(pid, msg);
            }
            InterfaceCommand::Disconnect(pid) => {
                let jitter = self.jitter();
                let future: MockFuture = Box::pin(async move {
                    tokio::time::sleep(jitter).await;
                    InterfaceEvent::Disconnected(pid)
                });
                self.pending.push(future);
            }
        }
    }
}

impl Stream for MockInitiatorInterface {
    type Item = InterfaceEvent<AnyMessage>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let event = self.pending.poll_next_unpin(cx);

        if let Poll::Ready(Some(InterfaceEvent::Connected(pid))) = &event {
            self.on_connected(pid);
        }

        match event {
            Poll::Ready(Some(event)) => Poll::Ready(Some(event)),
            Poll::Ready(None) => Poll::Pending,
            Poll::Pending => Poll::Pending,
        }
    }
}

impl FusedStream for MockInitiatorInterface {
    fn is_terminated(&self) -> bool {
        false
    }
}
