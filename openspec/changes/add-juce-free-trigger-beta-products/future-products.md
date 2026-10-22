# Deferred product increments

These are later increments, not first-beta tasks. Do not start VST3 or standalone implementation during the CLAP beta. No deferred task is marked complete by removing it from the current task list.

## VST3 preference

Prefer a direct Rust VST3 adapter using `coupler-rs/vst3-rs` over the existing format-neutral `TriggerHost`. Keep the current CLAP adapter. Reuse the engine, persistent session/history, capacity-one runtime activation/reclamation, bounded event handoffs, parameter/state machinery and existing React/Wry editor. Keep VST3 ABI and host-loop types at the adapter edge.

The `vst3-rs` gain example is ABI reference code, not a complete production implementation. Do not replace the architecture with a broader plugin framework merely to obtain VST3 support.

## First gate of the later VST3 increment

Begin with one bounded feasibility proof, before production integration. Set and record a finite execution/resource budget, pin/audit actual sources and licences, and use one minimal adapter/loader configuration. The proof must cover:

1. Actual plugin discovery and deliberate Trigger identity/resource resolution.
2. Actual signed audio and MIDI through TriggerHost, with independent waveform/note oracles.
3. Sample-offset automation, including nondefault public values and exact timing.
4. Fresh-instance state restoration through the shared patch/sample/parameter authority, including failed-state preservation.
5. The existing Wry editor's X11 parenting, host main-thread/main-loop lifecycle, failure propagation and destruction/recreation without destroying the session.

Record supported evidence and concrete gaps, then choose adopt, repair, fallback or defer before dependent integration. Neither successful compilation nor the gain example alone proves these gates. Do not silently turn unresolved lifecycle/automation behavior into a production guarantee.

Keep `clap-wrapper` as a fallback only if concrete integration evidence warrants it. Previous cached source/SDK/compiler investigations remain historical inputs, not an adopted dependency or an instruction to build the wrapper now. Apply the same loader/audio/automation/state/editor gates to any fallback.

## Standalone later increment

Standalone audio/MIDI device ownership, application main-loop ownership, demo registration and native patch Save/Load belong to a separate later OpenSpec increment. It should reuse the same instrument/session and portable state. No standalone implementation, device API adoption or standalone artifact is required for the first CLAP beta.
