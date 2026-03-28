//! Artifacts to emulate a network interface without any actual IO

use futures::{
    Stream, StreamExt as _,
    stream::{FusedStream, FuturesUnordered},
};
use rand::Rng as _;
use std::{pin::Pin, task::Poll, time::Duration};

use crate::{Interface, InterfaceCommand, InterfaceEvent, Message, PeerId};

pub mod happy;
pub mod initiator_mock;

/// Rules that define how the emulator responds to network commands.
///
/// Implement this trait to define the simulated behavior of a remote peer,
/// including what messages to reply with and how much jitter to introduce.
pub trait Rules {
    /// The message type used by the emulated protocol.
    type Message: Message + Clone + 'static;

    /// Enqueue reply messages for a given inbound message from a peer.
    fn reply_to(
        &self,
        pid: PeerId,
        msg: Self::Message,
        jitter: Duration,
        queue: &mut ReplyQueue<Self::Message>,
    );

    /// Whether the emulator should accept a connection from the given peer.
    fn should_connect(&self, _pid: PeerId) -> bool {
        true
    }

    /// Returns the random jitter duration to apply before delivering a reply.
    fn jitter(&self) -> Duration {
        Duration::from_secs(rand::rng().random_range(0..3))
    }
}

type ReplyFuture<M> = Pin<Box<dyn Future<Output = InterfaceEvent<M>> + Send>>;

/// A queue of delayed replies to be delivered by the emulator.
pub struct ReplyQueue<M>(FuturesUnordered<ReplyFuture<M>>)
where
    M: Message;

impl<M> ReplyQueue<M>
where
    M: Message,
{
    fn new() -> Self {
        Self(FuturesUnordered::new())
    }

    /// Pushes a raw reply future into the queue.
    pub fn push(&mut self, future: ReplyFuture<M>) {
        self.0.push(future);
    }

    /// Enqueues a message reply that will be delivered after the given jitter delay.
    pub fn push_jittered_msg(&mut self, peer_id: PeerId, msg: M, jitter: Duration) {
        let future = Box::pin(async move {
            tokio::time::sleep(jitter).await;
            tracing::debug!(%peer_id, "emulating recv from");
            InterfaceEvent::Recv(peer_id, vec![msg])
        });

        self.push(future);
    }

    /// Enqueues a disconnect event that will be delivered after the given jitter delay.
    pub fn push_jittered_disconnect(&mut self, peer_id: PeerId, jitter: Duration) {
        let future = Box::pin(async move {
            tokio::time::sleep(jitter).await;
            tracing::warn!(%peer_id, "emulating disconnect");
            InterfaceEvent::Disconnected(peer_id)
        });

        self.push(future);
    }
}

impl<M> Stream for ReplyQueue<M>
where
    M: Message,
{
    type Item = InterfaceEvent<M>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.0.poll_next_unpin(cx)
    }
}

impl<M> FusedStream for ReplyQueue<M>
where
    M: Message,
{
    fn is_terminated(&self) -> bool {
        false
    }
}

/// A fake network interface that simulates peer interactions without real IO.
///
/// The emulator implements [`Interface`] by immediately responding to commands
/// according to the provided [`Rules`], with configurable jitter to simulate
/// network latency.
pub struct Emulator<M, R>
where
    M: Message + Clone + Send + Sync + 'static,
    R: Rules<Message = M>,
{
    rules: R,
    pending: ReplyQueue<M>,
}

impl<M, R> Default for Emulator<M, R>
where
    M: Message + Clone + Send + Sync + 'static,
    R: Rules<Message = M> + Default,
{
    fn default() -> Self {
        Self {
            rules: R::default(),
            pending: ReplyQueue::new(),
        }
    }
}

impl<M, R> Interface<M> for Emulator<M, R>
where
    M: Message + Clone + Send + Sync + 'static,
    R: Rules<Message = M> + Unpin,
{
    fn dispatch(&mut self, cmd: InterfaceCommand<M>) {
        match cmd {
            InterfaceCommand::Connect(peer_id) => {
                let jitter = self.rules.jitter();

                let future = Box::pin(async move {
                    tokio::time::sleep(jitter).await;
                    tracing::info!(%peer_id, "emulating connected");
                    InterfaceEvent::Connected(peer_id)
                });

                self.pending.push(future);
            }
            InterfaceCommand::Disconnect(peer_id) => {
                let jitter = self.rules.jitter();

                let future = Box::pin(async move {
                    tokio::time::sleep(jitter).await;
                    tracing::info!(%peer_id, "emulating disconnected");
                    InterfaceEvent::Disconnected(peer_id)
                });

                self.pending.push(future);
            }
            InterfaceCommand::Send(peer_id, msg) => {
                let pid2 = peer_id.clone();
                let msg2 = msg.clone();
                let future1 = Box::pin(async move { InterfaceEvent::Sent(pid2, msg2) });

                self.pending.push(future1);

                let jitter = self.rules.jitter();
                self.rules.reply_to(peer_id, msg, jitter, &mut self.pending);
            }
        };
    }
}

impl<M, R> FusedStream for Emulator<M, R>
where
    M: Message + Clone + Send + Sync + 'static,
    R: Rules<Message = M> + Unpin,
{
    fn is_terminated(&self) -> bool {
        false
    }
}

impl<M, R> Stream for Emulator<M, R>
where
    M: Message + Clone + Send + Sync + 'static,
    R: Rules<Message = M> + Unpin,
{
    type Item = InterfaceEvent<M>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let event = self.get_mut().pending.poll_next_unpin(cx);

        match event {
            Poll::Ready(Some(event)) => Poll::Ready(Some(event)),
            Poll::Ready(None) => Poll::Pending,
            Poll::Pending => Poll::Pending,
        }
    }
}
