/*
    ALICE-FIX
    Copyright (C) 2026 Moroya Sakamoto
*/

//! FIX protocol tag number constants (FIX 4.4 / 5.0).
//!
//! Each constant is the integer tag number as defined in the FIX specification.
//! Tags are `u32` to match the field key type used throughout ALICE-FIX.

// ---------------------------------------------------------------------------
// Standard header tags
// ---------------------------------------------------------------------------

/// Tag 8 — BeginString: identifies the FIX version (e.g., "FIX.4.4").
pub const BEGIN_STRING: u32 = 8;

/// Tag 9 — BodyLength: number of bytes from the first byte after tag 9's
/// delimiter up to and including the delimiter preceding tag 10.
pub const BODY_LENGTH: u32 = 9;

/// Tag 35 — MsgType: identifies the message type (e.g., "D" = NewOrderSingle).
pub const MSG_TYPE: u32 = 35;

/// Tag 49 — SenderCompID: assigned value identifying the sending firm.
pub const SENDER_COMP_ID: u32 = 49;

/// Tag 56 — TargetCompID: assigned value identifying the receiving firm.
pub const TARGET_COMP_ID: u32 = 56;

/// Tag 34 — MsgSeqNum: integer message sequence number.
pub const MSG_SEQ_NUM: u32 = 34;

/// Tag 52 — SendingTime: UTC timestamp when the message was transmitted.
pub const SENDING_TIME: u32 = 52;

/// Tag 10 — CheckSum: three-digit modulo-256 checksum of the message bytes.
pub const CHECKSUM: u32 = 10;

// ---------------------------------------------------------------------------
// Order identification
// ---------------------------------------------------------------------------

/// Tag 11 — ClOrdID: unique identifier for an order assigned by the client.
pub const CL_ORD_ID: u32 = 11;

/// Tag 37 — OrderID: unique identifier for an order assigned by the broker.
pub const ORDER_ID: u32 = 37;

/// Tag 17 — ExecID: unique identifier for an execution report.
pub const EXEC_ID: u32 = 17;

// ---------------------------------------------------------------------------
// Instrument
// ---------------------------------------------------------------------------

/// Tag 55 — Symbol: ticker symbol for the traded instrument.
pub const SYMBOL: u32 = 55;

// ---------------------------------------------------------------------------
// Order attributes
// ---------------------------------------------------------------------------

/// Tag 54 — Side: direction of the order. "1" = Buy, "2" = Sell.
pub const SIDE: u32 = 54;

/// Tag 40 — OrdType: order classification. "1" = Market, "2" = Limit.
pub const ORD_TYPE: u32 = 40;

/// Tag 44 — Price: limit price for limit and stop-limit orders.
pub const PRICE: u32 = 44;

/// Tag 38 — OrderQty: number of units to buy or sell.
pub const ORDER_QTY: u32 = 38;

/// Tag 59 — TimeInForce: how long an order remains active.
/// "0" = Day, "1" = GTC, "3" = IOC, "4" = FOK.
pub const TIME_IN_FORCE: u32 = 59;

// ---------------------------------------------------------------------------
// Execution report fields
// ---------------------------------------------------------------------------

/// Tag 150 — ExecType: execution report type code.
pub const EXEC_TYPE: u32 = 150;

/// Tag 39 — OrdStatus: current status of an order.
pub const ORD_STATUS: u32 = 39;

/// Tag 31 — LastPx: price of the most recent fill.
pub const LAST_PX: u32 = 31;

/// Tag 32 — LastQty: quantity of the most recent fill.
pub const LAST_QTY: u32 = 32;

/// Tag 151 — LeavesQty: quantity open for further execution.
pub const LEAVES_QTY: u32 = 151;

/// Tag 14 — CumQty: total quantity filled across all executions for this order.
pub const CUM_QTY: u32 = 14;

/// Tag 6 — AvgPx: average price of all fills for this order.
pub const AVG_PX: u32 = 6;

/// Tag 60 — TransactTime: UTC timestamp of the transaction.
pub const TRANSACT_TIME: u32 = 60;

// ---------------------------------------------------------------------------
// Miscellaneous
// ---------------------------------------------------------------------------

/// Tag 58 — Text: free-form text field for human-readable annotations.
pub const TEXT: u32 = 58;
