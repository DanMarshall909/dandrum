## Why

React provides productive UI development but each open embedded editor has substantial resident-memory cost. We need runnable experiments to evaluate whether Slint or JIVE can provide comparably practical instrument UI development with native rendering.

## What Changes

- Add two isolated stereo filter-effect plugin spikes with Standalone and VST3 builds, driven by the same existing Rust filter graph and host parameter contract.
- Exercise graphically demanding UI: draggable logarithmic response graph, live FFT spectrum, scrolling spectrogram, animated meters and custom rotary controls.
- Register both experiments with the maintained demo launcher and preserve existing demo/build behaviour.
- Record reproducible build, audio, interaction, instance-lifetime and resource evidence plus an explicit adopt/revise/defer result.

## Capabilities

### New Capabilities

- `native-filter-ui-spikes`: Comparable, runnable native filter UI experiments and bounded evaluation evidence.

### Modified Capabilities

None. This experiment does not complete or replace the existing renderer-independent production UI plan.

## Impact

Optional spike sources and dependencies, root CMake registration, demo catalog/launcher/tests and documentation. Production Rust DSP and existing instrument editors remain unchanged. Slint and JIVE are pinned; dependency acquisition is opt-in with the spike build.
