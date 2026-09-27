## Why

The plugin loads its default Patch before the host supplies audio settings. The engine currently leaves that graph at 44.1 kHz when prepared at 48 or 96 kHz, producing different audio depending on load order; later preparation changes also leave graph resources stale.

## What Changes

- Apply host sample rate and maximum block size to an already loaded instrument during preparation, outside the audio callback.
- Preserve current parameter values and resolved automation targets across preparation while resetting transient DSP and queued events for the new processing session.
- Specify and test load order, repeated preparation, state restoration and failed replacement recovery through Rust and plugin boundaries.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `plugin-integration`: Instrument preparation uses current host settings regardless of loading order, preserving the loaded parameter surface and values.

## Impact

Rust engine preparation and graph lifecycle, focused Rust and native regression tests, and plugin integration specifications. No DSP algorithms, parameter IDs, automation-slot assignments, fixture event timing, asset resolution, or host/UI extraction changes. Local verified commits only; no merge or push under this goal.
