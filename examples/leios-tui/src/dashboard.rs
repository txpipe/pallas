//! Observable state of the initiator node, plus the pure mapping from
//! [`InitiatorEvent`]s to state mutations and outbound [`Action`]s.
//!
//! [`Dashboard::apply_event`] is deliberately free of any network handle so it
//! can be driven by a synthetic event sequence in tests (see the bottom of this
//! file) without touching a socket.

use std::collections::{HashSet, VecDeque};
use std::time::Instant;

use indexmap::IndexMap;
use pallas_codec::minicbor::{Decoder, data::Type};
use pallas_network2::{
    PeerId,
    behavior::initiator::InitiatorEvent,
    protocol::{
        EbId, Point,
        chainsync::HeaderContent,
        handshake::n2n::LEIOS_MIN_VERSION,
        leiosfetch::{self, Bitmaps},
        leiosnotify,
    },
};

use crate::logbuf::SharedLog;

/// Transactions requested per leios-fetch call: one 64-tx bitmap window. We page
/// across windows (see the `BlockTxs` handler) to pull a whole EB while keeping
/// each request inside the relay's per-response limit.
const MAX_TXS_PER_FETCH: usize = 64;

/// Cap on how many EB rows are retained (newest kept, oldest dropped).
const MAX_EBS: usize = 100;

/// A network command the loop should issue on the node's behalf. Returned by
/// [`Dashboard::apply_event`] so the dashboard itself stays network-free.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Resume chain-sync for the peer.
    ContinueSync(PeerId),
    /// Fetch a complete EB body.
    FetchEb(PeerId, EbId),
    /// Fetch a subset of an EB's transactions.
    FetchEbTxs(PeerId, EbId, Bitmaps),
}

/// The lifecycle stage of an EB as observed from the initiator. Monotonic:
/// transitions only ever advance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EbStage {
    /// The peer offered the body (`BlockOffer`).
    Offered,
    /// We fetched the body and learned the tx count.
    BodyFetched,
    /// The peer offered the transactions (`BlockTxsOffer`).
    TxsOffered,
    /// We fetched (some of) the transactions.
    TxsFetched,
}

/// One row of the EB table.
#[derive(Debug, Clone)]
pub struct EbRow {
    pub slot: u64,
    pub hash: Vec<u8>,
    pub size: Option<u32>,
    pub tx_total: Option<usize>,
    pub tx_fetched: usize,
    pub votes: usize,
    pub voters: HashSet<u64>,
    pub stage: EbStage,
}

impl EbRow {
    fn new(slot: u64, hash: Vec<u8>) -> Self {
        Self {
            slot,
            hash,
            size: None,
            tx_total: None,
            tx_fetched: 0,
            votes: 0,
            voters: HashSet::new(),
            stage: EbStage::Offered,
        }
    }

    /// Advances the stage, never regressing.
    fn advance(&mut self, stage: EbStage) {
        if stage > self.stage {
            self.stage = stage;
        }
    }
}

/// Aggregate counters for the Leios overlay funnel.
#[derive(Debug, Default)]
pub struct OverlayCounters {
    pub announced: u64,
    pub offered: u64,
    pub bodies: u64,
    pub txs_offered: u64,
    pub txs_ebs: u64,
    pub tx_count: u64,
    pub votes: u64,
    pub voters: HashSet<u64>,
    pub bytes: u64,
}

/// Praos chain-sync view.
#[derive(Debug, Default)]
pub struct ChainView {
    pub tip_height: Option<u64>,
    pub tip_slot: Option<u64>,
    pub local_height: Option<u64>,
    pub local_slot: Option<u64>,
    pub era: Option<u8>,
    pub headers: u64,
    pub rollbacks: u64,
    /// Arrival instants of recent headers, for the rate readout / sparkline.
    pub hdr_times: VecDeque<Instant>,
}

/// The negotiated peer, once the handshake completes.
#[derive(Debug, Clone)]
pub struct PeerView {
    pub addr: String,
    pub version: u64,
    pub leios: bool,
}

/// Everything the UI renders.
pub struct Dashboard {
    pub started: Instant,
    pub relay: String,
    pub magic: u64,
    pub peer: Option<PeerView>,
    pub chain: ChainView,
    pub overlay: OverlayCounters,
    pub ebs: IndexMap<EbId, EbRow>,
    pub log: SharedLog,
    pub selected: usize,
    pub follow: bool,
}

impl Dashboard {
    pub fn new(relay: String, magic: u64, log: SharedLog) -> Self {
        Self {
            started: Instant::now(),
            relay,
            magic,
            peer: None,
            chain: ChainView::default(),
            overlay: OverlayCounters::default(),
            ebs: IndexMap::new(),
            log,
            selected: 0,
            follow: true,
        }
    }

    /// Applies an initiator event to the dashboard, returning any network actions
    /// the caller should execute. Pure with respect to the network.
    pub fn apply_event(&mut self, event: &InitiatorEvent) -> Vec<Action> {
        let mut actions = Vec::new();

        match event {
            InitiatorEvent::PeerInitialized(pid, (version, _data)) => {
                let leios = *version >= LEIOS_MIN_VERSION;
                self.peer = Some(PeerView {
                    addr: pid.to_string(),
                    version: *version,
                    leios,
                });
                tracing::info!(version = *version, leios, "peer initialized");
            }

            // --- Praos chain-sync ---
            InitiatorEvent::IntersectionFound(pid, point, tip) => {
                self.set_tip(tip);
                self.chain.local_slot = Some(point.slot_or_default());
                actions.push(Action::ContinueSync(pid.clone()));
                tracing::info!(
                    slot = point.slot_or_default(),
                    tip = tip.1,
                    "intersection found"
                );
            }
            InitiatorEvent::BlockHeaderReceived(pid, header, tip) => {
                self.set_tip(tip);
                self.chain.era = Some(header.variant);
                if let Some((height, slot)) = header_pos(header) {
                    self.chain.local_height = Some(height);
                    self.chain.local_slot = Some(slot);
                }
                self.chain.headers += 1;
                self.chain.hdr_times.push_back(Instant::now());
                while self.chain.hdr_times.len() > 256 {
                    self.chain.hdr_times.pop_front();
                }
                actions.push(Action::ContinueSync(pid.clone()));
            }
            InitiatorEvent::RollbackReceived(pid, point, tip) => {
                self.set_tip(tip);
                self.chain.local_slot = Some(point.slot_or_default());
                self.chain.rollbacks += 1;
                actions.push(Action::ContinueSync(pid.clone()));
                tracing::warn!(slot = point.slot_or_default(), "rollback");
            }

            // --- Leios overlay: notify ---
            InitiatorEvent::EbNotification(pid, notification) => {
                actions.extend(self.on_notification(pid, notification));
            }

            // --- Leios overlay: fetch ---
            InitiatorEvent::EbFetched(pid, eb, response) => match response {
                leiosfetch::Response::Block(body) => {
                    let n = eb_tx_count(body.raw_bytes());
                    self.overlay.bodies += 1;
                    self.overlay.bytes += body.raw_bytes().len() as u64;
                    if let Some(row) = self.ebs.get_mut(eb) {
                        row.tx_total = Some(n);
                        row.advance(EbStage::BodyFetched);
                    }
                    tracing::info!(eb = %fmt_eb(eb), bytes = body.raw_bytes().len(), txs = n, "eb body fetched");
                }
                leiosfetch::Response::BlockTxs { txs } => {
                    let bytes: usize = txs.iter().map(|t| t.raw_bytes().len()).sum();
                    self.overlay.txs_ebs += 1;
                    self.overlay.tx_count += txs.len() as u64;
                    self.overlay.bytes += bytes as u64;

                    // Page across the remaining 64-tx windows until the whole EB
                    // is fetched. We advance from the actual fetched count so a
                    // short response self-corrects; an empty response stops paging.
                    let mut next = None;
                    let mut fetched = 0;
                    if let Some(row) = self.ebs.get_mut(eb) {
                        row.tx_fetched += txs.len();
                        row.advance(EbStage::TxsFetched);
                        fetched = row.tx_fetched;
                        if let Some(total) = row.tx_total
                            && !txs.is_empty()
                            && row.tx_fetched < total
                        {
                            let start = row.tx_fetched;
                            let end = (start + MAX_TXS_PER_FETCH).min(total);
                            next = Some(Bitmaps::from_indices(start..end));
                        }
                    }
                    if let Some(bitmaps) = next {
                        actions.push(Action::FetchEbTxs(pid.clone(), eb.clone(), bitmaps));
                    }
                    tracing::info!(eb = %fmt_eb(eb), count = txs.len(), bytes, fetched, "eb txs fetched");
                }
            },

            // Block bodies / tx-submission requests are not part of this view.
            InitiatorEvent::BlockBodyReceived(..) | InitiatorEvent::TxRequested(..) => {}
        }

        actions
    }

    fn on_notification(
        &mut self,
        pid: &PeerId,
        notification: &leiosnotify::Notification,
    ) -> Vec<Action> {
        let mut actions = Vec::new();

        match notification {
            leiosnotify::Notification::BlockAnnouncement(raw) => {
                self.overlay.announced += 1;
                tracing::info!(bytes = raw.raw_bytes().len(), "eb announced");
            }
            leiosnotify::Notification::BlockOffer(eb, size) => {
                self.overlay.offered += 1;
                self.upsert_eb(eb).size = Some(*size);
                actions.push(Action::FetchEb(pid.clone(), eb.clone()));
                tracing::info!(eb = %fmt_eb(eb), size, "eb offered → fetching body");
            }
            leiosnotify::Notification::BlockTxsOffer(eb) => {
                self.overlay.txs_offered += 1;
                let total = self.ebs.get(eb).and_then(|r| r.tx_total);
                if let Some(row) = self.ebs.get_mut(eb) {
                    row.advance(EbStage::TxsOffered);
                }
                match total {
                    Some(n) if n > 0 => {
                        let want = n.min(MAX_TXS_PER_FETCH);
                        actions.push(Action::FetchEbTxs(
                            pid.clone(),
                            eb.clone(),
                            Bitmaps::all(want),
                        ));
                        tracing::info!(eb = %fmt_eb(eb), want, total = n, "txs offered → fetching");
                    }
                    _ => {
                        tracing::info!(eb = %fmt_eb(eb), "txs offered (body not yet fetched)");
                    }
                }
            }
            leiosnotify::Notification::Votes(votes) => {
                for vote in votes {
                    if let Some((eb_hash, voter)) = vote_meta(vote.raw_bytes()) {
                        self.overlay.votes += 1;
                        if let Some(v) = voter {
                            self.overlay.voters.insert(v);
                        }
                        if let Some(row) = self.ebs.values_mut().find(|r| r.hash == eb_hash) {
                            row.votes += 1;
                            if let Some(v) = voter {
                                row.voters.insert(v);
                            }
                        }
                    }
                }
                tracing::info!(count = votes.len(), "votes received");
            }
        }

        actions
    }

    /// Inserts (or returns) the row for an EB, trimming the oldest rows past the
    /// retention cap.
    fn upsert_eb(&mut self, eb: &EbId) -> &mut EbRow {
        if !self.ebs.contains_key(eb) {
            let (slot, hash) = match eb {
                Point::Specific(slot, hash) => (*slot, hash.clone()),
                Point::Origin => (0, Vec::new()),
            };
            self.ebs.insert(eb.clone(), EbRow::new(slot, hash));
            while self.ebs.len() > MAX_EBS {
                self.ebs.shift_remove_index(0);
            }
        }
        self.ebs.get_mut(eb).expect("just inserted")
    }

    fn set_tip(&mut self, tip: &pallas_network2::protocol::chainsync::Tip) {
        self.chain.tip_height = Some(tip.1);
        self.chain.tip_slot = Some(tip.0.slot_or_default());
    }

    /// Handles a key event, returning `true` if the app should quit.
    pub fn handle_input(&mut self, ev: crossterm::event::Event) -> bool {
        use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};

        let Event::Key(key) = ev else { return false };
        if key.kind != KeyEventKind::Press {
            return false;
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return true,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => return true,
            KeyCode::Char('c') => {
                if let Ok(mut buf) = self.log.lock() {
                    buf.clear();
                }
            }
            KeyCode::Char('f') => self.follow = !self.follow,
            KeyCode::Up => {
                self.follow = false;
                self.selected = self.selected.saturating_sub(1);
            }
            KeyCode::Down => {
                self.follow = false;
                self.selected = self.selected.saturating_add(1);
            }
            _ => {}
        }

        false
    }
}

/// Formats an EB reference (`[slot, hash]`) for logging.
pub fn fmt_eb(eb: &Point) -> String {
    match eb {
        Point::Origin => "origin".to_string(),
        Point::Specific(slot, hash) => format!("{slot}@{}", hex::encode(hash)),
    }
}

/// Counts the transactions in an EB body, which is a `{ tx_hash => size }` CBOR
/// map — the number of entries is the transaction count.
pub fn eb_tx_count(body: &[u8]) -> usize {
    let mut d = Decoder::new(body);
    match d.map() {
        Ok(Some(n)) => n as usize,
        Ok(None) => {
            let mut n = 0;
            while !matches!(d.datatype(), Ok(Type::Break)) {
                if d.skip().is_err() || d.skip().is_err() {
                    break;
                }
                n += 1;
            }
            n
        }
        Err(_) => 0,
    }
}

/// Decodes `(eb_hash, voter_id)` from a vote `[slot, eb_hash, voter_id, sig]`.
///
/// `voter_id` is a `uint` per the blueprint CDDL; it is returned as `None` if it
/// is absent or not an integer, so the vote is still attributed to its EB even
/// when the voter cannot be identified.
fn vote_meta(raw: &[u8]) -> Option<(Vec<u8>, Option<u64>)> {
    let mut d = Decoder::new(raw);
    d.array().ok()?;
    let _slot = d.u64().ok()?;
    let eb_hash = d.bytes().ok()?.to_vec();
    let voter = d.u64().ok();
    Some((eb_hash, voter))
}

/// Decodes `(block_number, slot)` from a non-Byron chain-sync header
/// (`header = [header_body, sig]`, `header_body = [block_no, slot, ...]`).
fn header_pos(header: &HeaderContent) -> Option<(u64, u64)> {
    if header.variant == 0 {
        return None;
    }
    let mut d = Decoder::new(&header.cbor);
    d.array().ok()?; // [header_body, body_signature]
    d.array().ok()?; // header_body = [block_no, slot, ...]
    let block_no = d.u64().ok()?;
    let slot = d.u64().ok()?;
    Some((block_no, slot))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pallas_codec::minicbor::Encoder;
    use pallas_codec::utils::AnyCbor;

    fn eb(slot: u64, hash: u8) -> EbId {
        Point::Specific(slot, vec![hash; 32])
    }

    /// Builds an EB body map with `n` `{hash => size}` entries.
    fn body(n: usize) -> AnyCbor {
        let mut buf = Vec::new();
        let mut e = Encoder::new(&mut buf);
        e.map(n as u64).unwrap();
        for i in 0..n {
            e.bytes(&[i as u8; 32]).unwrap().u32(100).unwrap();
        }
        AnyCbor::from_raw_bytes(buf)
    }

    /// Builds a vote `[slot, eb_hash, voter_id, sig]`.
    fn vote(eb_hash: u8, voter: u16) -> AnyCbor {
        let mut buf = Vec::new();
        Encoder::new(&mut buf)
            .array(4)
            .unwrap()
            .u64(1)
            .unwrap()
            .bytes(&[eb_hash; 32])
            .unwrap()
            .u16(voter)
            .unwrap()
            .bytes(&[0xEE; 48])
            .unwrap();
        AnyCbor::from_raw_bytes(buf)
    }

    fn dash() -> Dashboard {
        Dashboard::new("relay:3001".into(), 164, crate::logbuf::new_log())
    }

    fn pid() -> PeerId {
        PeerId {
            host: "relay".into(),
            port: 3001,
        }
    }

    #[test]
    fn eb_lifecycle_advances_through_all_stages() {
        let mut d = dash();
        let pid = pid();
        let id = eb(7, 0xAB);

        // Offer → fetch body command, row at Offered.
        let actions = d.apply_event(&InitiatorEvent::EbNotification(
            pid.clone(),
            leiosnotify::Notification::BlockOffer(id.clone(), 4096),
        ));
        assert_eq!(actions, vec![Action::FetchEb(pid.clone(), id.clone())]);
        assert_eq!(d.ebs[&id].stage, EbStage::Offered);
        assert_eq!(d.ebs[&id].size, Some(4096));

        // Body fetched → tx_total learned, stage BodyFetched.
        let actions = d.apply_event(&InitiatorEvent::EbFetched(
            pid.clone(),
            id.clone(),
            leiosfetch::Response::Block(body(12)),
        ));
        assert!(actions.is_empty());
        assert_eq!(d.ebs[&id].tx_total, Some(12));
        assert_eq!(d.ebs[&id].stage, EbStage::BodyFetched);

        // Txs offered → fetch sized from the known count.
        let actions = d.apply_event(&InitiatorEvent::EbNotification(
            pid.clone(),
            leiosnotify::Notification::BlockTxsOffer(id.clone()),
        ));
        assert_eq!(
            actions,
            vec![Action::FetchEbTxs(
                pid.clone(),
                id.clone(),
                Bitmaps::all(12)
            )]
        );
        assert_eq!(d.ebs[&id].stage, EbStage::TxsOffered);

        // Txs fetched → counts recorded, stage TxsFetched.
        let txs = vec![AnyCbor::from_raw_bytes(vec![1]); 12];
        let actions = d.apply_event(&InitiatorEvent::EbFetched(
            pid.clone(),
            id.clone(),
            leiosfetch::Response::BlockTxs { txs },
        ));
        assert!(actions.is_empty());
        assert_eq!(d.ebs[&id].tx_fetched, 12);
        assert_eq!(d.ebs[&id].stage, EbStage::TxsFetched);

        // Votes attributed back to the EB by hash.
        d.apply_event(&InitiatorEvent::EbNotification(
            pid,
            leiosnotify::Notification::Votes(vec![vote(0xAB, 1), vote(0xAB, 2), vote(0xAB, 2)]),
        ));
        assert_eq!(d.ebs[&id].votes, 3);
        assert_eq!(d.ebs[&id].voters.len(), 2);

        // Overlay counters reflect the whole flow.
        assert_eq!(d.overlay.offered, 1);
        assert_eq!(d.overlay.bodies, 1);
        assert_eq!(d.overlay.txs_offered, 1);
        assert_eq!(d.overlay.txs_ebs, 1);
        assert_eq!(d.overlay.tx_count, 12);
        assert_eq!(d.overlay.votes, 3);
        assert_eq!(d.overlay.voters.len(), 2);
    }

    #[test]
    fn txs_offer_without_body_does_not_fetch() {
        let mut d = dash();
        let pid = pid();
        let id = eb(7, 0xAB);

        d.apply_event(&InitiatorEvent::EbNotification(
            pid.clone(),
            leiosnotify::Notification::BlockOffer(id.clone(), 1),
        ));
        // Skip the body fetch; a txs offer now must not produce a fetch action.
        let actions = d.apply_event(&InitiatorEvent::EbNotification(
            pid,
            leiosnotify::Notification::BlockTxsOffer(id.clone()),
        ));
        assert!(actions.is_empty());
        assert_eq!(d.ebs[&id].stage, EbStage::TxsOffered);
    }

    #[test]
    fn stage_never_regresses() {
        let mut d = dash();
        let pid = pid();
        let id = eb(7, 0xAB);

        d.apply_event(&InitiatorEvent::EbNotification(
            pid.clone(),
            leiosnotify::Notification::BlockOffer(id.clone(), 1),
        ));
        d.apply_event(&InitiatorEvent::EbFetched(
            pid.clone(),
            id.clone(),
            leiosfetch::Response::Block(body(4)),
        ));
        // A late, duplicate offer must not pull the row back to Offered.
        d.apply_event(&InitiatorEvent::EbNotification(
            pid,
            leiosnotify::Notification::BlockOffer(id.clone(), 1),
        ));
        assert_eq!(d.ebs[&id].stage, EbStage::BodyFetched);
    }

    /// `n` dummy transactions.
    fn txs(n: usize) -> Vec<AnyCbor> {
        vec![AnyCbor::from_raw_bytes(vec![1]); n]
    }

    #[test]
    fn fetches_all_tx_windows_by_paging() {
        let mut d = dash();
        let pid = pid();
        let id = eb(7, 0xAB);

        // Offer + body so the tx count (150 → 3 windows) is known.
        d.apply_event(&InitiatorEvent::EbNotification(
            pid.clone(),
            leiosnotify::Notification::BlockOffer(id.clone(), 4096),
        ));
        d.apply_event(&InitiatorEvent::EbFetched(
            pid.clone(),
            id.clone(),
            leiosfetch::Response::Block(body(150)),
        ));

        // The txs offer fetches the first window (txs 0..64).
        let actions = d.apply_event(&InitiatorEvent::EbNotification(
            pid.clone(),
            leiosnotify::Notification::BlockTxsOffer(id.clone()),
        ));
        assert_eq!(
            actions,
            vec![Action::FetchEbTxs(
                pid.clone(),
                id.clone(),
                Bitmaps::all(64)
            )]
        );

        // Each full response pages into the next window…
        let actions = d.apply_event(&InitiatorEvent::EbFetched(
            pid.clone(),
            id.clone(),
            leiosfetch::Response::BlockTxs { txs: txs(64) },
        ));
        assert_eq!(
            actions,
            vec![Action::FetchEbTxs(
                pid.clone(),
                id.clone(),
                Bitmaps::from_indices(64..128)
            )]
        );

        let actions = d.apply_event(&InitiatorEvent::EbFetched(
            pid.clone(),
            id.clone(),
            leiosfetch::Response::BlockTxs { txs: txs(64) },
        ));
        assert_eq!(
            actions,
            vec![Action::FetchEbTxs(
                pid.clone(),
                id.clone(),
                Bitmaps::from_indices(128..150)
            )]
        );

        // …until the final partial window completes the EB — no further requests.
        let actions = d.apply_event(&InitiatorEvent::EbFetched(
            pid,
            id.clone(),
            leiosfetch::Response::BlockTxs { txs: txs(22) },
        ));
        assert!(actions.is_empty());

        assert_eq!(d.ebs[&id].tx_fetched, 150);
        assert_eq!(d.ebs[&id].stage, EbStage::TxsFetched);
        assert_eq!(d.overlay.tx_count, 150);
    }

    #[test]
    fn empty_txs_response_stops_paging() {
        let mut d = dash();
        let pid = pid();
        let id = eb(7, 0xAB);

        d.apply_event(&InitiatorEvent::EbNotification(
            pid.clone(),
            leiosnotify::Notification::BlockOffer(id.clone(), 1),
        ));
        d.apply_event(&InitiatorEvent::EbFetched(
            pid.clone(),
            id.clone(),
            leiosfetch::Response::Block(body(150)),
        ));
        // A peer that yields nothing must not spin us into endless re-requests.
        let actions = d.apply_event(&InitiatorEvent::EbFetched(
            pid,
            id.clone(),
            leiosfetch::Response::BlockTxs { txs: txs(0) },
        ));
        assert!(actions.is_empty());
    }
}
