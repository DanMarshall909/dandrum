## Context

Trigger has a typed editor adapter, reusable pure domain edits and reversible store. JUCE already owns generation-checked notes, parameter gestures, waveform/meter services and asynchronous YAML replacement. The real engine exposes sample-map playback, while the mock also describes future voice, modulation and effects capabilities.

## Goals / Non-Goals

**Goals:** Run the full shell inside JUCE with actual sample playback, sample edits and native feedback. Preserve failed edits and generation boundaries. Keep the silent browser preview usable.

**Non-Goals:** Implement new DSP primitives, pretend every mock capability exists, or modify vendored JUCE.

## Decisions

- Compile accepted sample DTO edits to immutable sample-map YAML using pure JavaScript code; native preparation performs actual validation and activation.
- Extend the existing asynchronous reload worker to own YAML snapshots. No JSON/YAML parsing, allocation, filesystem work or renderer calls enter the audio callback.
- Native adapter owns serialized preparation, polling and real feedback. Domain reducers remain independent of React and MockEngine.
- Embed the built shell and fonts through JUCE BinaryData and register a separate maintained `react-trigger` demo. Preserve the smaller React sampler.
- Capability errors prevent unsupported operations from modifying accepted state.
- Work in primary checkout/main under the user's explicit worktree override. The formatting task has been integrated and its shared ledger resolved.

## Risks / Trade-offs

- The rich shell exposes more capabilities than the current engine. Report rejected operations explicitly and document supported audio behavior.
- Host replacement may retire jobs, notes or parameter generations. Reject stale replies and stop timers/listeners on closure.
- Physical device availability in WSL may differ from automated rendering. Verify actual host rendering independently of window visibility.
