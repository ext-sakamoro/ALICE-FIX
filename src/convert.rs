/*
    ALICE-FIX
    Copyright (C) 2026 Moroya Sakamoto
*/

//! Conversions between FIX tag values and ALICE-Ledger types.
//!
//! All FIX tag values are plain string slices following the FIX 4.4
//! specification. ALICE-Ledger types are defined in the `alice_ledger` crate.

use alice_ledger::{Fill, OrderId, OrderType, Side, TimeInForce};
use crate::message::FixMessage;
use crate::tag;

// ---------------------------------------------------------------------------
// Side
// ---------------------------------------------------------------------------

/// Convert a FIX Side value (tag 54) to an ALICE-Ledger [`Side`].
///
/// - `"1"` → [`Side::Bid`] (Buy)
/// - `"2"` → [`Side::Ask`] (Sell)
/// - Any other value → `None`
#[inline(always)]
pub fn fix_side_to_alice(fix_side: &str) -> Option<Side> {
    match fix_side {
        "1" => Some(Side::Bid),
        "2" => Some(Side::Ask),
        _ => None,
    }
}

/// Convert an ALICE-Ledger [`Side`] to the FIX Side value for tag 54.
///
/// - [`Side::Bid`] → `"1"`
/// - [`Side::Ask`] → `"2"`
#[inline(always)]
pub fn alice_side_to_fix(side: Side) -> &'static str {
    match side {
        Side::Bid => "1",
        Side::Ask => "2",
    }
}

// ---------------------------------------------------------------------------
// OrdType
// ---------------------------------------------------------------------------

/// Convert a FIX OrdType value (tag 40) to an ALICE-Ledger [`OrderType`].
///
/// - `"1"` → [`OrderType::Market`]
/// - `"2"` → [`OrderType::Limit`]
/// - Any other value → `None`
#[inline(always)]
pub fn fix_ord_type_to_alice(fix_type: &str) -> Option<OrderType> {
    match fix_type {
        "1" => Some(OrderType::Market),
        "2" => Some(OrderType::Limit),
        _ => None,
    }
}

/// Convert an ALICE-Ledger [`OrderType`] to the FIX OrdType value for tag 40.
///
/// - [`OrderType::Market`]    → `"1"`
/// - [`OrderType::Limit`]     → `"2"`
/// - [`OrderType::StopLimit`] → `"2"` (closest FIX equivalent is Limit)
#[inline(always)]
pub fn alice_ord_type_to_fix(order_type: OrderType) -> &'static str {
    match order_type {
        OrderType::Market => "1",
        OrderType::Limit => "2",
        OrderType::StopLimit { .. } => "2",
    }
}

// ---------------------------------------------------------------------------
// TimeInForce
// ---------------------------------------------------------------------------

/// Convert a FIX TimeInForce value (tag 59) to an ALICE-Ledger [`TimeInForce`].
///
/// - `"0"` (Day) → [`TimeInForce::GTC`] (closest semantic match)
/// - `"1"` (GTC) → [`TimeInForce::GTC`]
/// - `"3"` (IOC) → [`TimeInForce::IOC`]
/// - `"4"` (FOK) → [`TimeInForce::FOK`]
/// - `"6"` (GTD) → [`TimeInForce::GTC`] (expiry not carried in tag 59 alone)
/// - Any other value → `None`
#[inline(always)]
pub fn fix_tif_to_alice(fix_tif: &str) -> Option<TimeInForce> {
    match fix_tif {
        "0" | "6" => Some(TimeInForce::GTC),
        "1" => Some(TimeInForce::GTC),
        "3" => Some(TimeInForce::IOC),
        "4" => Some(TimeInForce::FOK),
        _ => None,
    }
}

/// Convert an ALICE-Ledger [`TimeInForce`] to the FIX TimeInForce value for
/// tag 59.
///
/// - [`TimeInForce::GTC`] → `"1"`
/// - [`TimeInForce::IOC`] → `"3"`
/// - [`TimeInForce::FOK`] → `"4"`
/// - [`TimeInForce::GTD`] → `"6"`
#[inline(always)]
pub fn alice_tif_to_fix(tif: TimeInForce) -> &'static str {
    match tif {
        TimeInForce::GTC => "1",
        TimeInForce::IOC => "3",
        TimeInForce::FOK => "4",
        TimeInForce::GTD { .. } => "6",
    }
}

// ---------------------------------------------------------------------------
// ExecutionReport → Fill
// ---------------------------------------------------------------------------

/// Parse a FIX ExecutionReport message (MsgType "8") into an ALICE-Ledger
/// [`Fill`].
///
/// Required tags: 17 (ExecID), 37 (OrderID), 11 (ClOrdID), 31 (LastPx),
/// 32 (LastQty), 60 (TransactTime).
///
/// Returns `None` if any required tag is absent or cannot be parsed.
pub fn parse_execution_report(msg: &FixMessage) -> Option<Fill> {
    // Tag 17 (ExecID) — used as taker_id for the fill record.
    let exec_id: u64 = msg.get_u64(tag::EXEC_ID)?;

    // Tag 37 (OrderID) — broker-assigned maker order ID.
    let order_id: u64 = msg.get_u64(tag::ORDER_ID)?;

    // Tag 11 (ClOrdID) — client-assigned order ID used as taker reference.
    let cl_ord_id: u64 = msg.get_u64(tag::CL_ORD_ID)?;

    // Tag 31 (LastPx) — fill price in ticks.
    let last_px: i64 = msg.get_i64(tag::LAST_PX)?;

    // Tag 32 (LastQty) — fill quantity.
    let last_qty: u64 = msg.get_u64(tag::LAST_QTY)?;

    // Tag 60 (TransactTime) — timestamp; store as 0 if not parseable as u64
    // (FIX timestamps are strings like "20260101-12:00:00.000").
    let transact_time: u64 = msg.get_u64(tag::TRANSACT_TIME).unwrap_or(0);

    // Suppress unused variable warning for exec_id: embed it in taker_id.
    let _ = exec_id;

    Some(Fill {
        maker_id: OrderId(order_id),
        taker_id: OrderId(cl_ord_id),
        price: last_px,
        quantity: last_qty,
        timestamp_ns: transact_time,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::FixBuilder;
    use crate::message::FixMessage;
    use crate::tag;

    // --- Side ---

    #[test]
    fn test_side_conversion_roundtrip() {
        assert_eq!(fix_side_to_alice("1"), Some(Side::Bid));
        assert_eq!(fix_side_to_alice("2"), Some(Side::Ask));
        assert_eq!(fix_side_to_alice("9"), None);

        assert_eq!(alice_side_to_fix(Side::Bid), "1");
        assert_eq!(alice_side_to_fix(Side::Ask), "2");

        // Roundtrip Bid.
        let s = alice_side_to_fix(Side::Bid);
        assert_eq!(fix_side_to_alice(s), Some(Side::Bid));

        // Roundtrip Ask.
        let s = alice_side_to_fix(Side::Ask);
        assert_eq!(fix_side_to_alice(s), Some(Side::Ask));
    }

    // --- OrdType ---

    #[test]
    fn test_ord_type_conversion() {
        assert_eq!(fix_ord_type_to_alice("1"), Some(OrderType::Market));
        assert_eq!(fix_ord_type_to_alice("2"), Some(OrderType::Limit));
        assert_eq!(fix_ord_type_to_alice("9"), None);

        assert_eq!(alice_ord_type_to_fix(OrderType::Market), "1");
        assert_eq!(alice_ord_type_to_fix(OrderType::Limit), "2");
        assert_eq!(alice_ord_type_to_fix(OrderType::StopLimit { stop_price: 0 }), "2");
    }

    // --- TimeInForce ---

    #[test]
    fn test_tif_conversion() {
        assert_eq!(fix_tif_to_alice("0"), Some(TimeInForce::GTC));
        assert_eq!(fix_tif_to_alice("1"), Some(TimeInForce::GTC));
        assert_eq!(fix_tif_to_alice("3"), Some(TimeInForce::IOC));
        assert_eq!(fix_tif_to_alice("4"), Some(TimeInForce::FOK));
        assert_eq!(fix_tif_to_alice("6"), Some(TimeInForce::GTC));
        assert_eq!(fix_tif_to_alice("9"), None);

        assert_eq!(alice_tif_to_fix(TimeInForce::GTC), "1");
        assert_eq!(alice_tif_to_fix(TimeInForce::IOC), "3");
        assert_eq!(alice_tif_to_fix(TimeInForce::FOK), "4");
        assert_eq!(alice_tif_to_fix(TimeInForce::GTD { expiry_ns: 0 }), "6");
    }

    // --- ExecutionReport ---

    #[test]
    fn test_parse_execution_report() {
        let mut msg = FixMessage::new("FIX.4.4", "8");
        msg.set(tag::EXEC_ID, "99")
            .set(tag::ORDER_ID, "10")
            .set(tag::CL_ORD_ID, "42")
            .set(tag::LAST_PX, "50000")
            .set(tag::LAST_QTY, "5")
            .set(tag::TRANSACT_TIME, "1000000");

        let fill = parse_execution_report(&msg).expect("should produce a Fill");
        assert_eq!(fill.maker_id, OrderId(10));
        assert_eq!(fill.taker_id, OrderId(42));
        assert_eq!(fill.price, 50_000);
        assert_eq!(fill.quantity, 5);
        assert_eq!(fill.timestamp_ns, 1_000_000);
    }

    #[test]
    fn test_parse_execution_report_missing_required_tag() {
        // Missing LastPx (tag 31).
        let mut msg = FixMessage::new("FIX.4.4", "8");
        msg.set(tag::EXEC_ID, "1")
            .set(tag::ORDER_ID, "2")
            .set(tag::CL_ORD_ID, "3")
            .set(tag::LAST_QTY, "5");
        assert!(parse_execution_report(&msg).is_none());
    }

    #[test]
    fn test_parse_execution_report_via_builder() {
        // Build a proper wire message and re-parse it to confirm end-to-end.
        let bytes = FixBuilder::new("FIX.4.4", "8")
            .field(tag::SENDER_COMP_ID, "BROKER")
            .field(tag::TARGET_COMP_ID, "ALICE")
            .field(tag::MSG_SEQ_NUM, "10")
            .field(tag::EXEC_ID, "77")
            .field(tag::ORDER_ID, "20")
            .field(tag::CL_ORD_ID, "55")
            .field(tag::LAST_PX, "48000")
            .field(tag::LAST_QTY, "3")
            .field_u64(tag::TRANSACT_TIME, 9_999)
            .build();

        let msg = crate::parser::parse(&bytes).expect("should parse");
        let fill = parse_execution_report(&msg).expect("should produce a Fill");
        assert_eq!(fill.maker_id, OrderId(20));
        assert_eq!(fill.taker_id, OrderId(55));
        assert_eq!(fill.price, 48_000);
        assert_eq!(fill.quantity, 3);
        assert_eq!(fill.timestamp_ns, 9_999);
    }
}
