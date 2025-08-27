use crate::behavior::PromotionTag;
use crate::protocol::{Point, chainsync as chainsync_proto};

use crate::{
    BehaviorOutput, InterfaceCommand, OutboundQueue, PeerId,
    behavior::{
        AnyMessage, BlockRange, ConnectionState, InitiatorBehavior, InitiatorEvent, InitiatorState,
        PeerVisitor,
    },
};

pub type Config = ();

type Intersection = Vec<Point>;

pub struct ChainSyncBehavior {
    //config: Config,
    intersection: Option<Intersection>,
}

impl Default for ChainSyncBehavior {
    fn default() -> Self {
        Self::new(())
    }
}

impl ChainSyncBehavior {
    pub fn new(_config: Config) -> Self {
        Self { intersection: None }
    }

    pub fn start(&mut self, intersection: Intersection) {
        self.intersection = Some(intersection);
    }

    pub fn request_intersection(
        &self,
        pid: &PeerId,
        intersection: &Intersection,
        outbound: &mut OutboundQueue<super::InitiatorBehavior>,
    ) {
        tracing::debug!("requesting intersection");

        let msg = chainsync_proto::Message::FindIntersect(intersection.clone());

        outbound.push_ready(BehaviorOutput::InterfaceCommand(InterfaceCommand::Send(
            pid.clone(),
            AnyMessage::ChainSync(msg),
        )));
    }

    pub fn request_next(
        &self,
        pid: &PeerId,
        state: &mut InitiatorState,
        outbound: &mut OutboundQueue<super::InitiatorBehavior>,
    ) {
        tracing::debug!("requesting next header");

        let out = chainsync_proto::Message::RequestNext;
        outbound.push_ready(BehaviorOutput::InterfaceCommand(InterfaceCommand::Send(
            pid.clone(),
            AnyMessage::ChainSync(out),
        )));
    }

    pub fn drain_data(
        &self,
        pid: &PeerId,
        state: &mut InitiatorState,
        outbound: &mut OutboundQueue<super::InitiatorBehavior>,
    ) {
        let Some(data) = state.chainsync.drain() else {
            return;
        };

        match data {
            chainsync_proto::Data::Content(h, tip) => {
                let out = InitiatorEvent::BlockHeaderReceived(pid.clone(), h.clone(), tip.clone());
                outbound.push_ready(BehaviorOutput::ExternalEvent(out));
            }
            chainsync_proto::Data::Rollback(point, tip) => {
                let out = InitiatorEvent::RollbackReceived(pid.clone(), point.clone(), tip.clone());
                outbound.push_ready(BehaviorOutput::ExternalEvent(out));
            }
            chainsync_proto::Data::Intersection(point, tip) => {
                let out =
                    InitiatorEvent::IntersectionFound(pid.clone(), point.clone(), tip.clone());
                outbound.push_ready(BehaviorOutput::ExternalEvent(out));
            }
            chainsync_proto::Data::NoIntersection(..) => {
                tracing::error!("no intersection found");
                state.violation = true;
            }
            _ => (),
        }
    }
}

fn peer_is_syncing(state: &InitiatorState) -> bool {
    !state.chainsync.is_new()
}

fn peer_should_sync(state: &InitiatorState) -> bool {
    matches!(state.connection, ConnectionState::Initialized) && state.promotion == PromotionTag::Hot
}

impl PeerVisitor for ChainSyncBehavior {
    fn visit_inbound_msg(
        &mut self,
        pid: &PeerId,
        state: &mut InitiatorState,
        outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) {
        if !peer_is_syncing(state) {
            tracing::trace!("peer is not syncing, skipping drain data");
            return;
        }

        self.drain_data(pid, state, outbound);
    }

    fn visit_tagged(
        &mut self,
        pid: &PeerId,
        state: &mut InitiatorState,
        outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) {
        if !peer_is_syncing(state) {
            tracing::trace!("peer is not syncing, skipping tagged");
            return;
        }

        if !state.chainsync.is_idle() {
            tracing::trace!("chainsync is not idle, skipping request next");
            return;
        }

        if state.continue_sync {
            tracing::debug!("peer wants to continue sync");
            self.request_next(pid, state, outbound);
        }
    }

    fn visit_housekeeping(
        &mut self,
        pid: &PeerId,
        state: &mut InitiatorState,
        outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) {
        let Some(intersection) = &self.intersection else {
            tracing::trace!("no intersection requested");
            return;
        };

        if !peer_should_sync(state) {
            tracing::trace!("peer is not suitable for sync");
            return;
        }

        if peer_is_syncing(state) {
            tracing::trace!("peer is already syncing");
            return;
        }

        tracing::trace!("peer needs to sync");

        self.request_intersection(pid, intersection, outbound);
    }
}
