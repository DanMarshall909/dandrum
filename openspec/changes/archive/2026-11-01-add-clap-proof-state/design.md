## Context

Continue issue #24 and the accepted JUCE-free beta on the existing feature worktree. The two experimental built-ins expose 1/7 public numeric root controls; processing and flush already share atomic parameter values. Existing dynamic tests prove signed gain and timed TB-303 audio. No state extension is advertised.

## Goals / Non-Goals

**Goals:** replacement-owned parameter state for these two fixtures, before activation and between processing calls; explicit bounded rejection; honest stopped-flush evidence.

**Non-Goals:** Trigger assets/documents/journal, final shipping IDs, structural replacement, legacy JUCE import, UI/DAW/device execution, disk-backed undo or mutation runs. #23 remains the single session/SDK authority.

## Decisions

- Enable the pinned Clack `state` feature. Define schema version 1 JSON with `instrument` (experimental descriptor ID) and a complete `parameters` array of musical `id` and plain `value`. Reject unknown fields and duplicate/missing/unknown parameter IDs, wrong instrument, unsupported version and invalid/out-of-range numbers. Entry order does not define identity. Direct serde/serde_json dependencies reuse already-resolved engine dependencies; no framework dependency enters the engine.
- Bound reads at 16 KiB plus one overflow byte and validate all entries before publishing any value. Read/write/serialization happen on the CLAP main thread. Handle partial stream operations through standard Read/Write helpers; failed save/load reports failure. This fixture format stores scalar controls only and will require explicit versioning when session state is introduced.
- Restore publishes the existing independent atomic controls and notifies an available host parameter extension with VALUES rescan. It neither reparses/prepares a graph nor resets DSP. Concurrent sample-offset automation retains the existing per-parameter last-writer semantics; coherent multi-parameter snapshots during concurrent automation and transactional structural/session acceptance are not claimed by this increment.
- Keep implementation in the existing small adapter owner rather than extracting another module. Dynamic host tests exercise the actual exported release binary, including independently authored input state and short/failing streams. Existing normal CTest registration owns these boundary tests. Keep parser rejection cases in a table under one contract.

## Risks / Trade-offs

- JSON bytes are untrusted host data → bound input and validate completely; no filesystem or arbitrary patch loading.
- A round trip can hide matching writer/reader bugs → independently assert exported schema values, load reversed independently authored named controls and check signed output against known fixtures.
- No display or native DAW currently available → retain explicit pending UI/DAW gates; this slice establishes state API behavior only.
- Stream callbacks can stall → byte storage/read requests are bounded; host callback execution time is a host responsibility.

## Migration Plan

Characterize stopped flush, write missing-extension RED, implement minimal save/load, prove invalid-state recovery and stream behavior, widen real-binary/validator/build/spec gates, independently review, then publish the verified slice. Retain existing comparison and warm caches. Beta platform/DAW selection remains pending user input.
