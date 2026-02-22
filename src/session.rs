/*
    ALICE-FIX
    Copyright (C) 2026 Moroya Sakamoto
*/

//! FIX session state machine.
//!
//! [`FixSession`] manages the state and sequence numbers for a single FIX
//! session. It provides helpers to build standard administrative messages
//! (Logon, Logout, Heartbeat) and to construct a NewOrderSingle (35=D)
//! from an ALICE-Ledger [`Order`].
//!
//! ## Session States
//!
//! ```text
//! Disconnected → (send Logon) → LogonSent → (receive Logon) → Active
//! Active → (send Logout) → LogoutSent → (receive Logout) → Disconnected
//! ```

use crate::builder::FixBuilder;
use crate::convert::{alice_ord_type_to_fix, alice_side_to_fix, alice_tif_to_fix};
use crate::tag;
use alice_ledger::Order;

/// Operational state of a FIX session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// No connection established; no messages exchanged.
    Disconnected,
    /// Logon message has been sent; waiting for the counterparty's Logon.
    LogonSent,
    /// Session is fully established and messages may be exchanged.
    Active,
    /// Logout message has been sent; waiting for the counterparty's Logout.
    LogoutSent,
}

/// FIX session context tracking sequence numbers and administrative state.
pub struct FixSession {
    sender_comp_id: String,
    target_comp_id: String,
    begin_string: String,
    /// Next sequence number to assign to an outgoing message.
    outgoing_seq: u64,
    /// Next sequence number expected from the counterparty.
    incoming_seq: u64,
    state: SessionState,
}

impl FixSession {
    /// Create a new session in the [`SessionState::Disconnected`] state.
    ///
    /// Sequence numbers start at 1 per FIX specification.
    #[inline(always)]
    pub fn new(sender: &str, target: &str, begin_string: &str) -> Self {
        Self {
            sender_comp_id: sender.to_string(),
            target_comp_id: target.to_string(),
            begin_string: begin_string.to_string(),
            outgoing_seq: 1,
            incoming_seq: 1,
            state: SessionState::Disconnected,
        }
    }

    /// Return the current session state.
    #[inline(always)]
    pub fn state(&self) -> &SessionState {
        &self.state
    }

    /// Increment the outgoing sequence number and return the value assigned
    /// to the next message.
    ///
    /// The counter starts at 1; the first call returns 1.
    #[inline(always)]
    pub fn next_outgoing_seq(&mut self) -> u64 {
        let seq = self.outgoing_seq;
        self.outgoing_seq += 1;
        seq
    }

    /// Validate that an incoming message has the expected sequence number.
    ///
    /// Returns `true` and advances the expected counter when the sequence
    /// matches; returns `false` without updating state when it does not.
    #[inline(always)]
    pub fn validate_incoming_seq(&mut self, seq: u64) -> bool {
        if seq == self.incoming_seq {
            self.incoming_seq += 1;
            true
        } else {
            false
        }
    }

    /// Build a Logon message (MsgType "A") and transition to
    /// [`SessionState::LogonSent`].
    pub fn build_logon(&mut self) -> Vec<u8> {
        let seq = self.next_outgoing_seq();
        self.state = SessionState::LogonSent;
        self.build_admin("A", seq)
    }

    /// Build a Logout message (MsgType "5") and transition to
    /// [`SessionState::LogoutSent`].
    pub fn build_logout(&mut self) -> Vec<u8> {
        let seq = self.next_outgoing_seq();
        self.state = SessionState::LogoutSent;
        self.build_admin("5", seq)
    }

    /// Build a Heartbeat message (MsgType "0") without changing session state.
    pub fn build_heartbeat(&mut self) -> Vec<u8> {
        let seq = self.next_outgoing_seq();
        self.build_admin("0", seq)
    }

    /// Build a NewOrderSingle (MsgType "D") from an ALICE-Ledger [`Order`].
    ///
    /// The `symbol` parameter provides the instrument identifier (tag 55),
    /// since [`Order`] does not carry a symbol string.
    pub fn build_new_order(&mut self, order: &Order, symbol: &str) -> Vec<u8> {
        let seq = self.next_outgoing_seq();
        let price_str = order.price.to_string();
        let qty_str = order.quantity.to_string();
        let cl_ord_id = order.id.0.to_string();

        FixBuilder::new(&self.begin_string, "D")
            .field(tag::SENDER_COMP_ID, &self.sender_comp_id)
            .field(tag::TARGET_COMP_ID, &self.target_comp_id)
            .field_u64(tag::MSG_SEQ_NUM, seq)
            .field(tag::CL_ORD_ID, &cl_ord_id)
            .field(tag::SYMBOL, symbol)
            .field(tag::SIDE, alice_side_to_fix(order.side))
            .field(tag::ORD_TYPE, alice_ord_type_to_fix(order.order_type))
            .field(tag::PRICE, &price_str)
            .field(tag::ORDER_QTY, &qty_str)
            .field(tag::TIME_IN_FORCE, alice_tif_to_fix(order.time_in_force))
            .build()
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    /// Construct a minimal administrative message with standard header fields.
    fn build_admin(&self, msg_type: &str, seq: u64) -> Vec<u8> {
        FixBuilder::new(&self.begin_string, msg_type)
            .field(tag::SENDER_COMP_ID, &self.sender_comp_id)
            .field(tag::TARGET_COMP_ID, &self.target_comp_id)
            .field_u64(tag::MSG_SEQ_NUM, seq)
            .build()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;
    use crate::tag;
    use alice_ledger::{Order, OrderId, OrderType, Side, TimeInForce};

    fn make_session() -> FixSession {
        FixSession::new("ALICE", "BROKER", "FIX.4.4")
    }

    fn make_limit_order(id: u64, side: Side, price: i64, qty: u64) -> Order {
        Order {
            id: OrderId(id),
            side,
            order_type: OrderType::Limit,
            price,
            quantity: qty,
            filled_quantity: 0,
            timestamp_ns: 0,
            time_in_force: TimeInForce::GTC,
        }
    }

    #[test]
    fn test_session_creation() {
        let session = make_session();
        assert_eq!(*session.state(), SessionState::Disconnected);
    }

    #[test]
    fn test_outgoing_seq_increments() {
        let mut session = make_session();
        assert_eq!(session.next_outgoing_seq(), 1);
        assert_eq!(session.next_outgoing_seq(), 2);
        assert_eq!(session.next_outgoing_seq(), 3);
    }

    #[test]
    fn test_incoming_seq_validation() {
        let mut session = make_session();
        // Sequence 1 is expected first.
        assert!(session.validate_incoming_seq(1));
        // Now sequence 2 is expected.
        assert!(session.validate_incoming_seq(2));
        // Sequence 1 again is out of order.
        assert!(!session.validate_incoming_seq(1));
        // Sequence 4 is a gap.
        assert!(!session.validate_incoming_seq(4));
        // Sequence 3 is the correct next.
        assert!(session.validate_incoming_seq(3));
    }

    #[test]
    fn test_build_logon_message() {
        let mut session = make_session();
        let bytes = session.build_logon();
        let msg = parser::parse(&bytes).expect("logon should parse");

        assert_eq!(msg.msg_type, "A");
        assert_eq!(msg.get(tag::SENDER_COMP_ID), Some("ALICE"));
        assert_eq!(msg.get(tag::TARGET_COMP_ID), Some("BROKER"));
        assert_eq!(msg.get_u64(tag::MSG_SEQ_NUM), Some(1));
        assert_eq!(*session.state(), SessionState::LogonSent);
    }

    #[test]
    fn test_build_logout_message() {
        let mut session = make_session();
        let bytes = session.build_logout();
        let msg = parser::parse(&bytes).expect("logout should parse");
        assert_eq!(msg.msg_type, "5");
        assert_eq!(*session.state(), SessionState::LogoutSent);
    }

    #[test]
    fn test_build_heartbeat_message() {
        let mut session = make_session();
        let bytes = session.build_heartbeat();
        let msg = parser::parse(&bytes).expect("heartbeat should parse");
        assert_eq!(msg.msg_type, "0");
        // State should remain Disconnected (heartbeat does not change state).
        assert_eq!(*session.state(), SessionState::Disconnected);
    }

    #[test]
    fn test_build_new_order() {
        let mut session = make_session();
        let order = make_limit_order(42, Side::Bid, 50_000, 10);
        let bytes = session.build_new_order(&order, "BTCUSD");
        let msg = parser::parse(&bytes).expect("new order should parse");

        assert_eq!(msg.msg_type, "D");
        assert_eq!(msg.get(tag::SYMBOL), Some("BTCUSD"));
        assert_eq!(msg.get(tag::SIDE), Some("1")); // Bid = "1"
        assert_eq!(msg.get(tag::ORD_TYPE), Some("2")); // Limit = "2"
        assert_eq!(msg.get_i64(tag::PRICE), Some(50_000));
        assert_eq!(msg.get_u64(tag::ORDER_QTY), Some(10));
        assert_eq!(msg.get(tag::CL_ORD_ID), Some("42"));
    }

    #[test]
    fn test_seq_advances_across_messages() {
        let mut session = make_session();
        let b1 = session.build_logon();
        let b2 = session.build_heartbeat();
        let b3 = session.build_heartbeat();

        let m1 = parser::parse(&b1).unwrap();
        let m2 = parser::parse(&b2).unwrap();
        let m3 = parser::parse(&b3).unwrap();

        assert_eq!(m1.get_u64(tag::MSG_SEQ_NUM), Some(1));
        assert_eq!(m2.get_u64(tag::MSG_SEQ_NUM), Some(2));
        assert_eq!(m3.get_u64(tag::MSG_SEQ_NUM), Some(3));
    }

    // -----------------------------------------------------------------------
    // Additional session tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_logon_changes_state_to_logon_sent() {
        let mut session = make_session();
        assert_eq!(*session.state(), SessionState::Disconnected);
        let _ = session.build_logon();
        assert_eq!(*session.state(), SessionState::LogonSent);
    }

    #[test]
    fn test_logout_changes_state_to_logout_sent() {
        let mut session = make_session();
        let _ = session.build_logout();
        assert_eq!(*session.state(), SessionState::LogoutSent);
    }

    #[test]
    fn test_heartbeat_does_not_change_state() {
        let mut session = make_session();
        let _ = session.build_logon();
        assert_eq!(*session.state(), SessionState::LogonSent);
        let _ = session.build_heartbeat();
        // State should remain LogonSent.
        assert_eq!(*session.state(), SessionState::LogonSent);
    }

    #[test]
    fn test_multiple_logons_advance_seq() {
        let mut session = make_session();
        let b1 = session.build_logon();
        let b2 = session.build_logon();
        let m1 = parser::parse(&b1).unwrap();
        let m2 = parser::parse(&b2).unwrap();
        assert_eq!(m1.get_u64(tag::MSG_SEQ_NUM), Some(1));
        assert_eq!(m2.get_u64(tag::MSG_SEQ_NUM), Some(2));
    }

    #[test]
    fn test_incoming_seq_starts_at_one() {
        let mut session = make_session();
        assert!(!session.validate_incoming_seq(0));
        assert!(session.validate_incoming_seq(1));
    }

    #[test]
    fn test_incoming_seq_gap_rejection() {
        let mut session = make_session();
        assert!(session.validate_incoming_seq(1));
        // Skip 2, send 3 -> should fail.
        assert!(!session.validate_incoming_seq(3));
        // Sequence 2 is still expected.
        assert!(session.validate_incoming_seq(2));
    }

    #[test]
    fn test_build_new_order_ask_side() {
        let mut session = make_session();
        let order = Order {
            id: OrderId(100),
            side: Side::Ask,
            order_type: OrderType::Limit,
            price: 60_000,
            quantity: 25,
            filled_quantity: 0,
            timestamp_ns: 0,
            time_in_force: TimeInForce::IOC,
        };
        let bytes = session.build_new_order(&order, "ETHUSD");
        let msg = parser::parse(&bytes).unwrap();
        assert_eq!(msg.msg_type, "D");
        assert_eq!(msg.get(tag::SIDE), Some("2")); // Ask = "2"
        assert_eq!(msg.get(tag::SYMBOL), Some("ETHUSD"));
        assert_eq!(msg.get(tag::TIME_IN_FORCE), Some("3")); // IOC = "3"
        assert_eq!(msg.get_u64(tag::ORDER_QTY), Some(25));
    }

    #[test]
    fn test_build_new_order_market_type() {
        let mut session = make_session();
        let order = Order {
            id: OrderId(200),
            side: Side::Bid,
            order_type: OrderType::Market,
            price: 0,
            quantity: 50,
            filled_quantity: 0,
            timestamp_ns: 0,
            time_in_force: TimeInForce::FOK,
        };
        let bytes = session.build_new_order(&order, "BTCUSD");
        let msg = parser::parse(&bytes).unwrap();
        assert_eq!(msg.get(tag::ORD_TYPE), Some("1")); // Market = "1"
        assert_eq!(msg.get(tag::TIME_IN_FORCE), Some("4")); // FOK = "4"
    }

    #[test]
    fn test_session_state_debug() {
        let state = SessionState::Active;
        assert_eq!(format!("{state:?}"), "Active");
    }

    #[test]
    fn test_session_state_clone() {
        let s1 = SessionState::LogonSent;
        let s2 = s1;
        assert_eq!(s1, s2);
    }

    #[test]
    fn test_full_session_lifecycle_seq_numbers() {
        let mut session = make_session();
        // Logon = seq 1
        let b1 = session.build_logon();
        assert_eq!(*session.state(), SessionState::LogonSent);
        // Heartbeat = seq 2
        let b2 = session.build_heartbeat();
        // New order = seq 3
        let order = make_limit_order(1, Side::Bid, 100, 10);
        let b3 = session.build_new_order(&order, "SYM");
        // Logout = seq 4
        let b4 = session.build_logout();
        assert_eq!(*session.state(), SessionState::LogoutSent);

        let m1 = parser::parse(&b1).unwrap();
        let m2 = parser::parse(&b2).unwrap();
        let m3 = parser::parse(&b3).unwrap();
        let m4 = parser::parse(&b4).unwrap();

        assert_eq!(m1.get_u64(tag::MSG_SEQ_NUM), Some(1));
        assert_eq!(m2.get_u64(tag::MSG_SEQ_NUM), Some(2));
        assert_eq!(m3.get_u64(tag::MSG_SEQ_NUM), Some(3));
        assert_eq!(m4.get_u64(tag::MSG_SEQ_NUM), Some(4));
    }
}
