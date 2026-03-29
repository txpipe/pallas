use std::collections::VecDeque;

use crate::protocol::blockfetch as blockfetch_proto;

use crate::{
    BehaviorOutput, InterfaceCommand, OutboundQueue, PeerId,
    behavior::{AnyMessage, BlockRange, ConnectionState},
};

use super::{InitiatorBehavior, InitiatorEvent, InitiatorState, PeerVisitor};

pub type BlockFetchConfig = ();

pub type Request = BlockRange;

pub struct BlockFetchBehavior {
    //config: BlockFetchConfig,
    requests: VecDeque<Request>,
}

impl Default for BlockFetchBehavior {
    fn default() -> Self {
        Self::new(())
    }
}

impl BlockFetchBehavior {
    pub fn new(_config: BlockFetchConfig) -> Self {
        Self {
            requests: VecDeque::new(),
        }
    }

    pub fn enqueue(&mut self, request: Request) {
        self.requests.push_back(request);
        tracing::info!(total = self.requests.len(), "new request");
    }

    pub fn request_block_batch(
        &self,
        pid: &PeerId,
        range: BlockRange,
        outbound: &mut OutboundQueue<super::InitiatorBehavior>,
    ) {
        tracing::info!("requesting block batch");

        outbound.push_ready(BehaviorOutput::InterfaceCommand(InterfaceCommand::Send(
            pid.clone(),
            AnyMessage::BlockFetch(blockfetch_proto::Message::RequestRange(range)),
        )));
    }

    pub fn dispatch_block(
        &self,
        pid: &PeerId,
        state: &InitiatorState,
        outbound: &mut OutboundQueue<super::InitiatorBehavior>,
    ) {
        if let blockfetch_proto::State::Streaming(Some(block)) = &state.blockfetch {
            let out = InitiatorEvent::BlockBodyReceived(pid.clone(), block.clone());

            outbound.push_ready(BehaviorOutput::ExternalEvent(out));
        }
    }
}

fn peer_is_available(state: &InitiatorState) -> bool {
    matches!(state.connection, ConnectionState::Initialized)
        && matches!(state.blockfetch, blockfetch_proto::State::Idle)
}

impl PeerVisitor for BlockFetchBehavior {
    fn visit_inbound_msg(
        &mut self,
        pid: &PeerId,
        state: &mut InitiatorState,
        outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) {
        self.dispatch_block(pid, state, outbound);
    }

    fn visit_housekeeping(
        &mut self,
        pid: &PeerId,
        state: &mut InitiatorState,
        outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) {
        if self.requests.is_empty() {
            tracing::trace!("no requests pending");
            return;
        }

        if peer_is_available(state) {
            tracing::debug!("peer looks available");

            if let Some(request) = self.requests.pop_front() {
                tracing::debug!("granting request to peer");
                self.request_block_batch(pid, request, outbound);
            }
        } else {
            tracing::warn!("no peer available");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{Point, blockfetch as bf};
    use crate::OutboundQueue;

    fn drain_outputs(
        outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) -> Vec<BehaviorOutput<InitiatorBehavior>> {
        outbound.drain_ready()
    }

    #[test]
    fn enqueue_adds_to_queue() {
        let mut bf = BlockFetchBehavior::new(());
        let range = (Point::Origin, Point::new(100, vec![0xAA; 32]));

        bf.enqueue(range);
        assert_eq!(bf.requests.len(), 1);

        bf.enqueue((Point::Origin, Point::Origin));
        assert_eq!(bf.requests.len(), 2);
    }

    #[test]
    fn dispatch_block_emits_event_when_streaming() {
        let bf = BlockFetchBehavior::new(());
        let pid = PeerId::test(1);
        let mut state = InitiatorState::new();
        let mut outbound = OutboundQueue::new();

        state.blockfetch = bf::State::Streaming(Some(vec![0xBE; 64]));

        bf.dispatch_block(&pid, &state, &mut outbound);

        let outputs = drain_outputs(&mut outbound);
        let has_event = outputs.iter().any(|o| {
            matches!(o, BehaviorOutput::ExternalEvent(InitiatorEvent::BlockBodyReceived(..)))
        });
        assert!(has_event, "should emit BlockBodyReceived");
    }

    #[test]
    fn dispatch_block_noop_when_idle() {
        let bf = BlockFetchBehavior::new(());
        let pid = PeerId::test(1);
        let state = InitiatorState::new();
        let mut outbound = OutboundQueue::new();

        // Default state is Idle
        bf.dispatch_block(&pid, &state, &mut outbound);

        let outputs = drain_outputs(&mut outbound);
        assert!(outputs.is_empty());
    }

    #[test]
    fn housekeeping_dispatches_request_for_available_peer() {
        let mut bf = BlockFetchBehavior::new(());
        let pid = PeerId::test(1);
        let mut state = InitiatorState::new();
        let mut outbound = OutboundQueue::new();

        let range = (Point::Origin, Point::new(100, vec![0xAA; 32]));
        bf.enqueue(range);

        // Peer must be Initialized + blockfetch Idle
        state.connection = ConnectionState::Initialized;
        state.blockfetch = bf::State::Idle;

        bf.visit_housekeeping(&pid, &mut state, &mut outbound);

        let outputs = drain_outputs(&mut outbound);
        let has_request = outputs.iter().any(|o| {
            matches!(
                o,
                BehaviorOutput::InterfaceCommand(InterfaceCommand::Send(
                    _,
                    AnyMessage::BlockFetch(bf::Message::RequestRange(_))
                ))
            )
        });
        assert!(has_request, "should send RequestRange");
        assert!(bf.requests.is_empty(), "request should be consumed from queue");
    }
}
