use crate::{BehaviorOutput, OutboundQueue, PeerId, protocol::leiosfetch as fetch_proto};

use super::{ResponderBehavior, ResponderEvent, ResponderPeerVisitor, ResponderState};

/// Responder sub-behavior that surfaces leios-fetch requests from peers as
/// events, so the application can answer with the corresponding `Provide*`
/// responder commands.
#[derive(Default)]
pub struct LeiosFetchResponder;

impl ResponderPeerVisitor for LeiosFetchResponder {
    fn visit_inbound_msg(
        &mut self,
        pid: &PeerId,
        state: &mut ResponderState,
        outbound: &mut OutboundQueue<ResponderBehavior>,
    ) {
        if !state.is_initialized() {
            return;
        }

        let event = match &state.leios_fetch {
            fetch_proto::State::AwaitingBlock(point) => {
                Some(ResponderEvent::EbRequested(pid.clone(), point.clone()))
            }
            fetch_proto::State::AwaitingBlockTxs(point, bitmaps) => Some(
                ResponderEvent::EbTxsRequested(pid.clone(), point.clone(), bitmaps.clone()),
            ),
            // Idle/Done carry no inbound request to surface.
            _ => None,
        };

        if let Some(event) = event {
            tracing::debug!("leios fetch requested");
            outbound.push_ready(BehaviorOutput::ExternalEvent(event));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behavior::ConnectionState;
    use crate::protocol::Point;
    use std::collections::BTreeMap;

    fn events(state: &mut ResponderState) -> Vec<ResponderEvent> {
        let mut outbound = OutboundQueue::new();
        LeiosFetchResponder.visit_inbound_msg(&PeerId::test(1), state, &mut outbound);
        outbound
            .drain_ready()
            .into_iter()
            .map(|x| match x {
                BehaviorOutput::ExternalEvent(e) => e,
                _ => panic!("an output other than an event"),
            })
            .collect()
    }

    fn initialized(fetch: fetch_proto::State) -> ResponderState {
        let mut state = ResponderState::new();
        state.connection = ConnectionState::Initialized;
        state.leios_fetch = fetch;
        state
    }

    fn point() -> Point {
        Point::Specific(7, vec![0xCD; 32])
    }

    #[test]
    fn a_block_request_is_surfaced_once_initialized() {
        let mut state = initialized(fetch_proto::State::AwaitingBlock(point()));
        let got = events(&mut state);
        assert!(
            matches!(&got[..], [ResponderEvent::EbRequested(p, x)] if *p == PeerId::test(1) && *x == point()),
            "{got:?}"
        );

        state.connection = ConnectionState::Connected;
        assert!(events(&mut state).is_empty());
    }

    #[test]
    fn a_block_txs_request_is_surfaced_with_its_bitmaps() {
        let bitmaps = fetch_proto::Bitmaps(BTreeMap::from([(2u16, 1u64 << 63)]));
        let mut state = initialized(fetch_proto::State::AwaitingBlockTxs(
            point(),
            bitmaps.clone(),
        ));
        let got = events(&mut state);
        assert!(
            matches!(&got[..], [ResponderEvent::EbTxsRequested(p, x, b)] if *p == PeerId::test(1) && *x == point() && *b == bitmaps),
            "{got:?}"
        );

        let mut idle = initialized(fetch_proto::State::Idle(None));
        assert!(events(&mut idle).is_empty());
    }
}
