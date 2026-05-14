//! Test-only extensions for domain types.
//!
//! This module is compiled only under `#[cfg(test)]` and provides convenience
//! methods on [`PeerId`], [`OutboundQueue`], and [`BehaviorOutput`] slices to
//! reduce boilerplate in unit and integration tests.

use futures::StreamExt;

use crate::{Behavior, BehaviorOutput, InterfaceCommand, OutboundQueue, PeerId};

impl PeerId {
    /// Creates a PeerId with a deterministic address for testing.
    pub fn test(id: u8) -> Self {
        PeerId {
            host: format!("10.0.0.{}", id),
            port: 3000 + id as u16,
        }
    }
}

impl<B: Behavior> OutboundQueue<B> {
    /// Synchronously drains all immediately-ready outputs.
    pub fn drain_ready(&mut self) -> Vec<BehaviorOutput<B>> {
        let mut outputs = Vec::new();
        let waker = futures::task::noop_waker();
        let mut cx = std::task::Context::from_waker(&waker);

        while let std::task::Poll::Ready(Some(output)) = self.futures.poll_next_unpin(&mut cx) {
            outputs.push(output);
        }

        outputs
    }
}

/// Extension trait for inspecting slices of [`BehaviorOutput`] in tests.
pub(crate) trait BehaviorOutputExt<B: Behavior> {
    /// Returns true if any output is a Connect command for the given peer.
    fn has_connect_for(&self, pid: &PeerId) -> bool;
    /// Returns true if any output is a Disconnect command for the given peer.
    fn has_disconnect_for(&self, pid: &PeerId) -> bool;
    /// Returns true if any output is a Send command whose message matches the predicate.
    fn has_send<F>(&self, pred: F) -> bool
    where
        F: Fn(&B::Message) -> bool;
    /// Returns true if any output is an external event matching the predicate.
    fn has_event<F>(&self, pred: F) -> bool
    where
        F: Fn(&B::Event) -> bool;
}

impl<B: Behavior> BehaviorOutputExt<B> for [BehaviorOutput<B>] {
    fn has_connect_for(&self, pid: &PeerId) -> bool {
        self.iter().any(|o| {
            matches!(o, BehaviorOutput::InterfaceCommand(InterfaceCommand::Connect(p)) if p == pid)
        })
    }

    fn has_disconnect_for(&self, pid: &PeerId) -> bool {
        self.iter().any(|o| {
            matches!(o, BehaviorOutput::InterfaceCommand(InterfaceCommand::Disconnect(p)) if p == pid)
        })
    }

    fn has_send<F>(&self, pred: F) -> bool
    where
        F: Fn(&B::Message) -> bool,
    {
        self.iter().any(|o| match o {
            BehaviorOutput::InterfaceCommand(InterfaceCommand::Send(_, msg)) => pred(msg),
            _ => false,
        })
    }

    fn has_event<F>(&self, pred: F) -> bool
    where
        F: Fn(&B::Event) -> bool,
    {
        self.iter().any(|o| match o {
            BehaviorOutput::ExternalEvent(e) => pred(e),
            _ => false,
        })
    }
}
