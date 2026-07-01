//! `FIX` session gap recovery.
//!
//! Implements the message-loss / duplicate-detection state machine required
//! by the `FIX 4.4` / `5.0` session layer:
//!
//! - Detect gaps in the incoming `MsgSeqNum` stream.
//! - Emit `ResendRequest` (`35=2`) covering the missing range.
//! - Fill gaps administratively via `SequenceReset-GapFill` (`35=4`,
//!   `GapFillFlag=Y`, `NewSeqNo=<next>`).
//! - Recognise inbound `SequenceReset-Reset` (`35=4`, `GapFillFlag=N`) and
//!   snap the receive sequence forward.

// ---------------------------------------------------------------------------
// Enumerations
// ---------------------------------------------------------------------------

/// Outcome of processing one inbound sequence number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InboundOutcome {
    /// Sequence number was exactly the next expected value; deliver.
    Deliver,
    /// A gap was detected. The engine expects a `ResendRequest` covering
    /// `[begin_seq_no, end_seq_no]` (`end_seq_no = 0` means "infinity" per
    /// `FIX 4.4`).
    RequestResend { begin_seq_no: u64, end_seq_no: u64 },
    /// The message was older than the current tail and must be discarded.
    Duplicate,
    /// The message is a valid `SequenceReset`-`GapFill` that advances the
    /// receive counter to `new_seq_no`.
    GapFillAccepted { new_seq_no: u64 },
    /// The message is a valid `SequenceReset`-`Reset` (administrative);
    /// receive counter jumps to `new_seq_no`.
    ResetAccepted { new_seq_no: u64 },
    /// A `SequenceReset` attempted to move the counter backwards; per
    /// `FIX 4.4 Vol.2 §Recovering from an Out-of-Sequence Message`, reject
    /// with a `Session Reject` (`35=3`).
    RejectBackwardsReset,
}

// ---------------------------------------------------------------------------
// Session state
// ---------------------------------------------------------------------------

/// Minimal state machine for FIX gap recovery.
///
/// Tracks the next expected `MsgSeqNum` for received messages. Emit / send
/// paths are the caller's responsibility; this struct only makes decisions.
#[derive(Debug, Clone)]
pub struct GapRecoveryState {
    next_in_seq: u64,
}

impl GapRecoveryState {
    /// Fresh session state, starting from sequence number `1` per FIX
    /// convention.
    #[must_use]
    pub const fn new() -> Self {
        Self { next_in_seq: 1 }
    }

    /// Fresh session state seeded to a specific starting number (useful when
    /// resuming after a persistence snapshot).
    #[must_use]
    pub const fn with_next_in_seq(next_in_seq: u64) -> Self {
        Self { next_in_seq }
    }

    /// Next expected inbound sequence number.
    #[must_use]
    pub const fn next_in_seq(&self) -> u64 {
        self.next_in_seq
    }

    /// Process an inbound application message with `seq_no` (`MsgSeqNum`).
    pub fn on_application(&mut self, seq_no: u64) -> InboundOutcome {
        match seq_no.cmp(&self.next_in_seq) {
            std::cmp::Ordering::Equal => {
                self.next_in_seq += 1;
                InboundOutcome::Deliver
            }
            std::cmp::Ordering::Less => InboundOutcome::Duplicate,
            std::cmp::Ordering::Greater => InboundOutcome::RequestResend {
                begin_seq_no: self.next_in_seq,
                end_seq_no: seq_no.saturating_sub(1),
            },
        }
    }

    /// Process an inbound `SequenceReset` (`35=4`).
    ///
    /// - `gap_fill = true` corresponds to `GapFillFlag=Y` (administrative
    ///   fill after a `ResendRequest`).
    /// - `gap_fill = false` is the raw `SequenceReset-Reset` mode.
    pub fn on_sequence_reset(&mut self, new_seq_no: u64, gap_fill: bool) -> InboundOutcome {
        if new_seq_no < self.next_in_seq {
            return InboundOutcome::RejectBackwardsReset;
        }
        self.next_in_seq = new_seq_no;
        if gap_fill {
            InboundOutcome::GapFillAccepted { new_seq_no }
        } else {
            InboundOutcome::ResetAccepted { new_seq_no }
        }
    }
}

impl Default for GapRecoveryState {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// ResendRequest builder
// ---------------------------------------------------------------------------

/// Construct a canonical `ResendRequest` payload (`BeginSeqNo` + `EndSeqNo`)
/// suitable for passing to a `FIX` message serialiser.
#[must_use]
pub fn build_resend_request(begin_seq_no: u64, end_seq_no: u64) -> Vec<(u32, String)> {
    vec![
        (35, "2".to_owned()),
        (7, begin_seq_no.to_string()),
        (16, end_seq_no.to_string()),
    ]
}

/// Construct a canonical `SequenceReset-GapFill` message.
#[must_use]
pub fn build_gap_fill(msg_seq_num: u64, new_seq_no: u64) -> Vec<(u32, String)> {
    vec![
        (35, "4".to_owned()),
        (34, msg_seq_num.to_string()),
        (36, new_seq_no.to_string()),
        (123, "Y".to_owned()),
    ]
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_session_expects_seq_one() {
        let s = GapRecoveryState::new();
        assert_eq!(s.next_in_seq(), 1);
    }

    #[test]
    fn in_order_delivery_advances_counter() {
        let mut s = GapRecoveryState::new();
        assert_eq!(s.on_application(1), InboundOutcome::Deliver);
        assert_eq!(s.on_application(2), InboundOutcome::Deliver);
        assert_eq!(s.next_in_seq(), 3);
    }

    #[test]
    fn gap_triggers_resend_request() {
        let mut s = GapRecoveryState::new();
        s.on_application(1);
        assert_eq!(
            s.on_application(5),
            InboundOutcome::RequestResend {
                begin_seq_no: 2,
                end_seq_no: 4,
            }
        );
        // Counter must not advance; caller drives resend + gap fill.
        assert_eq!(s.next_in_seq(), 2);
    }

    #[test]
    fn duplicate_message_is_discarded() {
        let mut s = GapRecoveryState::new();
        s.on_application(1);
        s.on_application(2);
        assert_eq!(s.on_application(1), InboundOutcome::Duplicate);
    }

    #[test]
    fn gap_fill_advances_counter() {
        let mut s = GapRecoveryState::new();
        s.on_application(1);
        s.on_application(5); // triggers RequestResend
        assert_eq!(
            s.on_sequence_reset(5, true),
            InboundOutcome::GapFillAccepted { new_seq_no: 5 }
        );
        assert_eq!(s.next_in_seq(), 5);
    }

    #[test]
    fn plain_reset_advances_counter() {
        let mut s = GapRecoveryState::new();
        assert_eq!(
            s.on_sequence_reset(100, false),
            InboundOutcome::ResetAccepted { new_seq_no: 100 }
        );
        assert_eq!(s.next_in_seq(), 100);
    }

    #[test]
    fn backwards_reset_is_rejected() {
        let mut s = GapRecoveryState::with_next_in_seq(10);
        assert_eq!(
            s.on_sequence_reset(5, false),
            InboundOutcome::RejectBackwardsReset
        );
        // Counter must not move.
        assert_eq!(s.next_in_seq(), 10);
    }

    #[test]
    fn resend_request_builder_uses_expected_tags() {
        let msg = build_resend_request(2, 4);
        assert!(msg.contains(&(35, "2".to_owned())));
        assert!(msg.contains(&(7, "2".to_owned())));
        assert!(msg.contains(&(16, "4".to_owned())));
    }

    #[test]
    fn gap_fill_builder_sets_gap_fill_flag() {
        let msg = build_gap_fill(5, 10);
        assert!(msg.contains(&(35, "4".to_owned())));
        assert!(msg.contains(&(34, "5".to_owned())));
        assert!(msg.contains(&(36, "10".to_owned())));
        assert!(msg.contains(&(123, "Y".to_owned())));
    }

    #[test]
    fn after_gap_fill_next_in_order_message_delivers() {
        let mut s = GapRecoveryState::new();
        s.on_application(1);
        s.on_application(5); // triggers resend request
        s.on_sequence_reset(5, true);
        assert_eq!(s.on_application(5), InboundOutcome::Deliver);
        assert_eq!(s.next_in_seq(), 6);
    }
}
