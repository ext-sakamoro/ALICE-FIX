/*
    ALICE-FIX
    Copyright (C) 2026 Moroya Sakamoto
*/

//! FIX message builder / serializer.
//!
//! [`FixBuilder`] accumulates tag/value pairs and serializes them to a
//! complete FIX wire-format byte vector, including the auto-computed
//! BodyLength (tag 9) and Checksum (tag 10).
//!
//! ## Build Flow
//!
//! 1. Collect all user-supplied fields as `"tag=value\x01"` segments.
//! 2. Prepend `"35=<msg_type>\x01"` so it appears first in the body.
//! 3. Compute the body length (bytes of the body, including tag 35).
//! 4. Prepend `"8=<begin_string>\x01"` and `"9=<body_length>\x01"`.
//! 5. Compute the checksum over all preceding bytes, modulo 256.
//! 6. Append `"10=<checksum_3digits>\x01"`.

use crate::parser::SOH;
use crate::tag;

/// FIX message serializer.
///
/// Fields are appended in the order [`field`] is called. Tag 8 (BeginString),
/// tag 9 (BodyLength), tag 35 (MsgType), and tag 10 (Checksum) are managed
/// automatically.
pub struct FixBuilder {
    begin_string: String,
    msg_type: String,
    /// User-supplied body fields, in insertion order.
    fields: Vec<(u32, String)>,
}

impl FixBuilder {
    /// Create a new builder for a message of the given FIX version and type.
    #[inline(always)]
    pub fn new(begin_string: &str, msg_type: &str) -> Self {
        Self {
            begin_string: begin_string.to_string(),
            msg_type: msg_type.to_string(),
            fields: Vec::new(),
        }
    }

    /// Append a string tag/value pair to the message body.
    ///
    /// Returns `&mut self` for method chaining.
    #[inline(always)]
    pub fn field(&mut self, tag: u32, value: &str) -> &mut Self {
        self.fields.push((tag, value.to_string()));
        self
    }

    /// Append an `i64` value for the given tag.
    ///
    /// Returns `&mut self` for method chaining.
    #[inline(always)]
    pub fn field_i64(&mut self, tag: u32, value: i64) -> &mut Self {
        self.fields.push((tag, value.to_string()));
        self
    }

    /// Append a `u64` value for the given tag.
    ///
    /// Returns `&mut self` for method chaining.
    #[inline(always)]
    pub fn field_u64(&mut self, tag: u32, value: u64) -> &mut Self {
        self.fields.push((tag, value.to_string()));
        self
    }

    /// Serialize the message to FIX wire format.
    ///
    /// The returned bytes include the leading "8=..." and trailing "10=..."
    /// fields with correctly computed BodyLength and Checksum.
    pub fn build(&self) -> Vec<u8> {
        // Build the body: "35=<msg_type>\x01" + user fields.
        let mut body: Vec<u8> = Vec::new();
        append_field(&mut body, tag::MSG_TYPE, &self.msg_type);
        for (t, v) in &self.fields {
            append_field(&mut body, *t, v);
        }

        // Prefix: "8=<begin_string>\x01" + "9=<body_length>\x01"
        let mut prefix: Vec<u8> = Vec::new();
        append_field(&mut prefix, tag::BEGIN_STRING, &self.begin_string);
        append_field(&mut prefix, tag::BODY_LENGTH, &body.len().to_string());

        // Assemble everything before the checksum.
        let mut out: Vec<u8> = Vec::with_capacity(prefix.len() + body.len() + 7);
        out.extend_from_slice(&prefix);
        out.extend_from_slice(&body);

        // Compute checksum over all bytes so far.
        let chk = compute_checksum(&out);

        // Append "10=<chk>\x01" (checksum is always 3 digits, zero-padded).
        out.extend_from_slice(format!("10={chk:03}").as_bytes());
        out.push(SOH);

        out
    }
}

/// Append `"<tag>=<value>\x01"` to `buf`.
#[inline(always)]
fn append_field(buf: &mut Vec<u8>, tag: u32, value: &str) {
    buf.extend_from_slice(tag.to_string().as_bytes());
    buf.push(b'=');
    buf.extend_from_slice(value.as_bytes());
    buf.push(SOH);
}

/// Compute the FIX checksum: sum of all byte values, modulo 256.
#[inline(always)]
fn compute_checksum(bytes: &[u8]) -> u8 {
    let mut sum: u32 = 0;
    for &b in bytes {
        sum = sum.wrapping_add(b as u32);
    }
    (sum & 0xFF) as u8
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;
    use crate::tag;

    #[test]
    fn test_build_simple_message() {
        let bytes = FixBuilder::new("FIX.4.4", "0")
            .field(tag::SENDER_COMP_ID, "ALICE")
            .field(tag::TARGET_COMP_ID, "BROKER")
            .field(tag::MSG_SEQ_NUM, "1")
            .build();

        // Must start with BeginString.
        assert!(bytes.starts_with(b"8=FIX.4.4\x01"));
        // Must end with "10=XXX\x01".
        assert_eq!(bytes.last(), Some(&SOH));
        let end_str = core::str::from_utf8(&bytes[bytes.len() - 7..]).unwrap();
        assert!(end_str.starts_with("10="));
    }

    #[test]
    fn test_build_includes_msg_type() {
        let bytes = FixBuilder::new("FIX.4.4", "D")
            .field(tag::SENDER_COMP_ID, "X")
            .build();
        let msg = parser::parse(&bytes).unwrap();
        assert_eq!(msg.msg_type, "D");
    }

    #[test]
    fn test_roundtrip_build_parse() {
        let bytes = FixBuilder::new("FIX.4.4", "D")
            .field(tag::SENDER_COMP_ID, "ALICE")
            .field(tag::TARGET_COMP_ID, "BROKER")
            .field(tag::MSG_SEQ_NUM, "5")
            .field(tag::CL_ORD_ID, "ORD-42")
            .field(tag::SYMBOL, "BTCUSD")
            .field(tag::SIDE, "1")
            .field(tag::ORD_TYPE, "2")
            .field(tag::PRICE, "50000")
            .field(tag::ORDER_QTY, "10")
            .build();

        let msg = parser::parse(&bytes).expect("round-trip parse should succeed");

        assert_eq!(msg.begin_string, "FIX.4.4");
        assert_eq!(msg.msg_type, "D");
        assert_eq!(msg.get(tag::SENDER_COMP_ID), Some("ALICE"));
        assert_eq!(msg.get(tag::TARGET_COMP_ID), Some("BROKER"));
        assert_eq!(msg.get_u64(tag::MSG_SEQ_NUM), Some(5));
        assert_eq!(msg.get(tag::CL_ORD_ID), Some("ORD-42"));
        assert_eq!(msg.get(tag::SYMBOL), Some("BTCUSD"));
        assert_eq!(msg.get(tag::SIDE), Some("1"));
        assert_eq!(msg.get(tag::ORD_TYPE), Some("2"));
        assert_eq!(msg.get_i64(tag::PRICE), Some(50000));
        assert_eq!(msg.get_u64(tag::ORDER_QTY), Some(10));
    }

    #[test]
    fn test_field_i64() {
        let bytes = FixBuilder::new("FIX.4.4", "D")
            .field(tag::SENDER_COMP_ID, "X")
            .field_i64(tag::PRICE, -100)
            .build();
        let msg = parser::parse(&bytes).unwrap();
        assert_eq!(msg.get_i64(tag::PRICE), Some(-100));
    }

    #[test]
    fn test_checksum_is_three_digits() {
        let bytes = FixBuilder::new("FIX.4.4", "0")
            .field(tag::SENDER_COMP_ID, "A")
            .build();
        // The last 7 bytes are "10=XXX\x01"
        let chk_field = core::str::from_utf8(&bytes[bytes.len() - 7..]).unwrap();
        assert_eq!(&chk_field[..3], "10=");
        assert_eq!(chk_field.len(), 7);
        let digits = &chk_field[3..6];
        assert!(digits.chars().all(|c| c.is_ascii_digit()));
    }
}
