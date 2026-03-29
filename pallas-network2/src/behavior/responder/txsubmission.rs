use crate::{
    BehaviorOutput, InterfaceCommand, OutboundQueue, PeerId, behavior::AnyMessage,
    protocol::txsubmission as txsubmission_proto,
};

use super::{ResponderBehavior, ResponderEvent, ResponderPeerVisitor, ResponderState};

pub struct TxSubmissionResponderConfig {
    pub max_tx_request: u16,
}

impl Default for TxSubmissionResponderConfig {
    fn default() -> Self {
        Self { max_tx_request: 10 }
    }
}

pub struct TxSubmissionResponder {
    config: TxSubmissionResponderConfig,
}

impl Default for TxSubmissionResponder {
    fn default() -> Self {
        Self::new(TxSubmissionResponderConfig::default())
    }
}

impl TxSubmissionResponder {
    pub fn new(config: TxSubmissionResponderConfig) -> Self {
        Self { config }
    }

    fn try_init(
        &self,
        pid: &PeerId,
        state: &ResponderState,
        outbound: &mut OutboundQueue<ResponderBehavior>,
    ) {
        if !state.is_initialized() {
            return;
        }

        if !matches!(state.tx_submission, txsubmission_proto::State::Init) {
            return;
        }

        tracing::debug!("initializing tx submission");
        let msg = txsubmission_proto::Message::Init;
        outbound.push_ready(InterfaceCommand::Send(
            pid.clone(),
            AnyMessage::TxSubmission(msg),
        ));
    }

    fn try_request_tx_ids(
        &self,
        pid: &PeerId,
        state: &ResponderState,
        outbound: &mut OutboundQueue<ResponderBehavior>,
    ) {
        if !state.is_initialized() {
            return;
        }

        if !matches!(state.tx_submission, txsubmission_proto::State::Idle) {
            return;
        }

        tracing::debug!("requesting tx ids");
        let msg = txsubmission_proto::Message::RequestTxIds(true, 0, self.config.max_tx_request);
        outbound.push_ready(InterfaceCommand::Send(
            pid.clone(),
            AnyMessage::TxSubmission(msg),
        ));
    }

    fn try_extract_txs(
        &self,
        pid: &PeerId,
        state: &ResponderState,
        outbound: &mut OutboundQueue<ResponderBehavior>,
    ) {
        if let txsubmission_proto::State::Txs(txs) = &state.tx_submission {
            for tx in txs {
                outbound.push_ready(BehaviorOutput::ExternalEvent(ResponderEvent::TxReceived(
                    pid.clone(),
                    tx.clone(),
                )));
            }
        }
    }
}

impl ResponderPeerVisitor for TxSubmissionResponder {
    fn visit_housekeeping(
        &mut self,
        pid: &PeerId,
        state: &mut ResponderState,
        outbound: &mut OutboundQueue<ResponderBehavior>,
    ) {
        self.try_init(pid, state, outbound);
        self.try_request_tx_ids(pid, state, outbound);
    }

    fn visit_inbound_msg(
        &mut self,
        pid: &PeerId,
        state: &mut ResponderState,
        outbound: &mut OutboundQueue<ResponderBehavior>,
    ) {
        self.try_extract_txs(pid, state, outbound);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::OutboundQueue;
    use crate::behavior::ConnectionState;
    use crate::protocol::MAINNET_MAGIC;
    use crate::protocol::handshake;
    use crate::protocol::txsubmission::{EraTxBody, State as TxState};

    fn drain_outputs(
        outbound: &mut OutboundQueue<ResponderBehavior>,
    ) -> Vec<BehaviorOutput<ResponderBehavior>> {
        outbound.drain_ready()
    }

    fn make_initialized_state() -> ResponderState {
        let mut s = ResponderState::new();
        s.connection = ConnectionState::Initialized;
        let vd = handshake::n2n::VersionData::new(MAINNET_MAGIC, false, Some(1), Some(false));
        s.handshake = handshake::State::Done(handshake::DoneState::Accepted(13, vd));
        s
    }

    #[test]
    fn init_sent_when_initialized_and_init_state() {
        let mut txsub = TxSubmissionResponder::new(TxSubmissionResponderConfig::default());
        let pid = PeerId::test(1);
        let mut state = make_initialized_state();
        let mut outbound = OutboundQueue::new();

        state.tx_submission = TxState::Init;

        txsub.visit_housekeeping(&pid, &mut state, &mut outbound);

        let outputs = drain_outputs(&mut outbound);
        let has_init = outputs.iter().any(|o| {
            matches!(
                o,
                BehaviorOutput::InterfaceCommand(InterfaceCommand::Send(
                    _,
                    AnyMessage::TxSubmission(txsubmission_proto::Message::Init)
                ))
            )
        });
        assert!(has_init, "should send Init message");
    }

    #[test]
    fn request_tx_ids_sent_when_idle() {
        let mut txsub =
            TxSubmissionResponder::new(TxSubmissionResponderConfig { max_tx_request: 5 });
        let pid = PeerId::test(1);
        let mut state = make_initialized_state();
        let mut outbound = OutboundQueue::new();

        state.tx_submission = TxState::Idle;

        txsub.visit_housekeeping(&pid, &mut state, &mut outbound);

        let outputs = drain_outputs(&mut outbound);
        let req = outputs.iter().find_map(|o| match o {
            BehaviorOutput::InterfaceCommand(InterfaceCommand::Send(
                _,
                AnyMessage::TxSubmission(txsubmission_proto::Message::RequestTxIds(
                    blocking,
                    _,
                    count,
                )),
            )) => Some((*blocking, *count)),
            _ => None,
        });

        assert_eq!(
            req,
            Some((true, 5)),
            "should send RequestTxIds with max_tx_request=5"
        );
    }

    #[test]
    fn txs_extracted_and_emitted() {
        let mut txsub = TxSubmissionResponder::new(TxSubmissionResponderConfig::default());
        let pid = PeerId::test(1);
        let mut state = make_initialized_state();
        let mut outbound = OutboundQueue::new();

        state.tx_submission = TxState::Txs(vec![
            EraTxBody(1, vec![0xAA; 32]),
            EraTxBody(1, vec![0xBB; 64]),
        ]);

        txsub.visit_inbound_msg(&pid, &mut state, &mut outbound);

        let outputs = drain_outputs(&mut outbound);
        let tx_events: Vec<_> = outputs
            .iter()
            .filter(|o| {
                matches!(
                    o,
                    BehaviorOutput::ExternalEvent(ResponderEvent::TxReceived(..))
                )
            })
            .collect();

        assert_eq!(tx_events.len(), 2, "should emit TxReceived for each tx");
    }

    #[test]
    fn init_not_sent_when_not_initialized() {
        let mut txsub = TxSubmissionResponder::new(TxSubmissionResponderConfig::default());
        let pid = PeerId::test(1);
        let mut state = ResponderState::new(); // NOT initialized
        let mut outbound = OutboundQueue::new();

        state.tx_submission = TxState::Init;

        txsub.visit_housekeeping(&pid, &mut state, &mut outbound);

        let outputs = drain_outputs(&mut outbound);
        assert!(
            outputs.is_empty(),
            "should not send Init when not initialized"
        );
    }

    #[test]
    fn max_tx_request_used_in_request() {
        let mut txsub =
            TxSubmissionResponder::new(TxSubmissionResponderConfig { max_tx_request: 42 });
        let pid = PeerId::test(1);
        let mut state = make_initialized_state();
        let mut outbound = OutboundQueue::new();

        state.tx_submission = TxState::Idle;

        txsub.visit_housekeeping(&pid, &mut state, &mut outbound);

        let outputs = drain_outputs(&mut outbound);
        let count = outputs.iter().find_map(|o| match o {
            BehaviorOutput::InterfaceCommand(InterfaceCommand::Send(
                _,
                AnyMessage::TxSubmission(txsubmission_proto::Message::RequestTxIds(_, _, c)),
            )) => Some(*c),
            _ => None,
        });

        assert_eq!(count, Some(42), "should use configured max_tx_request");
    }
}
