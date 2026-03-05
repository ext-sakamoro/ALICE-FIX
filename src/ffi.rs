/*
    ALICE-FIX  FFI
    Copyright (C) 2026 Moroya Sakamoto
*/

//! C-ABI FFI — 33 `extern "C"` functions with `af_fix_*` prefix.
//!
//! Provides zero-overhead access to ALICE-FIX from C, C++, C#, and any
//! language that can call `extern "C"` functions.

#![allow(clippy::missing_safety_doc)]

use std::ffi::{c_char, CStr, CString};
use std::sync::OnceLock;

use crate::builder::FixBuilder;
use crate::convert;
use crate::message::FixMessage;
use crate::parser;
use crate::session::{FixSession, SessionState};
use alice_ledger::{OrderType, Side, TimeInForce};

// -----------------------------------------------------------------------
// Memory management
// -----------------------------------------------------------------------

/// Free a string returned by ALICE-FIX FFI functions.
#[no_mangle]
pub unsafe extern "C" fn af_fix_string_free(s: *mut c_char) {
    if !s.is_null() {
        drop(CString::from_raw(s));
    }
}

/// Free a byte buffer returned by build/logon/logout/heartbeat functions.
///
/// `len` must match the value written to `out_len` by the allocating call.
#[no_mangle]
pub unsafe extern "C" fn af_fix_bytes_free(ptr: *mut u8, len: i32) {
    if !ptr.is_null() && len > 0 {
        let slice = std::slice::from_raw_parts_mut(ptr, len as usize);
        let _ = Box::from_raw(slice as *mut [u8]);
    }
}

// -----------------------------------------------------------------------
// FixMessage
// -----------------------------------------------------------------------

/// Create a new empty FIX message.
#[no_mangle]
pub unsafe extern "C" fn af_fix_message_new(
    begin_string: *const c_char,
    msg_type: *const c_char,
) -> *mut FixMessage {
    if begin_string.is_null() || msg_type.is_null() {
        return std::ptr::null_mut();
    }
    let bs = CStr::from_ptr(begin_string).to_str().unwrap_or("");
    let mt = CStr::from_ptr(msg_type).to_str().unwrap_or("");
    Box::into_raw(Box::new(FixMessage::new(bs, mt)))
}

/// Free a FixMessage.
#[no_mangle]
pub unsafe extern "C" fn af_fix_message_free(msg: *mut FixMessage) {
    if !msg.is_null() {
        drop(Box::from_raw(msg));
    }
}

/// Set a tag/value field on a message.
#[no_mangle]
pub unsafe extern "C" fn af_fix_message_set(msg: *mut FixMessage, tag: u32, value: *const c_char) {
    if msg.is_null() || value.is_null() {
        return;
    }
    let msg = &mut *msg;
    let v = CStr::from_ptr(value).to_str().unwrap_or("");
    msg.set(tag, v);
}

/// Get a tag value as a newly allocated C string. Returns null if absent.
///
/// Caller must free the returned pointer with `af_fix_string_free`.
#[no_mangle]
pub unsafe extern "C" fn af_fix_message_get(msg: *const FixMessage, tag: u32) -> *mut c_char {
    if msg.is_null() {
        return std::ptr::null_mut();
    }
    let msg = &*msg;
    match msg.get(tag) {
        Some(s) => CString::new(s)
            .map(CString::into_raw)
            .unwrap_or(std::ptr::null_mut()),
        None => std::ptr::null_mut(),
    }
}

/// Parse a tag value as i64. Returns 1 on success, 0 on failure.
#[no_mangle]
pub unsafe extern "C" fn af_fix_message_get_i64(
    msg: *const FixMessage,
    tag: u32,
    out: *mut i64,
) -> u8 {
    if msg.is_null() || out.is_null() {
        return 0;
    }
    match (*msg).get_i64(tag) {
        Some(v) => {
            *out = v;
            1
        }
        None => 0,
    }
}

/// Parse a tag value as u64. Returns 1 on success, 0 on failure.
#[no_mangle]
pub unsafe extern "C" fn af_fix_message_get_u64(
    msg: *const FixMessage,
    tag: u32,
    out: *mut u64,
) -> u8 {
    if msg.is_null() || out.is_null() {
        return 0;
    }
    match (*msg).get_u64(tag) {
        Some(v) => {
            *out = v;
            1
        }
        None => 0,
    }
}

/// Get the BeginString (tag 8) as a newly allocated C string.
///
/// Caller must free with `af_fix_string_free`.
#[no_mangle]
pub unsafe extern "C" fn af_fix_message_begin_string(msg: *const FixMessage) -> *mut c_char {
    if msg.is_null() {
        return std::ptr::null_mut();
    }
    CString::new((*msg).begin_string.as_str())
        .map(CString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}

/// Get the MsgType (tag 35) as a newly allocated C string.
///
/// Caller must free with `af_fix_string_free`.
#[no_mangle]
pub unsafe extern "C" fn af_fix_message_msg_type(msg: *const FixMessage) -> *mut c_char {
    if msg.is_null() {
        return std::ptr::null_mut();
    }
    CString::new((*msg).msg_type.as_str())
        .map(CString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}

// -----------------------------------------------------------------------
// FixBuilder
// -----------------------------------------------------------------------

/// Create a new FIX message builder.
#[no_mangle]
pub unsafe extern "C" fn af_fix_builder_new(
    begin_string: *const c_char,
    msg_type: *const c_char,
) -> *mut FixBuilder {
    if begin_string.is_null() || msg_type.is_null() {
        return std::ptr::null_mut();
    }
    let bs = CStr::from_ptr(begin_string).to_str().unwrap_or("");
    let mt = CStr::from_ptr(msg_type).to_str().unwrap_or("");
    Box::into_raw(Box::new(FixBuilder::new(bs, mt)))
}

/// Free a FixBuilder.
#[no_mangle]
pub unsafe extern "C" fn af_fix_builder_free(builder: *mut FixBuilder) {
    if !builder.is_null() {
        drop(Box::from_raw(builder));
    }
}

/// Append a string field to the builder.
#[no_mangle]
pub unsafe extern "C" fn af_fix_builder_field(
    builder: *mut FixBuilder,
    tag: u32,
    value: *const c_char,
) {
    if builder.is_null() || value.is_null() {
        return;
    }
    let v = CStr::from_ptr(value).to_str().unwrap_or("");
    (*builder).field(tag, v);
}

/// Append an i64 field to the builder.
#[no_mangle]
pub unsafe extern "C" fn af_fix_builder_field_i64(builder: *mut FixBuilder, tag: u32, value: i64) {
    if builder.is_null() {
        return;
    }
    (*builder).field_i64(tag, value);
}

/// Append a u64 field to the builder.
#[no_mangle]
pub unsafe extern "C" fn af_fix_builder_field_u64(builder: *mut FixBuilder, tag: u32, value: u64) {
    if builder.is_null() {
        return;
    }
    (*builder).field_u64(tag, value);
}

/// Serialize the message to FIX wire format.
///
/// Writes the byte count to `*out_len`. Returns an owned buffer that the
/// caller must free with `af_fix_bytes_free(ptr, *out_len)`.
#[no_mangle]
pub unsafe extern "C" fn af_fix_builder_build(
    builder: *const FixBuilder,
    out_len: *mut i32,
) -> *mut u8 {
    if builder.is_null() || out_len.is_null() {
        return std::ptr::null_mut();
    }
    let bytes = (*builder).build();
    let boxed = bytes.into_boxed_slice();
    let len = boxed.len();
    let ptr = Box::into_raw(boxed) as *mut u8;
    *out_len = len as i32;
    ptr
}

// -----------------------------------------------------------------------
// Parser
// -----------------------------------------------------------------------

/// Parse FIX wire bytes into a FixMessage. Returns null on parse error.
///
/// Caller must free the returned message with `af_fix_message_free`.
#[no_mangle]
pub unsafe extern "C" fn af_fix_parse(input: *const u8, len: i32) -> *mut FixMessage {
    if input.is_null() || len <= 0 {
        return std::ptr::null_mut();
    }
    let slice = std::slice::from_raw_parts(input, len as usize);
    match parser::parse(slice) {
        Ok(msg) => Box::into_raw(Box::new(msg)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Compute the FIX checksum (sum of bytes mod 256).
#[no_mangle]
pub unsafe extern "C" fn af_fix_checksum(bytes: *const u8, len: i32) -> u8 {
    if bytes.is_null() || len <= 0 {
        return 0;
    }
    let slice = std::slice::from_raw_parts(bytes, len as usize);
    let mut sum: u32 = 0;
    for &b in slice {
        sum = sum.wrapping_add(b as u32);
    }
    (sum & 0xFF) as u8
}

// -----------------------------------------------------------------------
// FixSession
// -----------------------------------------------------------------------

/// Create a new FIX session in Disconnected state.
#[no_mangle]
pub unsafe extern "C" fn af_fix_session_new(
    sender: *const c_char,
    target: *const c_char,
    begin_string: *const c_char,
) -> *mut FixSession {
    if sender.is_null() || target.is_null() || begin_string.is_null() {
        return std::ptr::null_mut();
    }
    let s = CStr::from_ptr(sender).to_str().unwrap_or("");
    let t = CStr::from_ptr(target).to_str().unwrap_or("");
    let bs = CStr::from_ptr(begin_string).to_str().unwrap_or("");
    Box::into_raw(Box::new(FixSession::new(s, t, bs)))
}

/// Free a FixSession.
#[no_mangle]
pub unsafe extern "C" fn af_fix_session_free(session: *mut FixSession) {
    if !session.is_null() {
        drop(Box::from_raw(session));
    }
}

/// Get session state: 0=Disconnected, 1=LogonSent, 2=Active, 3=LogoutSent.
#[no_mangle]
pub unsafe extern "C" fn af_fix_session_state(session: *const FixSession) -> u8 {
    if session.is_null() {
        return 0;
    }
    match *(*session).state() {
        SessionState::Disconnected => 0,
        SessionState::LogonSent => 1,
        SessionState::Active => 2,
        SessionState::LogoutSent => 3,
    }
}

/// Increment and return the next outgoing sequence number.
#[no_mangle]
pub unsafe extern "C" fn af_fix_session_next_outgoing_seq(session: *mut FixSession) -> u64 {
    if session.is_null() {
        return 0;
    }
    (*session).next_outgoing_seq()
}

/// Validate an incoming sequence number. Returns 1 if valid, 0 if gap/duplicate.
#[no_mangle]
pub unsafe extern "C" fn af_fix_session_validate_incoming_seq(
    session: *mut FixSession,
    seq: u64,
) -> u8 {
    if session.is_null() {
        return 0;
    }
    if (*session).validate_incoming_seq(seq) {
        1
    } else {
        0
    }
}

/// Build a Logon message and transition to LogonSent state.
///
/// Caller must free with `af_fix_bytes_free(ptr, *out_len)`.
#[no_mangle]
pub unsafe extern "C" fn af_fix_session_build_logon(
    session: *mut FixSession,
    out_len: *mut i32,
) -> *mut u8 {
    if session.is_null() || out_len.is_null() {
        return std::ptr::null_mut();
    }
    let bytes = (*session).build_logon();
    let boxed = bytes.into_boxed_slice();
    let len = boxed.len();
    let ptr = Box::into_raw(boxed) as *mut u8;
    *out_len = len as i32;
    ptr
}

/// Build a Logout message and transition to LogoutSent state.
///
/// Caller must free with `af_fix_bytes_free(ptr, *out_len)`.
#[no_mangle]
pub unsafe extern "C" fn af_fix_session_build_logout(
    session: *mut FixSession,
    out_len: *mut i32,
) -> *mut u8 {
    if session.is_null() || out_len.is_null() {
        return std::ptr::null_mut();
    }
    let bytes = (*session).build_logout();
    let boxed = bytes.into_boxed_slice();
    let len = boxed.len();
    let ptr = Box::into_raw(boxed) as *mut u8;
    *out_len = len as i32;
    ptr
}

/// Build a Heartbeat message (state unchanged).
///
/// Caller must free with `af_fix_bytes_free(ptr, *out_len)`.
#[no_mangle]
pub unsafe extern "C" fn af_fix_session_build_heartbeat(
    session: *mut FixSession,
    out_len: *mut i32,
) -> *mut u8 {
    if session.is_null() || out_len.is_null() {
        return std::ptr::null_mut();
    }
    let bytes = (*session).build_heartbeat();
    let boxed = bytes.into_boxed_slice();
    let len = boxed.len();
    let ptr = Box::into_raw(boxed) as *mut u8;
    *out_len = len as i32;
    ptr
}

// -----------------------------------------------------------------------
// Convert — FIX ↔ ALICE enum mappings
// -----------------------------------------------------------------------

/// Convert ALICE Side (0=Bid, 1=Ask) to FIX Side string. Returns null if invalid.
///
/// Returned pointer is static and must NOT be freed.
#[no_mangle]
pub extern "C" fn af_fix_side_to_fix(side: u8) -> *const c_char {
    match side {
        0 => c"1".as_ptr(),
        1 => c"2".as_ptr(),
        _ => std::ptr::null(),
    }
}

/// Convert FIX Side string to ALICE Side. Returns 0=Bid, 1=Ask, -1=invalid.
#[no_mangle]
pub unsafe extern "C" fn af_fix_side_from_fix(fix_side: *const c_char) -> i8 {
    if fix_side.is_null() {
        return -1;
    }
    let s = CStr::from_ptr(fix_side).to_str().unwrap_or("");
    match convert::fix_side_to_alice(s) {
        Some(Side::Bid) => 0,
        Some(Side::Ask) => 1,
        None => -1,
    }
}

/// Convert ALICE OrderType (0=Market, 1=Limit) to FIX OrdType string.
///
/// Returned pointer is static and must NOT be freed.
#[no_mangle]
pub extern "C" fn af_fix_ord_type_to_fix(ord_type: u8) -> *const c_char {
    match ord_type {
        0 => c"1".as_ptr(),
        1 => c"2".as_ptr(),
        _ => std::ptr::null(),
    }
}

/// Convert FIX OrdType string to ALICE OrderType. Returns 0=Market, 1=Limit, -1=invalid.
#[no_mangle]
pub unsafe extern "C" fn af_fix_ord_type_from_fix(fix_type: *const c_char) -> i8 {
    if fix_type.is_null() {
        return -1;
    }
    let s = CStr::from_ptr(fix_type).to_str().unwrap_or("");
    match convert::fix_ord_type_to_alice(s) {
        Some(OrderType::Market) => 0,
        Some(OrderType::Limit) => 1,
        _ => -1,
    }
}

/// Convert ALICE TimeInForce (0=GTC, 1=IOC, 2=FOK, 3=GTD) to FIX TIF string.
///
/// Returned pointer is static and must NOT be freed.
#[no_mangle]
pub extern "C" fn af_fix_tif_to_fix(tif: u8) -> *const c_char {
    match tif {
        0 => c"1".as_ptr(),
        1 => c"3".as_ptr(),
        2 => c"4".as_ptr(),
        3 => c"6".as_ptr(),
        _ => std::ptr::null(),
    }
}

/// Convert FIX TimeInForce string to ALICE TIF. Returns 0=GTC, 1=IOC, 2=FOK, -1=invalid.
///
/// Note: FIX "0" (Day) and "6" (GTD) both map to GTC (0) for simplicity.
#[no_mangle]
pub unsafe extern "C" fn af_fix_tif_from_fix(fix_tif: *const c_char) -> i8 {
    if fix_tif.is_null() {
        return -1;
    }
    let s = CStr::from_ptr(fix_tif).to_str().unwrap_or("");
    match convert::fix_tif_to_alice(s) {
        Some(TimeInForce::GTC) => 0,
        Some(TimeInForce::IOC) => 1,
        Some(TimeInForce::FOK) => 2,
        _ => -1,
    }
}

// -----------------------------------------------------------------------
// Version
// -----------------------------------------------------------------------

/// Return the ALICE-FIX crate version string (static, do not free).
#[no_mangle]
pub extern "C" fn af_fix_version() -> *const c_char {
    static VERSION: OnceLock<CString> = OnceLock::new();
    VERSION
        .get_or_init(|| CString::new(crate::VERSION).unwrap())
        .as_ptr()
}

// -----------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    fn c(s: &str) -> CString {
        CString::new(s).unwrap()
    }

    // -- FixMessage --

    #[test]
    fn test_message_new_and_free() {
        unsafe {
            let bs = c("FIX.4.4");
            let mt = c("D");
            let msg = af_fix_message_new(bs.as_ptr(), mt.as_ptr());
            assert!(!msg.is_null());
            af_fix_message_free(msg);
        }
    }

    #[test]
    fn test_message_null_safety() {
        unsafe {
            assert!(af_fix_message_new(std::ptr::null(), std::ptr::null()).is_null());
            af_fix_message_free(std::ptr::null_mut());
        }
    }

    #[test]
    fn test_message_set_and_get() {
        unsafe {
            let bs = c("FIX.4.4");
            let mt = c("D");
            let msg = af_fix_message_new(bs.as_ptr(), mt.as_ptr());
            let val = c("ALICE");
            af_fix_message_set(msg, 49, val.as_ptr());

            let got = af_fix_message_get(msg, 49);
            assert!(!got.is_null());
            assert_eq!(CStr::from_ptr(got).to_str().unwrap(), "ALICE");
            af_fix_string_free(got);

            // Absent tag returns null.
            assert!(af_fix_message_get(msg, 999).is_null());

            af_fix_message_free(msg);
        }
    }

    #[test]
    fn test_message_get_i64_u64() {
        unsafe {
            let bs = c("FIX.4.4");
            let mt = c("D");
            let msg = af_fix_message_new(bs.as_ptr(), mt.as_ptr());
            let val = c("42");
            af_fix_message_set(msg, 34, val.as_ptr());

            let mut out_i: i64 = 0;
            assert_eq!(af_fix_message_get_i64(msg, 34, &mut out_i), 1);
            assert_eq!(out_i, 42);

            let mut out_u: u64 = 0;
            assert_eq!(af_fix_message_get_u64(msg, 34, &mut out_u), 1);
            assert_eq!(out_u, 42);

            // Missing tag.
            assert_eq!(af_fix_message_get_i64(msg, 999, &mut out_i), 0);
            assert_eq!(af_fix_message_get_u64(msg, 999, &mut out_u), 0);

            af_fix_message_free(msg);
        }
    }

    #[test]
    fn test_message_begin_string_and_msg_type() {
        unsafe {
            let bs = c("FIX.4.4");
            let mt = c("A");
            let msg = af_fix_message_new(bs.as_ptr(), mt.as_ptr());

            let got_bs = af_fix_message_begin_string(msg);
            assert_eq!(CStr::from_ptr(got_bs).to_str().unwrap(), "FIX.4.4");
            af_fix_string_free(got_bs);

            let got_mt = af_fix_message_msg_type(msg);
            assert_eq!(CStr::from_ptr(got_mt).to_str().unwrap(), "A");
            af_fix_string_free(got_mt);

            af_fix_message_free(msg);
        }
    }

    // -- FixBuilder --

    #[test]
    fn test_builder_roundtrip() {
        unsafe {
            let bs = c("FIX.4.4");
            let mt = c("0");
            let builder = af_fix_builder_new(bs.as_ptr(), mt.as_ptr());
            assert!(!builder.is_null());

            let sender = c("ALICE");
            af_fix_builder_field(builder, 49, sender.as_ptr());
            af_fix_builder_field_u64(builder, 34, 1);

            let mut len: i32 = 0;
            let ptr = af_fix_builder_build(builder, &mut len);
            assert!(!ptr.is_null());
            assert!(len > 0);

            // Parse the built bytes.
            let msg = af_fix_parse(ptr, len);
            assert!(!msg.is_null());

            let got_mt = af_fix_message_msg_type(msg);
            assert_eq!(CStr::from_ptr(got_mt).to_str().unwrap(), "0");
            af_fix_string_free(got_mt);

            af_fix_message_free(msg);
            af_fix_bytes_free(ptr, len);
            af_fix_builder_free(builder);
        }
    }

    #[test]
    fn test_builder_null_safety() {
        unsafe {
            assert!(af_fix_builder_new(std::ptr::null(), std::ptr::null()).is_null());
            af_fix_builder_free(std::ptr::null_mut());
        }
    }

    // -- Parser --

    #[test]
    fn test_parse_null_safety() {
        unsafe {
            assert!(af_fix_parse(std::ptr::null(), 0).is_null());
            assert!(af_fix_parse(std::ptr::null(), 100).is_null());
        }
    }

    #[test]
    fn test_checksum() {
        unsafe {
            let data = b"ABC";
            assert_eq!(af_fix_checksum(data.as_ptr(), 3), 198); // 65+66+67=198
            assert_eq!(af_fix_checksum(std::ptr::null(), 0), 0);
        }
    }

    // -- FixSession --

    #[test]
    fn test_session_lifecycle() {
        unsafe {
            let sender = c("ALICE");
            let target = c("BROKER");
            let bs = c("FIX.4.4");
            let session = af_fix_session_new(sender.as_ptr(), target.as_ptr(), bs.as_ptr());
            assert!(!session.is_null());

            // Initial state: Disconnected (0).
            assert_eq!(af_fix_session_state(session), 0);

            // Outgoing seq starts at 1.
            assert_eq!(af_fix_session_next_outgoing_seq(session), 1);
            assert_eq!(af_fix_session_next_outgoing_seq(session), 2);

            // Incoming seq validation.
            assert_eq!(af_fix_session_validate_incoming_seq(session, 1), 1);
            assert_eq!(af_fix_session_validate_incoming_seq(session, 3), 0); // gap
            assert_eq!(af_fix_session_validate_incoming_seq(session, 2), 1);

            // Build logon -> state becomes LogonSent (1).
            let mut len: i32 = 0;
            let logon = af_fix_session_build_logon(session, &mut len);
            assert!(!logon.is_null());
            assert!(len > 0);
            assert_eq!(af_fix_session_state(session), 1);
            af_fix_bytes_free(logon, len);

            // Build heartbeat -> state unchanged.
            let hb = af_fix_session_build_heartbeat(session, &mut len);
            assert!(!hb.is_null());
            assert_eq!(af_fix_session_state(session), 1);
            af_fix_bytes_free(hb, len);

            // Build logout -> state becomes LogoutSent (3).
            let logout = af_fix_session_build_logout(session, &mut len);
            assert!(!logout.is_null());
            assert_eq!(af_fix_session_state(session), 3);
            af_fix_bytes_free(logout, len);

            af_fix_session_free(session);
        }
    }

    #[test]
    fn test_session_null_safety() {
        unsafe {
            assert!(
                af_fix_session_new(std::ptr::null(), std::ptr::null(), std::ptr::null()).is_null()
            );
            af_fix_session_free(std::ptr::null_mut());
            assert_eq!(af_fix_session_state(std::ptr::null()), 0);
            assert_eq!(af_fix_session_next_outgoing_seq(std::ptr::null_mut()), 0);
        }
    }

    // -- Convert --

    #[test]
    fn test_side_conversion() {
        unsafe {
            // Bid=0 -> "1", Ask=1 -> "2"
            let bid = af_fix_side_to_fix(0);
            assert!(!bid.is_null());
            assert_eq!(CStr::from_ptr(bid).to_str().unwrap(), "1");

            let ask = af_fix_side_to_fix(1);
            assert_eq!(CStr::from_ptr(ask).to_str().unwrap(), "2");

            assert!(af_fix_side_to_fix(99).is_null());

            // Reverse.
            let one = c("1");
            let two = c("2");
            let bad = c("X");
            assert_eq!(af_fix_side_from_fix(one.as_ptr()), 0);
            assert_eq!(af_fix_side_from_fix(two.as_ptr()), 1);
            assert_eq!(af_fix_side_from_fix(bad.as_ptr()), -1);
            assert_eq!(af_fix_side_from_fix(std::ptr::null()), -1);
        }
    }

    #[test]
    fn test_ord_type_conversion() {
        unsafe {
            assert_eq!(
                CStr::from_ptr(af_fix_ord_type_to_fix(0)).to_str().unwrap(),
                "1"
            );
            assert_eq!(
                CStr::from_ptr(af_fix_ord_type_to_fix(1)).to_str().unwrap(),
                "2"
            );
            assert!(af_fix_ord_type_to_fix(99).is_null());

            let one = c("1");
            let two = c("2");
            assert_eq!(af_fix_ord_type_from_fix(one.as_ptr()), 0);
            assert_eq!(af_fix_ord_type_from_fix(two.as_ptr()), 1);
            assert_eq!(af_fix_ord_type_from_fix(std::ptr::null()), -1);
        }
    }

    #[test]
    fn test_tif_conversion() {
        unsafe {
            // GTC=0->"1", IOC=1->"3", FOK=2->"4", GTD=3->"6"
            assert_eq!(CStr::from_ptr(af_fix_tif_to_fix(0)).to_str().unwrap(), "1");
            assert_eq!(CStr::from_ptr(af_fix_tif_to_fix(1)).to_str().unwrap(), "3");
            assert_eq!(CStr::from_ptr(af_fix_tif_to_fix(2)).to_str().unwrap(), "4");
            assert_eq!(CStr::from_ptr(af_fix_tif_to_fix(3)).to_str().unwrap(), "6");
            assert!(af_fix_tif_to_fix(99).is_null());

            let gtc = c("1");
            let ioc = c("3");
            let fok = c("4");
            assert_eq!(af_fix_tif_from_fix(gtc.as_ptr()), 0);
            assert_eq!(af_fix_tif_from_fix(ioc.as_ptr()), 1);
            assert_eq!(af_fix_tif_from_fix(fok.as_ptr()), 2);
            assert_eq!(af_fix_tif_from_fix(std::ptr::null()), -1);
        }
    }

    // -- Version --

    #[test]
    fn test_version() {
        unsafe {
            let v = af_fix_version();
            assert!(!v.is_null());
            let s = CStr::from_ptr(v).to_str().unwrap();
            assert!(s.starts_with("0."));
        }
    }
}
