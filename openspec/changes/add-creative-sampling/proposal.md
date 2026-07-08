## Why

Creative sampling features such as realtime time-stretching, pitch-shifting with better formant behaviour, granular playback, scrub position, freeze, and spectral-style manipulation are useful, but they have different DSP and realtime constraints from drum-machine/chromatic sample playback.

They should be specified separately so the base sampler remains primitive-first and predictable.

## What Changes

- Add a future `creative-sampling` capability for time-stretch, granular, scrub/freeze, and other creative sample manipulation primitives.
- Keep creative sampling separate from prepared one-shot/zone/slice playback.
- Build creative features as focused primitives that can be composed into modules rather than as a single opaque mega-sampler.

## Capabilities

### New Capabilities

- `creative-sampling`: Future capability for realtime sample transformation primitives such as time-stretch, granular playback, scrub/freeze, and spectral-style manipulation.

### Modified Capabilities

- `advanced-sampling-options`: Remains focused on prepared region playback, explicit slicing, modest looping, zone selection, and choke behaviour.
- `built-in-modules`: May later include creative sample manipulation primitives.

## Impact

- Requires separate DSP design and tests for latency, phase continuity, windowing, interpolation, modulation, and realtime allocation behaviour.
- Non-goal: do not include granular/time-stretch behaviour in the first advanced sampling implementation.
