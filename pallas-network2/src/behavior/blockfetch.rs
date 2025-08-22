use std::collections::VecDeque;

use pallas_network::miniprotocols::{Agent as _, blockfetch as blockfetch_proto};

use crate::{
    BehaviorOutput, InterfaceCommand, OutboundQueue, PeerId,
    behavior::{
        AnyMessage, BlockRange, ConnectionState, InitiatorBehavior, InitiatorEvent, InitiatorState,
        PeerVisitor,
    },
};

pub type Config = ();

pub type Request = BlockRange;

pub struct BlockFetchBehavior {
    //config: Config,
    requests: VecDeque<Request>,
}

impl Default for BlockFetchBehavior {
    fn default() -> Self {
        Self::new(())
    }
}

impl BlockFetchBehavior {
    pub fn new(_config: Config) -> Self {
        Self {
            requests: VecDeque::new(),
        }
    }

    pub fn enqueue(&mut self, request: Request) {
        self.requests.push_back(request);
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
        if let blockfetch_proto::ClientState::Streaming(Some(block)) = state.blockfetch.state() {
            let out = InitiatorEvent::BlockBodyReceived(pid.clone(), block.clone());

            outbound.push_ready(BehaviorOutput::ExternalEvent(out));
        }
    }
}

fn peer_is_available(state: &InitiatorState) -> bool {
    matches!(state.connection, ConnectionState::Initialized)
        && matches!(
            state.blockfetch.state(),
            blockfetch_proto::ClientState::Idle
        )
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
        if peer_is_available(state) {
            tracing::debug!("found available peer");
            if let Some(request) = self.requests.pop_front() {
                self.request_block_batch(pid, request, outbound);
            }
        }
    }
}
