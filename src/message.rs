/*
    ALICE-FIX
    Copyright (C) 2026 Moroya Sakamoto
*/

//! FIX message representation.
//!
//! A [`FixMessage`] holds the parsed contents of a single FIX frame.
//! Tags are stored in a [`BTreeMap`] for deterministic, ascending iteration
//! order, which simplifies testing and logging.
//!
//! The structural tags 8 (BeginString), 9 (BodyLength), and 10 (Checksum)
//! are not stored in [`FixMessage::fields`]; they are either captured in
//! dedicated fields or reconstructed at serialisation time by [`crate::builder`].

use std::collections::BTreeMap;

/// A parsed FIX message.
///
/// Structural framing tags (8, 9, 10) are excluded from [`fields`]; they are
/// handled by the parser and builder layers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixMessage {
    /// FIX version string from tag 8 (e.g., "FIX.4.4" or "FIXT.1.1").
    pub begin_string: String,
    /// Message type from tag 35 (e.g., "D" for NewOrderSingle, "8" for ExecutionReport).
    pub msg_type: String,
    /// All non-structural tag/value pairs keyed by tag number.
    pub fields: BTreeMap<u32, String>,
}

impl FixMessage {
    /// Create a new, empty FIX message with the given version and message type.
    #[inline(always)]
    pub fn new(begin_string: &str, msg_type: &str) -> Self {
        Self {
            begin_string: begin_string.to_string(),
            msg_type: msg_type.to_string(),
            fields: BTreeMap::new(),
        }
    }

    /// Set (or overwrite) a tag/value field.
    ///
    /// Returns `&mut self` for method chaining.
    #[inline(always)]
    pub fn set(&mut self, tag: u32, value: &str) -> &mut Self {
        self.fields.insert(tag, value.to_string());
        self
    }

    /// Retrieve the string value for a tag, or `None` if absent.
    #[inline(always)]
    pub fn get(&self, tag: u32) -> Option<&str> {
        self.fields.get(&tag).map(String::as_str)
    }

    /// Parse the value of a tag as an `i64`.
    ///
    /// Returns `None` if the tag is absent or the value cannot be parsed.
    #[inline(always)]
    pub fn get_i64(&self, tag: u32) -> Option<i64> {
        self.fields.get(&tag)?.parse().ok()
    }

    /// Parse the value of a tag as a `u64`.
    ///
    /// Returns `None` if the tag is absent or the value cannot be parsed.
    #[inline(always)]
    pub fn get_u64(&self, tag: u32) -> Option<u64> {
        self.fields.get(&tag)?.parse().ok()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tag;

    #[test]
    fn test_new_message() {
        let msg = FixMessage::new("FIX.4.4", "D");
        assert_eq!(msg.begin_string, "FIX.4.4");
        assert_eq!(msg.msg_type, "D");
        assert!(msg.fields.is_empty());
    }

    #[test]
    fn test_set_and_get() {
        let mut msg = FixMessage::new("FIX.4.4", "D");
        msg.set(tag::SENDER_COMP_ID, "ALICE");
        msg.set(tag::TARGET_COMP_ID, "BROKER");

        assert_eq!(msg.get(tag::SENDER_COMP_ID), Some("ALICE"));
        assert_eq!(msg.get(tag::TARGET_COMP_ID), Some("BROKER"));
    }

    #[test]
    fn test_set_overwrites_existing_value() {
        let mut msg = FixMessage::new("FIX.4.4", "D");
        msg.set(tag::SYMBOL, "BTCUSD");
        msg.set(tag::SYMBOL, "ETHUSD");
        assert_eq!(msg.get(tag::SYMBOL), Some("ETHUSD"));
    }

    #[test]
    fn test_get_i64() {
        let mut msg = FixMessage::new("FIX.4.4", "D");
        msg.set(tag::MSG_SEQ_NUM, "42");
        assert_eq!(msg.get_i64(tag::MSG_SEQ_NUM), Some(42));
    }

    #[test]
    fn test_get_i64_negative() {
        let mut msg = FixMessage::new("FIX.4.4", "D");
        msg.set(tag::PRICE, "-1000");
        assert_eq!(msg.get_i64(tag::PRICE), Some(-1000));
    }

    #[test]
    fn test_get_u64() {
        let mut msg = FixMessage::new("FIX.4.4", "D");
        msg.set(tag::ORDER_QTY, "100");
        assert_eq!(msg.get_u64(tag::ORDER_QTY), Some(100));
    }

    #[test]
    fn test_get_missing_tag() {
        let msg = FixMessage::new("FIX.4.4", "D");
        assert_eq!(msg.get(tag::SYMBOL), None);
        assert_eq!(msg.get_i64(tag::PRICE), None);
        assert_eq!(msg.get_u64(tag::ORDER_QTY), None);
    }

    #[test]
    fn test_get_i64_non_numeric_returns_none() {
        let mut msg = FixMessage::new("FIX.4.4", "D");
        msg.set(tag::PRICE, "not_a_number");
        assert_eq!(msg.get_i64(tag::PRICE), None);
    }

    #[test]
    fn test_method_chaining() {
        let mut msg = FixMessage::new("FIX.4.4", "D");
        msg.set(tag::SENDER_COMP_ID, "A")
            .set(tag::TARGET_COMP_ID, "B")
            .set(tag::SYMBOL, "BTCUSD");

        assert_eq!(msg.get(tag::SENDER_COMP_ID), Some("A"));
        assert_eq!(msg.get(tag::TARGET_COMP_ID), Some("B"));
        assert_eq!(msg.get(tag::SYMBOL), Some("BTCUSD"));
    }
}
