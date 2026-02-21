/*
    ALICE-FIX
    Copyright (C) 2026 Moroya Sakamoto
*/

//! FIX wire-format parser.
//!
//! Parses a raw FIX message byte slice into a [`FixMessage`].
//!
//! ## Parsing Rules
//!
//! 1. Fields are delimited by the SOH character (`0x01`).
//! 2. The first field must be tag 8 (BeginString).
//! 3. The second field must be tag 9 (BodyLength); the declared length is validated.
//! 4. The last field must be tag 10 (Checksum); the checksum is validated.
//! 5. Tag 35 (MsgType) must be present among the body fields.
//! 6. All other fields are collected into [`FixMessage::fields`].

use crate::message::FixMessage;
use crate::tag;

/// SOH byte — the FIX field delimiter (ASCII 0x01).
pub const SOH: u8 = 0x01;

/// Errors that can occur while parsing a FIX message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// The input slice is empty.
    EmptyInput,
    /// Tag 8 (BeginString) is not the first field.
    MissingBeginString,
    /// Tag 9 (BodyLength) is not the second field.
    MissingBodyLength,
    /// Tag 10 (Checksum) is absent or not the final field.
    MissingChecksum,
    /// The computed checksum does not match the declared value.
    InvalidChecksum {
        /// Checksum declared in the message.
        expected: u8,
        /// Checksum computed over the message bytes.
        actual: u8,
    },
    /// A field does not contain the `=` separator.
    MalformedField(String),
    /// A tag number string cannot be parsed as a `u32`.
    InvalidTag(String),
}

impl core::fmt::Display for ParseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ParseError::EmptyInput => write!(f, "empty input"),
            ParseError::MissingBeginString => write!(f, "missing BeginString (tag 8)"),
            ParseError::MissingBodyLength => write!(f, "missing BodyLength (tag 9)"),
            ParseError::MissingChecksum => write!(f, "missing or misplaced Checksum (tag 10)"),
            ParseError::InvalidChecksum { expected, actual } => {
                write!(f, "invalid checksum: expected {expected:03}, actual {actual:03}")
            }
            ParseError::MalformedField(s) => write!(f, "malformed field: {s}"),
            ParseError::InvalidTag(s) => write!(f, "invalid tag number: {s}"),
        }
    }
}

/// Compute the FIX checksum over `bytes`.
///
/// The FIX checksum is the sum of all byte values, modulo 256.
#[inline(always)]
fn compute_checksum(bytes: &[u8]) -> u8 {
    let mut sum: u32 = 0;
    for &b in bytes {
        sum = sum.wrapping_add(b as u32);
    }
    (sum & 0xFF) as u8
}

/// Split a field string (e.g., `"49=ALICE"`) into `(tag_number, value)`.
#[inline(always)]
fn split_field(field: &str) -> Result<(u32, &str), ParseError> {
    let eq = field
        .find('=')
        .ok_or_else(|| ParseError::MalformedField(field.to_string()))?;
    let tag_str = &field[..eq];
    let value = &field[eq + 1..];
    let tag: u32 = tag_str
        .parse()
        .map_err(|_| ParseError::InvalidTag(tag_str.to_string()))?;
    Ok((tag, value))
}

/// Parse a raw FIX message byte slice into a [`FixMessage`].
///
/// Validates the BeginString, BodyLength, and Checksum fields.
/// All remaining fields are collected into the returned message.
pub fn parse(input: &[u8]) -> Result<FixMessage, ParseError> {
    if input.is_empty() {
        return Err(ParseError::EmptyInput);
    }

    // Split into SOH-delimited fields, dropping trailing empty tokens.
    let raw_fields: Vec<&str> = input
        .split(|&b| b == SOH)
        .filter(|f| !f.is_empty())
        .map(|f| {
            // We need &str from &[u8]. FIX messages are ASCII; treat as UTF-8.
            core::str::from_utf8(f).unwrap_or("")
        })
        .collect();

    if raw_fields.is_empty() {
        return Err(ParseError::EmptyInput);
    }

    // --- Field 0: must be tag 8 (BeginString) ---
    let (tag0, begin_string) = split_field(raw_fields[0])?;
    if tag0 != tag::BEGIN_STRING {
        return Err(ParseError::MissingBeginString);
    }
    let begin_string = begin_string.to_string();

    // --- Field 1: must be tag 9 (BodyLength) ---
    if raw_fields.len() < 2 {
        return Err(ParseError::MissingBodyLength);
    }
    let (tag1, body_len_str) = split_field(raw_fields[1])?;
    if tag1 != tag::BODY_LENGTH {
        return Err(ParseError::MissingBodyLength);
    }
    // Validate declared body length. The body starts after "8=X\x01" + "9=Y\x01".
    let declared_len: usize = body_len_str
        .parse()
        .map_err(|_| ParseError::MissingBodyLength)?;

    // Find the byte offset immediately after the tag-9 field (including its SOH).
    let tag8_field_len = raw_fields[0].len() + 1; // +1 for SOH
    let tag9_field_len = raw_fields[1].len() + 1;
    let body_start = tag8_field_len + tag9_field_len;

    // The checksum field ("10=XXX\x01") is always exactly 7 bytes.
    let checksum_field_len = 7_usize;
    let body_end = input.len().saturating_sub(checksum_field_len);

    if body_end < body_start || (body_end - body_start) != declared_len {
        // Body length mismatch — treat as missing/invalid.
        return Err(ParseError::MissingBodyLength);
    }

    // --- Last field: must be tag 10 (Checksum) ---
    let last = *raw_fields.last().ok_or(ParseError::MissingChecksum)?;
    let (tag_last, chk_str) = split_field(last)?;
    if tag_last != tag::CHECKSUM {
        return Err(ParseError::MissingChecksum);
    }

    // Validate checksum: sum of all bytes up to (but not including) "10=" field.
    let chk_offset = input
        .len()
        .checked_sub(checksum_field_len)
        .ok_or(ParseError::MissingChecksum)?;
    let actual_chk = compute_checksum(&input[..chk_offset]);
    let expected_chk: u8 = chk_str
        .parse::<u16>()
        .map(|v| (v & 0xFF) as u8)
        .map_err(|_| ParseError::InvalidChecksum {
            expected: 0,
            actual: actual_chk,
        })?;

    if actual_chk != expected_chk {
        return Err(ParseError::InvalidChecksum {
            expected: expected_chk,
            actual: actual_chk,
        });
    }

    // --- Body fields (everything between tag 9 and tag 10) ---
    // Iterate over fields[2..last], extracting tag 35 and everything else.
    let body_field_count = raw_fields.len().saturating_sub(3); // exclude 8, 9, 10
    let mut msg_type = String::new();
    let mut fields = std::collections::BTreeMap::new();

    for field_str in raw_fields.iter().skip(2).take(body_field_count + 1) {
        // The +1 accounts for body fields up to (but not including) tag 10.
        let (t, v) = split_field(field_str)?;
        if t == tag::MSG_TYPE {
            msg_type = v.to_string();
        } else if t != tag::CHECKSUM {
            fields.insert(t, v.to_string());
        }
    }

    Ok(FixMessage {
        begin_string,
        msg_type,
        fields,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::FixBuilder;
    use crate::tag;

    /// Build a minimal valid FIX message byte vector for testing.
    fn make_valid_message() -> Vec<u8> {
        FixBuilder::new("FIX.4.4", "0")
            .field(tag::SENDER_COMP_ID, "ALICE")
            .field(tag::TARGET_COMP_ID, "BROKER")
            .field(tag::MSG_SEQ_NUM, "1")
            .field(tag::SENDING_TIME, "20260101-00:00:00")
            .build()
    }

    #[test]
    fn test_parse_valid_message() {
        let bytes = make_valid_message();
        let msg = parse(&bytes).expect("valid message should parse");
        assert_eq!(msg.begin_string, "FIX.4.4");
        assert_eq!(msg.msg_type, "0");
        assert_eq!(msg.get(tag::SENDER_COMP_ID), Some("ALICE"));
        assert_eq!(msg.get(tag::TARGET_COMP_ID), Some("BROKER"));
        assert_eq!(msg.get_u64(tag::MSG_SEQ_NUM), Some(1));
    }

    #[test]
    fn test_parse_empty_input() {
        let result = parse(&[]);
        assert_eq!(result, Err(ParseError::EmptyInput));
    }

    #[test]
    fn test_parse_invalid_checksum() {
        let mut bytes = make_valid_message();
        // Corrupt the checksum digit: the last 4 bytes before the final SOH are "XXX"
        // The checksum field is "10=XXX\x01" (7 bytes at the end).
        let len = bytes.len();
        // Flip one digit of the checksum value.
        bytes[len - 4] = if bytes[len - 4] == b'0' { b'1' } else { b'0' };
        let result = parse(&bytes);
        assert!(matches!(result, Err(ParseError::InvalidChecksum { .. })));
    }

    #[test]
    fn test_parse_missing_begin_string() {
        // Construct a message that starts with tag 9 instead of tag 8.
        let bad: Vec<u8> = b"9=5\x0135=0\x0110=100\x01".to_vec();
        let result = parse(&bad);
        assert_eq!(result, Err(ParseError::MissingBeginString));
    }

    #[test]
    fn test_parse_all_tags_present() {
        let bytes = FixBuilder::new("FIX.4.4", "D")
            .field(tag::SENDER_COMP_ID, "ME")
            .field(tag::TARGET_COMP_ID, "YOU")
            .field(tag::MSG_SEQ_NUM, "7")
            .field(tag::CL_ORD_ID, "ORD-001")
            .field(tag::SYMBOL, "BTCUSD")
            .field(tag::SIDE, "1")
            .field(tag::ORD_TYPE, "2")
            .field(tag::PRICE, "50000")
            .field(tag::ORDER_QTY, "10")
            .build();

        let msg = parse(&bytes).expect("should parse");
        assert_eq!(msg.msg_type, "D");
        assert_eq!(msg.get(tag::CL_ORD_ID), Some("ORD-001"));
        assert_eq!(msg.get(tag::SYMBOL), Some("BTCUSD"));
        assert_eq!(msg.get_i64(tag::PRICE), Some(50000));
        assert_eq!(msg.get_u64(tag::ORDER_QTY), Some(10));
    }
}
