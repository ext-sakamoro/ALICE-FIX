# ALICE-FIX

**FIX Protocol 4.4/5.0 Message Parser, Builder, and Session Management**

> Zero-copy parsing. Auto-computed checksums. Sequence-safe sessions.

## Features

- **Zero-Copy Parser**: SOH-delimited byte iteration, no intermediate `Vec`
- **Auto Checksum/BodyLength**: `FixBuilder` computes tag 9 and tag 10 automatically
- **Session State Machine**: Logon/Logout/Heartbeat with bidirectional sequence tracking
- **ALICE-Ledger Integration**: Side, OrderType, TimeInForce, ExecutionReport conversions
- **O(1) Tag Lookup**: `FixMessage` backed by `HashMap<u32, String>`
- **C-ABI FFI**: 33 `extern "C"` functions (`af_fix_*` prefix)
- **Unity C# Bindings**: 33 DllImport + 5 RAII IDisposable handles
- **UE5 C++ Bindings**: 33 extern C + 5 RAII `unique_ptr` handles

## Quick Start

```rust
use alice_fix::{builder::FixBuilder, parser, tag};

// Build a Heartbeat message.
let bytes = FixBuilder::new("FIX.4.4", "0")
    .field(tag::SENDER_COMP_ID, "ALICE")
    .field(tag::TARGET_COMP_ID, "BROKER")
    .field(tag::MSG_SEQ_NUM, "1")
    .field(tag::SENDING_TIME, "20260101-00:00:00")
    .build();

// Parse back.
let msg = parser::parse(&bytes).unwrap();
assert_eq!(msg.msg_type, "0");
assert_eq!(msg.get(tag::SENDER_COMP_ID), Some("ALICE"));
```

## Modules

| Module | Description |
|--------|-------------|
| `tag` | 26 FIX tag constants (FIX 4.4/5.0) |
| `message` | `FixMessage` — O(1) tag/value map |
| `parser` | Zero-copy wire parser with checksum validation |
| `builder` | `FixBuilder` — auto BodyLength/Checksum serializer |
| `session` | `FixSession` — state machine + sequence tracking |
| `convert` | FIX ↔ ALICE-Ledger type conversions |
| `ffi` | C-ABI FFI 33 functions (feature: `ffi`) |

## Cargo Features

| Feature | Default | Description |
|---------|---------|-------------|
| `std` | Yes | Standard library support |
| `ffi` | No | C-ABI FFI (33 extern "C" functions) |

## FFI / Bindings

### C-ABI FFI (`--features ffi`)

33 `extern "C"` functions with `af_fix_*` prefix:

| Category | Functions | Description |
|----------|----------|-------------|
| Memory | 2 | String/bytes free |
| FixMessage | 8 | Create/free/set/get/get_i64/get_u64/begin_string/msg_type |
| FixBuilder | 6 | Create/free/field/field_i64/field_u64/build |
| Parser | 2 | parse/checksum |
| FixSession | 8 | Create/free/state/seq/validate/logon/logout/heartbeat |
| Convert | 6 | Side/OrdType/TimeInForce bidirectional |
| Version | 1 | Library version |

### Unity C# (`bindings/unity/AliceFix.cs`)

33 DllImport + 5 RAII IDisposable handles (MessageHandle, BuilderHandle, SessionHandle, StringHandle, BytesHandle) + AliceFix static class + Tag constants.

### UE5 C++ (`bindings/ue5/AliceFix.h`)

33 extern C + 5 RAII unique_ptr handles (MessagePtr, BuilderPtr, SessionPtr, StringPtr, BytesPtr) + helper functions + Tag constants.

## Test Suite

| Feature | Tests |
|---------|-------|
| Core (default) | 124 (123 unit + 1 doc) |
| FFI (`ffi`) | +15 |
| **Total** | **139** |

## License

MIT

## Author

Moroya Sakamoto ([@ext-sakamoro](https://github.com/ext-sakamoro))
