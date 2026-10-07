use std::collections::HashMap;

use crate::{Channel, InterfaceCommand, Message as _, OutboundQueue, PeerId, protocol as proto};

use super::{
    AnyMessage, BlockRange, InitiatorBehavior, InitiatorState,
    responder::{ResponderBehavior, ResponderState},
};

/// One peer's pushed requests whose Sent the protocol state has not applied,
/// at most one per channel.
#[derive(Debug, Default)]
pub(crate) struct UnsentRequests {
    by_channel: HashMap<Channel, AnyMessage>,
}

impl UnsentRequests {
    fn contains(&self, msg: &AnyMessage) -> bool {
        self.by_channel
            .get(&msg.channel())
            .is_some_and(|unsent| unsent.payload() == msg.payload())
    }

    fn insert(&mut self, msg: &AnyMessage) -> bool {
        if !is_request(msg) || self.by_channel.contains_key(&msg.channel()) {
            return false;
        }

        self.by_channel.insert(msg.channel(), msg.clone());
        true
    }
}

fn is_request(msg: &AnyMessage) -> bool {
    use proto::{
        blockfetch, chainsync, keepalive, leiosfetch, leiosnotify, peersharing, txsubmission,
    };

    matches!(
        msg,
        AnyMessage::KeepAlive(keepalive::Message::KeepAlive(_))
            | AnyMessage::PeerSharing(peersharing::Message::ShareRequest(_))
            | AnyMessage::BlockFetch(blockfetch::Message::RequestRange(_))
            | AnyMessage::ChainSync(
                chainsync::Message::RequestNext | chainsync::Message::FindIntersect(_)
            )
            | AnyMessage::LeiosNotify(leiosnotify::Message::RequestNext)
            | AnyMessage::LeiosFetch(
                leiosfetch::Message::BlockRequest(_) | leiosfetch::Message::BlockTxsRequest(..)
            )
            | AnyMessage::TxSubmission(txsubmission::Message::RequestTxIds(..))
    )
}

impl InitiatorState {
    fn can_apply(&self, msg: &AnyMessage) -> bool {
        match msg {
            AnyMessage::KeepAlive(m) => self.keepalive.apply(m).is_ok(),
            AnyMessage::PeerSharing(m) => self.peersharing.apply(m).is_ok(),
            AnyMessage::BlockFetch(m) => self.blockfetch.apply(m).is_ok(),
            AnyMessage::ChainSync(m) => self.chainsync.apply(m).is_ok(),
            AnyMessage::LeiosNotify(m) => self.leios_notify.apply(m).is_ok(),
            AnyMessage::LeiosFetch(m) => self.leios_fetch.apply(m).is_ok(),
            AnyMessage::Handshake(_) | AnyMessage::TxSubmission(_) => false,
        }
    }

    /// Pushes `msg` when it is a request, its channel has no unsent request,
    /// and the protocol state can apply it.
    pub(crate) fn try_send_request(
        &mut self,
        pid: &PeerId,
        msg: AnyMessage,
        outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) -> bool {
        if !self.can_apply(&msg) || !self.unsent.insert(&msg) {
            return false;
        }

        outbound.push_ready(InterfaceCommand::Send(pid.clone(), msg));
        true
    }

    /// Marks a chain sync `RequestNext` as deferred.
    pub(crate) fn defer_sync_next(&mut self) {
        self.sync_next_deferred = true;
    }

    /// Pushes the deferred `RequestNext`, if one is deferred, through
    /// [`Self::try_send_request`] and clears `sync_next_deferred` when it is
    /// pushed.
    pub(crate) fn try_send_deferred_next(
        &mut self,
        pid: &PeerId,
        outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) -> bool {
        let next = AnyMessage::ChainSync(proto::chainsync::Message::RequestNext);

        if !self.sync_next_deferred || !self.try_send_request(pid, next, outbound) {
            return false;
        }

        self.sync_next_deferred = false;
        true
    }

    /// Removes and returns this peer's unsent block fetch range.
    pub(crate) fn take_unsent_range(&mut self) -> Option<BlockRange> {
        match self
            .unsent
            .by_channel
            .remove(&proto::blockfetch::CHANNEL_ID)
        {
            Some(AnyMessage::BlockFetch(proto::blockfetch::Message::RequestRange(range))) => {
                Some(range)
            }
            _ => None,
        }
    }

    /// Drops the unsent copy of `msg` when the protocol state before `msg`
    /// can apply it.
    pub(crate) fn clear_unsent(&mut self, msg: &AnyMessage) {
        if self.unsent.contains(msg) && self.can_apply(msg) {
            self.unsent.by_channel.remove(&msg.channel());
        }
    }
}

impl ResponderState {
    fn can_apply(&self, msg: &AnyMessage) -> bool {
        match msg {
            AnyMessage::TxSubmission(m) => self.tx_submission.apply(m).is_ok(),
            _ => false,
        }
    }

    /// Pushes `msg` when it is a request, its channel has no unsent request,
    /// and the protocol state can apply it.
    pub(crate) fn try_send_request(
        &mut self,
        pid: &PeerId,
        msg: AnyMessage,
        outbound: &mut OutboundQueue<ResponderBehavior>,
    ) -> bool {
        if !self.can_apply(&msg) || !self.unsent.insert(&msg) {
            return false;
        }

        outbound.push_ready(InterfaceCommand::Send(pid.clone(), msg));
        true
    }

    /// Drops the unsent copy of `msg` when the protocol state before `msg`
    /// can apply it.
    pub(crate) fn clear_unsent(&mut self, msg: &AnyMessage) {
        if self.unsent.contains(msg) && self.can_apply(msg) {
            self.unsent.by_channel.remove(&msg.channel());
        }
    }
}
