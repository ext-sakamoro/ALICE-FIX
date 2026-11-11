# Changelog

All notable changes to ALICE-FIX will be documented in this file.

## [Unreleased]

### Changed
- 依存: `alice-ledger` (path) を ALICE-Ledger の main (git、branch、0.3.0) から取る `Cargo.lock` を追跡し、clippy と試験は `--locked` で走らせる CI は隣の repo の clone (`.github/alice-siblings`) をやめ、lock の alice-* がすべて git から来て `alice-det-math` が 1 つ以下であることを確かめる step を足した `scripts/preflight.sh` は生成し直した 公開 API は変わらない

## [0.1.1] - 2026-03-04

### Added
- `ffi` — C-ABI FFI 33 `extern "C"` functions (Memory/Message/Builder/Parser/Session/Convert/Version)
- Unity C# bindings — 33 DllImport + 5 RAII IDisposable handles (`bindings/unity/AliceFix.cs`)
- UE5 C++ bindings — 33 extern C + 5 RAII unique_ptr handles (`bindings/ue5/AliceFix.h`)
- FFI prefix: `af_fix_*`
- README.md
- 139 tests (124 core + 15 FFI)

### Fixed
- `cargo fmt` trailing space修正

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
