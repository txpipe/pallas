use pallas_codec::Fragment;

use super::{Client, ClientError, Message, NextResponse, State, Tip};

/// Default maximum number of pipelined `RequestNext` messages in flight.
///
/// Chosen to stay well below the multiplexer's 100-slot per-channel queues so
/// pipelined chain-sync traffic cannot starve sibling mini-protocols such as
/// keep-alive or block-fetch sharing the same bearer.
pub const DEFAULT_PIPELINE_WINDOW: usize = 64;

/// Upper bound enforced by [`Pipeline`] on concurrent in-flight `RequestNext`
/// messages to keep the multiplexer's 100-slot queues safe for concurrent
/// mini-protocols.
pub const MAX_PIPELINE_WINDOW: usize = 64;

/// Windowing policy used by [`Pipeline`] to decide how many `RequestNext`
/// requests to keep in flight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PipelineMode {
    /// Catch up to the peer's advertised `tip.1` using reported local block
    /// heights and return `Ok(None)` once `in_flight == 0` at tip.
    ///
    /// Stays in single-request probe mode after intersect or rollback until a
    /// `RollForward` establishes the local block number via
    /// [`Pipeline::set_local_height`]. Never issues extra `RequestNext`
    /// messages past `tip.1`; if `tip.1` shrinks mid-flight, stops refilling
    /// and drains remaining replies to `0` before returning `Ok(None)`.
    StopAtTip,

    /// Catch up to the peer's advertised `tip.1` and then keep at most `1`
    /// active `RequestNext` in flight to follow new live blocks.
    ///
    /// Uses [`Pipeline::set_local_height`] to taper smoothly to `1` at tip and
    /// stays in single-request probe mode after intersect or rollback.
    FollowTip,

    /// Fixed-window pipelining without block-height tracking.
    ///
    /// Sends a single probe request after intersect, rollback, or `AwaitReply`,
    /// then pipelines up to `max_in_flight` requests once `RollForward`
    /// responses are flowing. Useful for callers that do not decode headers
    /// (such as raw block streaming), though it may overshoot `tip.1` by up to
    /// `max_in_flight - 1` requests when reaching the chain tip.
    FixedWindow,
}

/// Bounded-window pipelining controller for a ChainSync [`Client`].
///
/// `Pipeline` borrows `&mut Client<O>` on each step, so it works equally well
/// with `PeerClient::chainsync()` (node-to-node) and `NodeClient::chainsync()`
/// (node-to-client).
///
/// # Typical usage (tip-aware catch-up)
///
/// ```no_run
/// # use pallas_network::miniprotocols::{Point, chainsync::{self, NextResponse}};
/// # async fn sync(
/// #     client: &mut chainsync::N2NClient,
/// #     known_points: Vec<Point>,
/// #     no_service: bool,
/// #     decode_block_number: impl Fn(&chainsync::HeaderContent) -> u64,
/// # ) -> Result<(), chainsync::ClientError> {
/// let mut pipeline = if no_service {
///     chainsync::Pipeline::stop_at_tip(chainsync::DEFAULT_PIPELINE_WINDOW)
/// } else {
///     chainsync::Pipeline::follow_tip(chainsync::DEFAULT_PIPELINE_WINDOW)
/// };
///
/// let (_, tip) = client.find_intersect(known_points).await?;
/// pipeline.reset_after_intersect(&tip);
///
/// while let Some(next) = pipeline.next(client).await? {
///     match next {
///         NextResponse::RollForward(header, _tip) => {
///             pipeline.set_local_height(decode_block_number(&header));
///         }
///         NextResponse::RollBackward(_point, _tip) => {
///             // Revert orphaned blocks in arrival order; Pipeline has already
///             // returned to probe mode unless the rollback landed on tip.
///         }
///         NextResponse::Await => {
///             // Pipeline pauses refills while the client is in MustReply.
///         }
///     }
/// }
///
/// // When `next` returns `Ok(None)` in `StopAtTip` mode, `client.in_flight() == 0`
/// // and `*client.state() == chainsync::State::Idle`.
/// client.send_done().await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct Pipeline {
    mode: PipelineMode,
    max_in_flight: usize,
    min_refill: usize,
    peer_tip: Option<u64>,
    local_height: Option<u64>,
    fixed_active: bool,
}

impl Pipeline {
    /// Create a pipeline controller with the given [`PipelineMode`] and maximum
    /// in-flight window.
    ///
    /// `max_in_flight` is clamped to `1..=MAX_PIPELINE_WINDOW` (`64`).
    pub fn new(mode: PipelineMode, max_in_flight: usize) -> Self {
        let max_in_flight = max_in_flight.clamp(1, MAX_PIPELINE_WINDOW);
        Self {
            mode,
            max_in_flight,
            min_refill: 1,
            peer_tip: None,
            local_height: None,
            fixed_active: false,
        }
    }

    /// Create a one-shot pipeline that stops cleanly at the peer's advertised
    /// tip with `in_flight == 0`.
    pub fn stop_at_tip(max_in_flight: usize) -> Self {
        Self::new(PipelineMode::StopAtTip, max_in_flight)
    }

    /// Create a continuous pipeline that catches up behind the tip and then
    /// keeps at most `1` active request while following live blocks.
    pub fn follow_tip(max_in_flight: usize) -> Self {
        Self::new(PipelineMode::FollowTip, max_in_flight)
    }

    /// Create a fixed-window pipeline that does not require block-height
    /// feedback.
    pub fn fixed_window(max_in_flight: usize) -> Self {
        Self::new(PipelineMode::FixedWindow, max_in_flight)
    }

    /// Set the minimum refill batch size used once requests are already in
    /// flight.
    ///
    /// Values are clamped to `1..=max_in_flight`. The default is `1` (refill on
    /// every step to keep `in_flight` maximally full). Larger values (for
    /// example `16`) coalesce outbound `RequestNext` batches into fewer
    /// multiplexer segments and syscalls while always refilling immediately
    /// whenever `in_flight == 0` or `in_flight <= target / 2`.
    pub fn with_min_refill(mut self, min_refill: usize) -> Self {
        self.min_refill = min_refill.clamp(1, self.max_in_flight);
        self
    }

    /// Returns the configured [`PipelineMode`].
    pub fn mode(&self) -> PipelineMode {
        self.mode
    }

    /// Returns the clamped maximum in-flight request window.
    pub fn max_in_flight(&self) -> usize {
        self.max_in_flight
    }

    /// Returns the configured minimum refill batch size.
    pub fn min_refill(&self) -> usize {
        self.min_refill
    }

    /// Returns the most recently advertised peer tip block number, if known.
    pub fn peer_tip_height(&self) -> Option<u64> {
        self.peer_tip
    }

    /// Returns the most recently reported local block height, if known.
    pub fn local_height(&self) -> Option<u64> {
        self.local_height
    }

    /// Reset pipeline tracking after [`Client::find_intersect`], recording the
    /// peer's advertised tip and entering single-request probe mode.
    pub fn reset_after_intersect(&mut self, tip: &Tip) {
        self.peer_tip = Some(tip.1);
        self.local_height = None;
        self.fixed_active = false;
    }

    /// Report the block number of the most recently received `RollForward`
    /// header or block.
    ///
    /// In [`PipelineMode::StopAtTip`] and [`PipelineMode::FollowTip`], this
    /// unlocks pipelining up to `min(max_in_flight, tip.1 - block_number)`.
    pub fn set_local_height(&mut self, block_number: u64) {
        self.local_height = Some(block_number);
    }

    /// Clear the known local height, returning to single-request probe mode
    /// until the next [`Self::set_local_height`] call.
    pub fn clear_local_height(&mut self) {
        self.local_height = None;
        self.fixed_active = false;
    }

    /// Compute the desired number of in-flight `RequestNext` messages for the
    /// client's current protocol state.
    pub fn target_in_flight(&self, state: &State) -> usize {
        if !matches!(state, State::Idle | State::CanAwait) {
            return 0;
        }

        match self.mode {
            PipelineMode::StopAtTip => match (self.local_height, self.peer_tip) {
                (Some(local), Some(tip)) => {
                    let remaining = tip.saturating_sub(local);
                    usize::try_from(remaining)
                        .unwrap_or(self.max_in_flight)
                        .min(self.max_in_flight)
                }
                _ => 1,
            },
            PipelineMode::FollowTip => match (self.local_height, self.peer_tip) {
                (Some(local), Some(tip)) => {
                    let remaining = tip.saturating_sub(local);
                    usize::try_from(remaining)
                        .unwrap_or(self.max_in_flight)
                        .min(self.max_in_flight)
                        .max(1)
                }
                _ => 1,
            },
            PipelineMode::FixedWindow => {
                if self.fixed_active {
                    self.max_in_flight
                } else {
                    1
                }
            }
        }
    }

    fn record_response<O>(&mut self, response: &NextResponse<O>) {
        match response {
            NextResponse::RollForward(_, tip) => {
                self.peer_tip = Some(tip.1);
                self.fixed_active = true;
            }
            NextResponse::RollBackward(point, tip) => {
                self.peer_tip = Some(tip.1);
                self.fixed_active = false;
                if *point == tip.0 {
                    // Already at the peer's tip; in `StopAtTip` this lets the
                    // next iteration return `Ok(None)` without sending an extra
                    // `RequestNext` that would trigger `AwaitReply`.
                    self.local_height = Some(tip.1);
                } else {
                    self.local_height = None;
                }
            }
            NextResponse::Await => {
                self.fixed_active = false;
            }
        }
    }

    /// Refill the pipeline if warranted and receive the next ChainSync reply.
    ///
    /// In [`PipelineMode::StopAtTip`], returns `Ok(None)` once the advertised
    /// tip has been reached **and** all in-flight requests have drained to `0`,
    /// leaving `client` in [`State::Idle`] ready for [`Client::send_done`].
    ///
    /// # Errors
    ///
    /// Propagates any [`ClientError`] from sending or receiving.
    pub async fn next<O>(
        &mut self,
        client: &mut Client<O>,
    ) -> Result<Option<NextResponse<O>>, ClientError>
    where
        Message<O>: Fragment,
    {
        let in_flight = client.in_flight();
        let target = self.target_in_flight(client.state());

        if in_flight == 0 && target == 0 {
            return match client.state() {
                State::Idle => Ok(None),
                _ => Err(ClientError::InvalidInbound),
            };
        }

        if client.can_pipeline_request_next() {
            let needed = target.saturating_sub(in_flight);
            let should_refill = needed > 0
                && (in_flight == 0 || needed >= self.min_refill || in_flight <= target / 2);

            if should_refill {
                client.send_request_next_n(needed).await?;
            }
        }

        let response = client.recv_next().await?;
        self.record_response(&response);
        Ok(Some(response))
    }

    /// Receive one already-in-flight reply without sending any new
    /// `RequestNext` messages.
    ///
    /// Returns `Ok(None)` once `client.in_flight() == 0`, allowing callers to
    /// drain an active pipeline cleanly before calling [`Client::send_done`].
    pub async fn drain_next<O>(
        &mut self,
        client: &mut Client<O>,
    ) -> Result<Option<NextResponse<O>>, ClientError>
    where
        Message<O>: Fragment,
    {
        if client.in_flight() == 0 {
            return Ok(None);
        }

        let response = client.recv_next().await?;
        self.record_response(&response);
        Ok(Some(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::miniprotocols::Point;

    #[test]
    fn clamps_window_and_min_refill() {
        let low = Pipeline::stop_at_tip(0).with_min_refill(0);
        assert_eq!(low.max_in_flight(), 1);
        assert_eq!(low.min_refill(), 1);

        let high = Pipeline::follow_tip(10_000).with_min_refill(999);
        assert_eq!(high.max_in_flight(), MAX_PIPELINE_WINDOW);
        assert_eq!(high.min_refill(), MAX_PIPELINE_WINDOW);
    }

    #[test]
    fn target_respects_probe_tip_gap_and_must_reply() {
        let mut p = Pipeline::stop_at_tip(64);
        assert_eq!(p.target_in_flight(&State::Idle), 1);

        let tip = Tip(Point::Specific(1000, vec![1]), 1000);
        p.reset_after_intersect(&tip);
        assert_eq!(p.target_in_flight(&State::Idle), 1);
        assert_eq!(p.target_in_flight(&State::MustReply), 0);

        p.set_local_height(500);
        assert_eq!(p.target_in_flight(&State::CanAwait), 64);

        p.set_local_height(990);
        assert_eq!(p.target_in_flight(&State::CanAwait), 10);

        p.set_local_height(1000);
        assert_eq!(p.target_in_flight(&State::CanAwait), 0);

        // In FollowTip mode, target stays at 1 at tip.
        let mut follow = Pipeline::follow_tip(64);
        follow.reset_after_intersect(&tip);
        follow.set_local_height(1000);
        assert_eq!(follow.target_in_flight(&State::Idle), 1);
        assert_eq!(follow.target_in_flight(&State::MustReply), 0);
    }

    #[test]
    fn rollback_to_tip_sets_height_otherwise_enters_probe() {
        let tip_point = Point::Specific(500, vec![0xaa]);
        let older_point = Point::Specific(400, vec![0xbb]);
        let tip = Tip(tip_point.clone(), 50);

        let mut p = Pipeline::stop_at_tip(64);
        p.set_local_height(45);

        p.record_response(&NextResponse::<()>::RollBackward(older_point, tip.clone()));
        assert_eq!(p.local_height(), None);
        assert_eq!(p.target_in_flight(&State::Idle), 1);

        p.record_response(&NextResponse::<()>::RollBackward(tip_point, tip));
        assert_eq!(p.local_height(), Some(50));
        assert_eq!(p.target_in_flight(&State::Idle), 0);
    }
}
