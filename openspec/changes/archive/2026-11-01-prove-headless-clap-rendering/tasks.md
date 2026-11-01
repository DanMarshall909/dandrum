## 1. Baseline and dynamic-host RED

- [x] 1.1 Add public engine baselines for signed stereo gain and timed TB-303 notes/root controls; run against the existing production engine and record a known signed TB-303 oracle before adapter implementation.
- [x] 1.2 Create isolated pinned dependency/test scaffolding and compatible lockfile using the existing task cache; dynamic-load the exported minimal stub and confirm expected missing-factory/behavior RED rather than an unrelated build failure.

## 2. Real engine format adapter

- [x] 2.1 Implement exported experimental descriptors, metadata/root controls and owned off-audio preparation; verify dynamic discovery and literal signed gain output without JUCE/browser/device dependencies.
- [x] 2.2 Preserve nonzero/shared-offset automation order and supported MIDI1 note behavior with preallocated rendering; verify signed segment/TB-303 parity and public root-mapping tests pass.
- [x] 2.3 Verify reset, changed-settings reactivation and two independent instances through dynamic host tests, retaining parameter values and off-render lifecycle.
- [x] 2.4 Add focused callback boundary guards for tested malformed bounds/values/layouts with deterministic fallback; verify no normal PluginError/panic path and review signed valid paths remain passing, without claiming full realtime hardening.

## 3. Independent packaging and evidence

- [x] 3.1 Add explicit default-OFF independent CMake plugin selection and actual .clap packaging/test path; test the build guard, affected default selection and dynamically loaded artifact with no JUCE/browser resources.
- [x] 3.2 Run the pinned official CLAP validator against the actual artifact without hidden exclusions and record complete scoped results/limitations, exact dependency/feature/attribution receipt and current physical build-volume capacity (C: on the source Windows host).
- [x] 3.3 Sync/map the four proven scenarios and run strict scoped/canonical OpenSpec, acceptance map and final diff review with no new todo entries.
- [x] 3.4 Commit/push the verified bounded proof with a detailed body, verify live exact head, and update progress/shared ledger retaining all remaining full #24 obligations.
