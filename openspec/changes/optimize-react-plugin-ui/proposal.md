## Why

Measured embedded TB-303 control delays and idle CPU are dominated by whole-panel scaling and repeated processing of unchanged state. Shared parameter commands also wait for redundant snapshots, reducing drag throughput.

## What Changes

- Fit TB-303 with responsive CSS instead of scaling its entire panel.
- Share bounded, asynchronous parameter refresh and retain unchanged state identities in both React editors.
- Publish native parameter snapshots only when needed and preserve meter display identity when visible values are unchanged.
- Stop prepared-analysis timers after terminal results.
- Rebuild embedded assets and measure the same native editor workloads before and after.

## Capabilities

### New Capabilities

- `react-plugin-efficiency`: Responsive, bounded editor updates without redundant idle work or snapshot delays in control commands.

### Modified Capabilities

None. Existing renderer, gesture, generation, telemetry and audio contracts remain applicable.

## Impact

TB-303 and sampler React entrypoints, shared UI helpers, native web bridge, focused JavaScript/C++ tests and embedded production assets. No audio engine changes or new runtime dependencies.
