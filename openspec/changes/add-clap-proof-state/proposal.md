## Why

The verified JUCE-free CLAP proof cannot persist its controls in a host project. The accepted JUCE-free Trigger beta goal needs a replacement-owned state path before adding editor/session and format adapters; this increment proves that boundary on the existing two fixtures.

## What Changes

- Advertise CLAP state and save a bounded versioned envelope with built-in instrument identity and musical parameter names/values.
- Restore into a fresh instance before activation, validate the complete envelope before changing values, and retain working values after invalid input or stream failure.
- Prove host parameter flushing without running audio and preserve stopped changes into saved state and subsequent signed output.
- Retain existing deliberately experimental descriptor/numeric IDs and the JUCE comparison path.

## Capabilities

### New Capabilities

- `juce-free-clap-state`: bounded replacement-owned fixture parameter persistence, rejected-state recovery and stopped control updates.

### Modified Capabilities

None.

## Impact

Issue #24 J2/J3 prerequisite on `feat/juce-free-host-adapters`, using pinned Clack state support and existing serde dependencies. Changes affect the CLAP adapter, dynamic-host tests, dependency receipt and OpenSpec mapping. This does not implement Trigger documents/assets/history, choose the final product identity, replace #23's session, or claim native DAW/editor/VST3/standalone completion. Mutation testing remains excluded by the recorded user direction.
