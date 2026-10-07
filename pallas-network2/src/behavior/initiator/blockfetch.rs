use std::collections::{HashMap, VecDeque};

use crate::protocol::blockfetch as blockfetch_proto;

use crate::{
    BehaviorOutput, InterfaceCommand, OutboundQueue, PeerId,
    behavior::{AnyMessage, BlockRange, ConnectionState},
};

use super::{
    InitiatorBehavior, InitiatorEvent, InitiatorState, PeerVisitor, connection::needs_disconnect,
};

/// Configuration for the block-fetch sub-behavior (currently unused).
pub type BlockFetchConfig = ();

/// A block-fetch request, defined as a range of points.
pub type Request = BlockRange;

/// Sub-behavior that manages block fetching from peers.
pub struct BlockFetchBehavior {
    //config: BlockFetchConfig,
    requests: VecDeque<Request>,
    sent: HashMap<PeerId, Request>,
}

impl Default for BlockFetchBehavior {
    fn default() -> Self {
        Self::new(())
    }
}

impl BlockFetchBehavior {
    /// Creates a new block-fetch behavior with the given configuration.
    pub fn new(_config: BlockFetchConfig) -> Self {
        Self {
            requests: VecDeque::new(),
            sent: HashMap::new(),
        }
    }

    /// Adds a block range request to the pending queue.
    pub fn enqueue(&mut self, request: Request) {
        self.requests.push_back(request);
        tracing::info!(total = self.requests.len(), "new request");
    }

    /// Sends a block range request to the specified peer.
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

    /// Sends the first queued range to `pid` when that peer is initialized,
    /// idle, not due for a disconnect, and holds no unsent range request.
    pub(super) fn serve_next(
        &mut self,
        pid: &PeerId,
        state: &mut InitiatorState,
        outbound: &mut OutboundQueue<super::InitiatorBehavior>,
    ) {
        if !peer_is_available(state) {
            return;
        }

        if let Some(request) = self.requests.front()
            && state.try_send_request(
                pid,
                AnyMessage::BlockFetch(blockfetch_proto::Message::RequestRange(request.clone())),
                outbound,
            )
        {
            tracing::debug!("granting request to peer");
            self.requests.pop_front();
        }
    }

    /// Emits a [`BlockBodyReceived`](super::InitiatorEvent::BlockBodyReceived)
    /// event if the peer's block-fetch state contains a new block.
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
        && !needs_disconnect(state)
}

impl PeerVisitor for BlockFetchBehavior {
    fn visit_inbound_msg(
        &mut self,
        pid: &PeerId,
        state: &mut InitiatorState,
        outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) {
        self.dispatch_block(pid, state, outbound);

        if matches!(state.blockfetch, blockfetch_proto::State::Idle) {
            self.sent.remove(pid);
        }

        self.serve_next(pid, state, outbound);
    }

    fn visit_outbound_msg(
        &mut self,
        pid: &PeerId,
        state: &mut InitiatorState,
        _outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) {
        if let blockfetch_proto::State::Busy(range) = &state.blockfetch {
            self.sent.insert(pid.clone(), range.clone());
        }
    }

    fn visit_housekeeping(
        &mut self,
        pid: &PeerId,
        state: &mut InitiatorState,
        outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) {
        self.serve_next(pid, state, outbound);
    }

    fn visit_disconnected(
        &mut self,
        pid: &PeerId,
        state: &mut InitiatorState,
        _outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) {
        if let Some(range) = self.sent.remove(pid) {
            self.requests.push_front(range);
        }

        if let Some(range) = state.take_unsent_range() {
            self.requests.push_front(range);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::OutboundQueue;
    use crate::behavior::initiator::PromotionTag;
    use crate::protocol::{Point, blockfetch as bf};

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
            matches!(
                o,
                BehaviorOutput::ExternalEvent(InitiatorEvent::BlockBodyReceived(..))
            )
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
    fn inbound_while_busy_keeps_the_next_range_queued() {
        let mut bf = BlockFetchBehavior::new(());
        let pid = PeerId::test(1);
        let mut state = InitiatorState::new();
        let mut outbound = OutboundQueue::new();

        state.connection = ConnectionState::Initialized;
        state.promotion = PromotionTag::Warm;
        state.blockfetch = bf::State::Busy((Point::Origin, Point::Origin));
        bf.enqueue((Point::Origin, Point::Origin));

        bf.visit_inbound_msg(&pid, &mut state, &mut outbound);
        assert!(
            drain_outputs(&mut outbound).is_empty(),
            "nothing is sent while Busy"
        );
        assert_eq!(bf.requests.len(), 1, "the range stays queued");
    }

    #[test]
    fn housekeeping_dispatches_request_for_available_peer() {
        let mut bf = BlockFetchBehavior::new(());
        let pid = PeerId::test(1);
        let mut state = InitiatorState::new();
        let mut outbound = OutboundQueue::new();

        let range = (Point::Origin, Point::new(100, vec![0xAA; 32]));
        bf.enqueue(range);

        // Peer must be Initialized, Warm or Hot, and blockfetch Idle
        state.connection = ConnectionState::Initialized;
        state.promotion = PromotionTag::Warm;
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
        assert!(
            bf.requests.is_empty(),
            "request should be consumed from queue"
        );
    }
}
