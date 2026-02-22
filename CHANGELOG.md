# Changelog

All notable changes to ALICE-FIX will be documented in this file.

## [0.1.0] - 2026-02-23

### Added
- `tag` — well-known FIX tag constants (FIX 4.4 / 5.0)
- `message` — `FixMessage` parsed tag/value map representation
- `parser` — zero-copy FIX wire-format parser with checksum validation
- `builder` — `FixBuilder` message serializer (auto BeginString, BodyLength, Checksum)
- `session` — FIX session state machine (Logon, Logout, Heartbeat, sequence gap detection)
- `convert` — conversions between FIX field values and ALICE-Ledger types (`Side`, `OrderType`, `TimeInForce`)
- Integration with ALICE-Ledger order types
- 124 tests (123 unit + 1 doc-test)
