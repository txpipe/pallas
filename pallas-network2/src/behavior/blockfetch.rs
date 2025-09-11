use std::collections::VecDeque;

use crate::protocol::blockfetch as blockfetch_proto;

use crate::{
    BehaviorOutput, InterfaceCommand, OutboundQueue, PeerId,
    behavior::{
        AnyMessage, BlockRange, ConnectionState, InitiatorBehavior, InitiatorEvent, InitiatorState,
        PeerVisitor,
    },
};

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
