## Why

Issue #24's direct Rust plugin path must transfer a prepared runtime into a host-owned audio thread. The pinned Clack interface requires `Send`, but two internal trait objects currently erase that guarantee despite the existing Rust/C++ host behavior.

## What Changes

- Make ownership transfer of the public prepared runtime compiler-checked through its actual internal owners.
- Add an external public-API integration test that prepares a representative gain/filter graph off audio, moves its processor and preallocated planes to a render thread, checks literal signed output and moves it back for reset/re-render and off-audio destruction.
- Retain existing DSP, exclusivity, realtime allocation guards and C ABI consumers; use no unsafe marker or locking wrapper.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `headless-engine`: add owned-runtime transfer between host threads while preserving signed rendering and reset behavior.

## Impact

Two internal Rust trait bounds and a public integration test, proving spec mapping and the issue24 progress receipt. This is a direct-Rust adapter prerequisite; it does not introduce a plugin framework dependency, new DSP, a second session/journal, shared concurrent mutation or a completed CLAP/native-host migration claim.
