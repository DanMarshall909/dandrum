## Why

Separate Cargo targets and repeatedly changing verification source paths have accumulated more than 135 GiB of retained output. Dan has authorized consolidation and cleanup, with fast unchanged and small-edit turnaround preserved and further growth bounded.

## What Changes

- Reuse ordinary Rust intermediates through a repository-owned cache across maintained build, test and hook entry points, while preserving private final artifacts and separate coverage measurements.
- Keep verification snapshot paths stable and exclusively owned, and remove superseded native fixture output after preserving required evidence.
- Enforce the existing 64 GiB retention policy with measured admission, explicit ownership and inactive-generation retirement; preserve active warm state and report a capacity conflict when it cannot fit.
- Seal exact replay/coverage evidence, retire obsolete duplicate targets, and measure real warm and small-edit behavior before claiming completion.

## Capabilities

### New Capabilities

- `rust-build-cache`: Bounded, reusable Rust build storage with correct artifact ownership and safe cleanup.

### Modified Capabilities

None. Existing source-free engine and native artifact verification contracts remain in force.

## Impact

CMake Rust commands, Git hooks, Rust coverage entry points, native/source-free verification fixtures, development commands, and local generated-output retention. No DSP, plugin behavior, dependency versions, or compiler optimization changes are included. The unrelated sampler Vite PR #25 and unfinished beta acceptance remain separate work.
