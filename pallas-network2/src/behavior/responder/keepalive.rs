use crate::{
    InterfaceCommand, OutboundQueue, PeerId,
    behavior::AnyMessage,
    protocol::keepalive as keepalive_proto,
};

use super::{ResponderBehavior, ResponderPeerVisitor, ResponderState};

#[derive(Default)]
pub struct KeepaliveResponder;

impl KeepaliveResponder {
    fn try_respond(
        &self,
        pid: &PeerId,
        state: &ResponderState,
        outbound: &mut OutboundQueue<ResponderBehavior>,
    ) {
        if !state.is_initialized() {
            return;
        }

        if let keepalive_proto::State::Server(cookie) = &state.keepalive {
            tracing::debug!("responding to keepalive");

            let msg = keepalive_proto::Message::ResponseKeepAlive(*cookie);
            outbound.push_ready(InterfaceCommand::Send(
                pid.clone(),
                AnyMessage::KeepAlive(msg),
            ));
        }
    }
}

impl ResponderPeerVisitor for KeepaliveResponder {
    fn visit_inbound_msg(
        &mut self,
        pid: &PeerId,
        state: &mut ResponderState,
        outbound: &mut OutboundQueue<ResponderBehavior>,
    ) {
        self.try_respond(pid, state, outbound);
    }
}
