# Processor portable IBStream state — 3 November 2026

The real IComponent adapts standard IBStream to the existing Trigger state codec.
It captures an owned coherent output snapshot before foreign writes. Input reads
header/metadata, checks native admission, buffers payload outside owner borrows,
then revalidates and publishes through the shared owner. Foreign callbacks may
query the processor or terminate it; liveness is rechecked after both read phases.
No second state codec, session or history was added.

Four dynamic tests prove processor-only legacy packaged restoration, successful
partial7/11/13/17-byte I/O, all23 independently non-default named values, complete
source-instance/package loss and fresh signed restoration with a decoy factory
sample removed. Restored note60 renders literal-.14536858 and-.1444931 after its
offset5. A selected-resource wire fixture uses independently hashed constant
negative PCM; native save re-encodes it, source/file loss follows, and a fresh
instance renders13 samples of-.09375 afteroffset3. This is portable selected-
payload mechanics, not actual native file-picker acceptance.

Real native mapping/Undo creates both undoCount1 and redoCount1. Malformed,
truncated, checksum/trailing/metadata/parameter failures, zero progress, failing
I/O and negative/excess SDK progress counts preserve the full accepted session,
all saved values and sounding cursor. Continuation is11869/32768; retained Redo
then remaps to72 and renders-.5. Failed output may retain its external prefix;
musical state is preserved. Missing encoded factory input fails before foreign
output while the already prepared audio still renders-.5.

The pinned SDK MemoryStream contract returns kResultOk for successful short reads
and count-zero EOF. Every failing tresult rejects. Write-zero rejects. Empty Rust
Read/Write makes no foreign call. The shared observed host fixture covers these
at the cheapest owning layer; the new production stream module measures65/65
lines,157/157 regions and all functions. Strict Clippy passes. The processor-only
standard-message fixture was reused by extraction, retaining all four original
owner tests. The current shared-flush CLAP product passes20 dynamic tests and its
parent-executed allocator child. Initial borrow/setup errors remain separate
from the real semantic RED(kNotImplemented for valid legacy input).

Full controller projection, native gestures/editor destruction/recreation,
installed REAPER, unfiltered validator and exact combined Astra review/publication
remain mandatory. These accumulated TDD profiles are not final clean-profile
coverage. All binaries/full LLVM JSON remain private.
