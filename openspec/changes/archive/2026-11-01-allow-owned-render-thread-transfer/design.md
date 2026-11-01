## Context

The independent engine build is published at fcd8eaf with receipt e817c03. Its full baseline passed 18 CTest checks, including 1,088 Rust tests. Exact pinned Clack source 27ca283 requires PluginAudioProcessor: Sized + Send; its main thread activates/deactivates and the host moves exclusive mutable audio ownership between render threads. The engine's erased PreparedStepExecutor and FilterAlgorithm owners currently omit Send.

## Goals / Non-Goals

**Goals:** compiler-checked transfer of the existing owned runtime, preserving actual child DSP and reset state through a public production-API consumer.

**Non-Goals:** Sync/shared mutation, new DSP, allocations or locks in callbacks, unsafe marker implementations, a new facade/session/journal or declaring the replacement plugin proven.

## Decisions

- Establish the current signed gain/filter output and reset oracle on the control thread before production edits, then add external integration tests using only public preparation and runtime APIs. The transfer test must fail to compile with E0277 before bounds change.
- Require Send on the two internal erased-owner traits and let Rust verify every concrete owner. Do not use unsafe impl Send or Arc<Mutex<Runtime>> to bypass the constraint. No process function or filter transformation should change.
- Prepare buffers and create/join test threads off rendering. Return the owned processor to the control thread for reset and destruction. Include a second independent instance so ownership/state persistence is genuinely exercised.
- Keep the current engine C ABI/build and realtime allocation tests; run focused public test then complete relevant engine checks using the existing task-owned Cargo target. Track tests in the headless-engine canonical map.

## Risks / Trade-offs

- [A concrete owner may genuinely be thread-bound] → Compiler rejection is evidence; stop and inspect that owner rather than adding an unsafe assertion or altering DSP.
- [A test under cfg(test) may miss the production type] → Use the external integration test against the public compiled library.
- [Signed filter oracle becomes a relative-only comparison] → Establish literal signed expected samples before changes, use an explicit numerical tolerance and independently assert both channels.
- [Growing duplicate caches] → Reuse the existing task Cargo target; retain source/build outputs and check C: free space before broad checks.

## Migration Plan

Characterize the baseline, confirm compiler RED, add owner bounds, run focused and full engine checks, sync/map the additive scenarios, review and publish this prerequisite. Continue the bounded real-engine CLAP adapter proof afterward; full J1/J2 and #24 remain open.
