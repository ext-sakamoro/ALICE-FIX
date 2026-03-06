//! Administrative message builders
//!
//! Logon, Heartbeat, Logout, `TestRequest`, `ResendRequest`。

use crate::builder::FixBuilder;
use crate::tag;

/// Administrative メッセージ種別。
pub mod msg_type {
    /// Heartbeat。
    pub const HEARTBEAT: &str = "0";
    /// Test Request。
    pub const TEST_REQUEST: &str = "1";
    /// Resend Request。
    pub const RESEND_REQUEST: &str = "2";
    /// Reject。
    pub const REJECT: &str = "3";
    /// Sequence Reset。
    pub const SEQUENCE_RESET: &str = "4";
    /// Logout。
    pub const LOGOUT: &str = "5";
    /// Logon。
    pub const LOGON: &str = "A";
}

/// Logon タグ。
const ENCRYPT_METHOD: u32 = 98;
/// Heartbeat interval タグ。
const HEART_BT_INT: u32 = 108;
/// Test request ID タグ。
const TEST_REQ_ID: u32 = 112;
/// Begin sequence number タグ。
const BEGIN_SEQ_NO: u32 = 7;
/// End sequence number タグ。
const END_SEQ_NO: u32 = 16;

/// Logon メッセージを構築。
#[must_use]
pub fn build_logon(
    begin_string: &str,
    sender: &str,
    target: &str,
    seq_num: u64,
    sending_time: &str,
    heartbeat_interval: u32,
) -> Vec<u8> {
    let mut b = FixBuilder::new(begin_string, msg_type::LOGON);
    b.field(tag::SENDER_COMP_ID, sender);
    b.field(tag::TARGET_COMP_ID, target);
    b.field(tag::MSG_SEQ_NUM, &seq_num.to_string());
    b.field(tag::SENDING_TIME, sending_time);
    b.field(ENCRYPT_METHOD, "0");
    b.field(HEART_BT_INT, &heartbeat_interval.to_string());
    b.build()
}

/// Heartbeat メッセージを構築。
#[must_use]
pub fn build_heartbeat(
    begin_string: &str,
    sender: &str,
    target: &str,
    seq_num: u64,
    sending_time: &str,
    test_req_id: Option<&str>,
) -> Vec<u8> {
    let mut b = FixBuilder::new(begin_string, msg_type::HEARTBEAT);
    b.field(tag::SENDER_COMP_ID, sender);
    b.field(tag::TARGET_COMP_ID, target);
    b.field(tag::MSG_SEQ_NUM, &seq_num.to_string());
    b.field(tag::SENDING_TIME, sending_time);
    if let Some(id) = test_req_id {
        b.field(TEST_REQ_ID, id);
    }
    b.build()
}

/// Logout メッセージを構築。
#[must_use]
pub fn build_logout(
    begin_string: &str,
    sender: &str,
    target: &str,
    seq_num: u64,
    sending_time: &str,
    text: Option<&str>,
) -> Vec<u8> {
    let mut b = FixBuilder::new(begin_string, msg_type::LOGOUT);
    b.field(tag::SENDER_COMP_ID, sender);
    b.field(tag::TARGET_COMP_ID, target);
    b.field(tag::MSG_SEQ_NUM, &seq_num.to_string());
    b.field(tag::SENDING_TIME, sending_time);
    if let Some(t) = text {
        b.field(tag::TEXT, t);
    }
    b.build()
}

/// Test Request メッセージを構築。
#[must_use]
pub fn build_test_request(
    begin_string: &str,
    sender: &str,
    target: &str,
    seq_num: u64,
    sending_time: &str,
    test_req_id: &str,
) -> Vec<u8> {
    let mut b = FixBuilder::new(begin_string, msg_type::TEST_REQUEST);
    b.field(tag::SENDER_COMP_ID, sender);
    b.field(tag::TARGET_COMP_ID, target);
    b.field(tag::MSG_SEQ_NUM, &seq_num.to_string());
    b.field(tag::SENDING_TIME, sending_time);
    b.field(TEST_REQ_ID, test_req_id);
    b.build()
}

/// Resend Request メッセージを構築。
#[must_use]
pub fn build_resend_request(
    begin_string: &str,
    sender: &str,
    target: &str,
    seq_num: u64,
    sending_time: &str,
    begin_seq_no: u64,
    end_seq_no: u64,
) -> Vec<u8> {
    let mut b = FixBuilder::new(begin_string, msg_type::RESEND_REQUEST);
    b.field(tag::SENDER_COMP_ID, sender);
    b.field(tag::TARGET_COMP_ID, target);
    b.field(tag::MSG_SEQ_NUM, &seq_num.to_string());
    b.field(tag::SENDING_TIME, sending_time);
    b.field(BEGIN_SEQ_NO, &begin_seq_no.to_string());
    b.field(END_SEQ_NO, &end_seq_no.to_string());
    b.build()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;

    const FIX44: &str = "FIX.4.4";
    const TIME: &str = "20260101-00:00:00";

    #[test]
    fn logon_message() {
        let bytes = build_logon(FIX44, "ALICE", "BROKER", 1, TIME, 30);
        let msg = parser::parse(&bytes).unwrap();
        assert_eq!(msg.msg_type, "A");
        assert_eq!(msg.get(tag::SENDER_COMP_ID), Some("ALICE"));
        assert_eq!(msg.get(HEART_BT_INT), Some("30"));
    }

    #[test]
    fn heartbeat_message() {
        let bytes = build_heartbeat(FIX44, "ALICE", "BROKER", 2, TIME, None);
        let msg = parser::parse(&bytes).unwrap();
        assert_eq!(msg.msg_type, "0");
        assert!(msg.get(TEST_REQ_ID).is_none());
    }

    #[test]
    fn heartbeat_with_test_req_id() {
        let bytes = build_heartbeat(FIX44, "ALICE", "BROKER", 2, TIME, Some("REQ1"));
        let msg = parser::parse(&bytes).unwrap();
        assert_eq!(msg.get(TEST_REQ_ID), Some("REQ1"));
    }

    #[test]
    fn logout_message() {
        let bytes = build_logout(FIX44, "ALICE", "BROKER", 3, TIME, None);
        let msg = parser::parse(&bytes).unwrap();
        assert_eq!(msg.msg_type, "5");
    }

    #[test]
    fn logout_with_text() {
        let bytes = build_logout(FIX44, "ALICE", "BROKER", 3, TIME, Some("Session ended"));
        let msg = parser::parse(&bytes).unwrap();
        assert_eq!(msg.get(tag::TEXT), Some("Session ended"));
    }

    #[test]
    fn test_request_message() {
        let bytes = build_test_request(FIX44, "ALICE", "BROKER", 4, TIME, "TEST123");
        let msg = parser::parse(&bytes).unwrap();
        assert_eq!(msg.msg_type, "1");
        assert_eq!(msg.get(TEST_REQ_ID), Some("TEST123"));
    }

    #[test]
    fn resend_request_message() {
        let bytes = build_resend_request(FIX44, "ALICE", "BROKER", 5, TIME, 1, 10);
        let msg = parser::parse(&bytes).unwrap();
        assert_eq!(msg.msg_type, "2");
        assert_eq!(msg.get(BEGIN_SEQ_NO), Some("1"));
        assert_eq!(msg.get(END_SEQ_NO), Some("10"));
    }

    #[test]
    fn logon_encrypt_method() {
        let bytes = build_logon(FIX44, "A", "B", 1, TIME, 60);
        let msg = parser::parse(&bytes).unwrap();
        assert_eq!(msg.get(ENCRYPT_METHOD), Some("0"));
    }

    #[test]
    fn logon_seq_num() {
        let bytes = build_logon(FIX44, "A", "B", 42, TIME, 30);
        let msg = parser::parse(&bytes).unwrap();
        assert_eq!(msg.get_u64(tag::MSG_SEQ_NUM), Some(42));
    }

    #[test]
    fn msg_type_constants() {
        assert_eq!(msg_type::LOGON, "A");
        assert_eq!(msg_type::HEARTBEAT, "0");
        assert_eq!(msg_type::LOGOUT, "5");
        assert_eq!(msg_type::TEST_REQUEST, "1");
        assert_eq!(msg_type::RESEND_REQUEST, "2");
        assert_eq!(msg_type::REJECT, "3");
        assert_eq!(msg_type::SEQUENCE_RESET, "4");
    }

    #[test]
    fn fix50_logon() {
        let bytes = build_logon("FIXT.1.1", "ALICE", "BROKER", 1, TIME, 30);
        let msg = parser::parse(&bytes).unwrap();
        assert_eq!(msg.begin_string, "FIXT.1.1");
    }

    #[test]
    fn resend_request_zero_end() {
        let bytes = build_resend_request(FIX44, "A", "B", 1, TIME, 5, 0);
        let msg = parser::parse(&bytes).unwrap();
        assert_eq!(msg.get(END_SEQ_NO), Some("0"));
    }

    #[test]
    fn all_admin_messages_parseable() {
        // 全 administrative メッセージが parse 可能であることを確認
        let msgs = [
            build_logon(FIX44, "A", "B", 1, TIME, 30),
            build_heartbeat(FIX44, "A", "B", 2, TIME, None),
            build_logout(FIX44, "A", "B", 3, TIME, None),
            build_test_request(FIX44, "A", "B", 4, TIME, "T1"),
            build_resend_request(FIX44, "A", "B", 5, TIME, 1, 10),
        ];
        for bytes in &msgs {
            assert!(parser::parse(bytes).is_ok());
        }
    }
}
