## Why

Issue #24 needs an actual exported JUCE-free plugin proof around the retained Rust engine. The independent build and owned thread-transfer prerequisites now pass, but neither proves format discovery, sample-offset automation or host lifecycle.

## What Changes

- Add a pinned, opt-in experimental headless CLAP adapter with deliberately distinct TB-303 and signed stereo gain-fixture descriptors.
- Load the exported `.clap` through a real dynamic Clack host consumer and validate discovery, literal signed engine output, public root-parameter delivery, MIDI timing/order, reset/reactivation and two independent instances.
- Declare MIDI 1 input only for the instrument; do not advertise CLAP note IDs/expression, modulation, GUI, device management or unsupported capabilities.
- Retain default JUCE comparison products and all remaining state/editor/format/device migration gates. Record exact build/dependency and validator limits.

## Capabilities

### New Capabilities

- `juce-free-clap-proof`: dynamically loadable experimental CLAP engine consumers with signed rendering, sample-offset controls/events and independent host lifecycle.

### Modified Capabilities

None.

## Impact

New isolated Rust adapter/test crate, pinned Clack plugin/extensions and a dynamically loading dev host, opt-in independent CMake packaging/test registration, focused acceptance map and migration receipt. The core remains independent of plugin types and dependencies. No new authoring session, journal or musical features; this bounded rendering proof is not complete J2 or a production stack/cutover decision.
