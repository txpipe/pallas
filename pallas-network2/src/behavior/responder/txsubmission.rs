use crate::{
    InterfaceCommand, OutboundQueue, PeerId, behavior::AnyMessage,
    protocol::txsubmission as txsubmission_proto,
};

use super::{ResponderBehavior, ResponderPeerVisitor, ResponderState};

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
        _pid: &PeerId,
        state: &ResponderState,
        _outbound: &mut OutboundQueue<ResponderBehavior>,
    ) {
        if let txsubmission_proto::State::Txs = &state.tx_submission {
            // The state machine stays in Txs after ReplyTxs, so we can't easily
            // detect new replies. For now, this is a placeholder for when the
            // protocol state machine supports extracting received tx data.
            tracing::trace!("tx submission in Txs state");
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
