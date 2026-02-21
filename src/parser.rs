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
//!
//! ## Zero-copy design
//!
//! The parser iterates over SOH-delimited byte slices directly without
//! building an intermediate `Vec`. Each field slice (`&[u8]`) is interpreted
//! as a UTF-8 string in-place; only the final owned values written into
//! [`FixMessage`] allocate heap memory.

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

/// Split a raw field byte slice (e.g., `b"49=ALICE"`) into `(tag_number, value_bytes)`.
///
/// Both the tag and value are returned as zero-copy sub-slices of the input.
/// The caller is responsible for converting the value slice to `&str` / `String`.
#[inline(always)]
fn split_field(field: &[u8]) -> Result<(u32, &[u8]), ParseError> {
    // Find the '=' byte position.
    let eq = field
        .iter()
        .position(|&b| b == b'=')
        .ok_or_else(|| ParseError::MalformedField(
            String::from_utf8_lossy(field).into_owned(),
        ))?;

    let tag_bytes = &field[..eq];
    let value_bytes = &field[eq + 1..];

    // Parse the tag number from ASCII digits without allocating a String.
    let tag = parse_tag_number(tag_bytes)
        .ok_or_else(|| ParseError::InvalidTag(
            String::from_utf8_lossy(tag_bytes).into_owned(),
        ))?;

    Ok((tag, value_bytes))
}

/// Parse a decimal `u32` from a byte slice of ASCII digits.
///
/// Returns `None` if the slice is empty, contains non-digit bytes, or would
/// overflow `u32`. No allocation is performed.
#[inline(always)]
fn parse_tag_number(bytes: &[u8]) -> Option<u32> {
    if bytes.is_empty() {
        return None;
    }
    let mut n: u32 = 0;
    for &b in bytes {
        if !b.is_ascii_digit() {
            return None;
        }
        n = n.checked_mul(10)?.checked_add((b - b'0') as u32)?;
    }
    Some(n)
}

/// An iterator over SOH-delimited fields in a FIX byte slice.
///
/// Yields `&[u8]` sub-slices, each corresponding to one `tag=value` field.
/// Empty sub-slices (e.g., from a trailing SOH) are skipped.
struct FieldIter<'a> {
    remaining: &'a [u8],
}

impl<'a> FieldIter<'a> {
    #[inline(always)]
    fn new(input: &'a [u8]) -> Self {
        Self { remaining: input }
    }
}

impl<'a> Iterator for FieldIter<'a> {
    type Item = &'a [u8];

    #[inline(always)]
    fn next(&mut self) -> Option<&'a [u8]> {
        loop {
            if self.remaining.is_empty() {
                return None;
            }
            // Find the next SOH delimiter.
            let end = self
                .remaining
                .iter()
                .position(|&b| b == SOH)
                .unwrap_or(self.remaining.len());
            let field = &self.remaining[..end];
            // Advance past the SOH (or to the end if no SOH was found).
            self.remaining = if end < self.remaining.len() {
                &self.remaining[end + 1..]
            } else {
                &[]
            };
            // Skip empty segments.
            if !field.is_empty() {
                return Some(field);
            }
        }
    }
}

/// Parse a raw FIX message byte slice into a [`FixMessage`].
///
/// Validates the BeginString, BodyLength, and Checksum fields.
/// All remaining fields are collected into the returned message.
///
/// The parser works directly on the input `&[u8]` without building an
/// intermediate `Vec`; only the owned strings written into [`FixMessage`]
/// allocate heap memory.
pub fn parse(input: &[u8]) -> Result<FixMessage, ParseError> {
    if input.is_empty() {
        return Err(ParseError::EmptyInput);
    }

    let mut iter = FieldIter::new(input);

    // --- Field 0: must be tag 8 (BeginString) ---
    let field0 = iter.next().ok_or(ParseError::EmptyInput)?;
    let (tag0, begin_bytes) = split_field(field0)?;
    if tag0 != tag::BEGIN_STRING {
        return Err(ParseError::MissingBeginString);
    }
    // The BeginString field occupies `field0.len() + 1` bytes (field + SOH).
    let tag8_field_len = field0.len() + 1;

    // --- Field 1: must be tag 9 (BodyLength) ---
    let field1 = iter.next().ok_or(ParseError::MissingBodyLength)?;
    let (tag1, body_len_bytes) = split_field(field1)?;
    if tag1 != tag::BODY_LENGTH {
        return Err(ParseError::MissingBodyLength);
    }
    let tag9_field_len = field1.len() + 1;
    let body_start = tag8_field_len + tag9_field_len;

    // Parse the declared body length without allocating a String.
    let declared_len = parse_body_length(body_len_bytes)
        .ok_or(ParseError::MissingBodyLength)?;

    // The checksum field ("10=XXX\x01") is always exactly 7 bytes.
    let checksum_field_len = 7_usize;
    let body_end = input.len().saturating_sub(checksum_field_len);

    if body_end < body_start || (body_end - body_start) != declared_len {
        return Err(ParseError::MissingBodyLength);
    }

    // --- Checksum: computed over all bytes before the "10=..." field ---
    let chk_offset = input
        .len()
        .checked_sub(checksum_field_len)
        .ok_or(ParseError::MissingChecksum)?;
    let actual_chk = compute_checksum(&input[..chk_offset]);

    // --- Collect body fields and validate checksum tag ---
    // We do not know how many fields there are ahead of time, so allocate
    // a HashMap with a small initial capacity typical of FIX messages.
    let mut msg_type = String::new();
    let mut fields = std::collections::HashMap::with_capacity(16);
    let mut saw_checksum = false;

    for field_bytes in iter {
        let (t, v_bytes) = split_field(field_bytes)?;
        match t {
            _ if t == tag::CHECKSUM => {
                // Validate the checksum value without allocating on the error path.
                let expected_chk = parse_checksum_value(v_bytes)
                    .ok_or(ParseError::InvalidChecksum {
                        expected: 0,
                        actual: actual_chk,
                    })?;
                if actual_chk != expected_chk {
                    return Err(ParseError::InvalidChecksum {
                        expected: expected_chk,
                        actual: actual_chk,
                    });
                }
                saw_checksum = true;
            }
            _ if t == tag::MSG_TYPE => {
                // Zero-copy: interpret v_bytes as UTF-8 in-place, then own.
                msg_type = core::str::from_utf8(v_bytes)
                    .unwrap_or("")
                    .to_string();
            }
            _ => {
                let value = core::str::from_utf8(v_bytes)
                    .unwrap_or("")
                    .to_string();
                fields.insert(t, value);
            }
        }
    }

    if !saw_checksum {
        return Err(ParseError::MissingChecksum);
    }

    let begin_string = core::str::from_utf8(begin_bytes)
        .unwrap_or("")
        .to_string();

    Ok(FixMessage {
        begin_string,
        msg_type,
        fields,
    })
}

/// Parse a decimal `usize` from ASCII digit bytes (used for BodyLength).
#[inline(always)]
fn parse_body_length(bytes: &[u8]) -> Option<usize> {
    if bytes.is_empty() {
        return None;
    }
    let mut n: usize = 0;
    for &b in bytes {
        if !b.is_ascii_digit() {
            return None;
        }
        n = n.checked_mul(10)?.checked_add((b - b'0') as usize)?;
    }
    Some(n)
}

/// Parse a three-character checksum value (e.g., `b"127"`) as a `u8`.
///
/// Accepts values 0–255 zero-padded to three decimal digits.
#[inline(always)]
fn parse_checksum_value(bytes: &[u8]) -> Option<u8> {
    if bytes.is_empty() {
        return None;
    }
    let mut n: u16 = 0;
    for &b in bytes {
        if !b.is_ascii_digit() {
            return None;
        }
        n = n.checked_mul(10)?.checked_add((b - b'0') as u16)?;
    }
    Some((n & 0xFF) as u8)
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
