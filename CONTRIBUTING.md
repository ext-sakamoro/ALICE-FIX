# Contributing to ALICE-FIX

## Build

```bash
cargo build
```

## Test

```bash
cargo test
```

## Lint

```bash
cargo clippy -- -W clippy::all
cargo fmt -- --check
cargo doc --no-deps 2>&1 | grep warning
```

## Design Constraints

- **Zero-copy parsing**: the parser borrows from the input buffer — no allocation for field values.
- **Automatic framing**: `FixBuilder` manages BeginString (tag 8), BodyLength (tag 9), and Checksum (tag 10).
- **Session sequencing**: sequence numbers are tracked per session; gap detection triggers resend requests.
- **FIX 4.4 / 5.0**: tag constants cover both protocol versions.
- **ALICE-Ledger interop**: `convert` module maps FIX field values to ALICE-Ledger `Side`, `OrderType`, `TimeInForce`.
