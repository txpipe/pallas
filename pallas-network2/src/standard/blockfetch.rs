use pallas_network::miniprotocols::{Agent as _, blockfetch as blockfetch_proto};

use crate::{
    BehaviorOutput, InterfaceCommand, OutboundQueue, PeerId,
    standard::{AnyMessage, BlockRange, InitiatorEvent, InitiatorState},
};

pub type Config = ();

pub struct BlockFetchBehavior {
    //config: Config,
}

impl Default for BlockFetchBehavior {
    fn default() -> Self {
        Self::new(())
    }
}

impl BlockFetchBehavior {
    pub fn new(_config: Config) -> Self {
        Self {}
    }

    pub fn request_block_batch(
        &self,
        pid: &PeerId,
        range: BlockRange,
        outbound: &mut OutboundQueue<super::InitiatorBehavior>,
    ) {
        outbound.push_ready(BehaviorOutput::InterfaceCommand(InterfaceCommand::Send(
            pid.clone(),
            AnyMessage::BlockFetch(blockfetch_proto::Message::RequestRange(range)),
        )));
    }

    pub fn visit_updated_peer(
        &self,
        pid: &PeerId,
        state: &InitiatorState,
        outbound: &mut OutboundQueue<super::InitiatorBehavior>,
    ) {
        if let blockfetch_proto::ClientState::Streaming(Some(block)) = state.blockfetch.state() {
            let out = BehaviorOutput::ExternalEvent(InitiatorEvent::BlockBodyReceived(
                pid.clone(),
                block.clone(),
            ));

            outbound.push_ready(out);
        }
    }
}
