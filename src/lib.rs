/*
    ALICE-FIX
    Copyright (C) 2026 Moroya Sakamoto
*/

//! # ALICE-FIX
//!
//! FIX protocol 4.4/5.0 message parser, builder, and session management
//! for the ALICE financial system.
//!
//! ## Modules
//!
//! - [`tag`]     — Well-known FIX tag number constants (FIX 4.4 / 5.0)
//! - [`message`] — [`FixMessage`] representation (parsed tag/value map)
//! - [`parser`]  — Zero-copy FIX wire-format parser
//! - [`builder`] — FIX message serializer / builder
//! - [`session`] — FIX session state machine (logon, logout, heartbeat, sequencing)
//! - [`convert`] — Conversions between FIX values and ALICE-Ledger types
//!
//! ## Example
//!
//! ```rust
//! use alice_fix::{builder::FixBuilder, parser, tag};
//!
//! // Build a simple Heartbeat.
//! let bytes = FixBuilder::new("FIX.4.4", "0")
//!     .field(tag::SENDER_COMP_ID, "ALICE")
//!     .field(tag::TARGET_COMP_ID, "BROKER")
//!     .field(tag::MSG_SEQ_NUM, "1")
//!     .field(tag::SENDING_TIME, "20260101-00:00:00")
//!     .build();
//!
//! let msg = parser::parse(&bytes).unwrap();
//! assert_eq!(msg.msg_type, "0");
//! assert_eq!(msg.get(tag::SENDER_COMP_ID), Some("ALICE"));
//! ```

pub mod builder;
pub mod convert;
pub mod message;
pub mod parser;
pub mod session;
pub mod tag;

// Re-export the most commonly used types at the crate root.
pub use builder::FixBuilder;
pub use message::FixMessage;
pub use parser::ParseError;
pub use session::{FixSession, SessionState};

/// ALICE-FIX crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
