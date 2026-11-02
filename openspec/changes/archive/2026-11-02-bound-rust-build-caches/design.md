## Context

The repository has five independent Rust packages, per-CMake targets, per-package targets and instrumented targets. A measured comparison found 5.86 GiB of identical duplicate file content. More significantly, repeated source-free snapshots used UUID paths while retaining their Cargo cache: first-party package identities accumulated across runs. The largest target contains 104 GiB, including 92.5 GiB of debug output. The primary engine target is an existing warm baseline.

Dan explicitly accepted consolidation and retirement of obsolete caches while preserving fast turnaround. This maintenance change is separate from the pending sampler dependency PR and beta tasks 7.3/8.1/8.2.

Dan subsequently selected a single primary checkout with no additional Git worktrees. Branches switch in place, implementation/builds are serialized, and independent review reads the same checkout. The stable disposable source-free fixture remains necessary test input. One checkout reduces duplication; the cache budget still controls compiler/dependency generations, product variants and coverage storage within it.

## Goals / Non-Goals

**Goals:** reuse compatible dependencies, retain incremental development, keep final products correctly bound, remove recurring source-path churn, bound managed storage, preserve replay evidence and demonstrate actual warm/small-edit reuse.

**Non-Goals:** change Rust dependencies, introduce a Cargo workspace, reduce optimization/debugging quality, install another compiler cache, complete beta acceptance, or promise that arbitrary additional configurations fit a fixed budget without rebuilding.

## Decisions

1. A repository driver owns shared ordinary intermediates via Cargo's stable `CARGO_BUILD_BUILD_DIR`. Cargo target directories remain private to each output owner. Hooks, CMake, coverage and documented developer commands use the driver. Instrumented builds have their own target/build directories, ignore an exported ordinary target and refuse explicit coverage target overrides. The existing opt-in mutation registration uses raw Cargo with separate output and no inherited ordinary build directory; mutation remains excluded from this delivery. Cargo continues to decide compatibility and freshness; fingerprints are never rewritten.
2. A cache generation is bound to the compiler and the repository's resolved dependency inputs, not source commits. Normal source edits keep the current generation and incremental state. The current primary engine cache can be explicitly adopted at its existing path after validating ownership; future generations use a dedicated common-Git cache directory. A repository lease covers execution and any artifact selection/copy, and protects unfinished producers. Cleanup removes only whole owned inactive generations, never guessed Cargo-internal file sets.
3. The existing retention and free-space policy controls measured admission. Eligible obsolete generations are retired before admitting new output. If current warm graphs plus the configured growth allowance do not fit, the driver refuses the new producer and reports the conflict. Completion measures actual growth and reports an overrun; subsequent growth is blocked. This is an admission/retention contract, not an instantaneous filesystem quota. Interrupted/failed commands preserve their cache and evidence. Raw Cargo commands outside the maintained driver are explicitly unmanaged.
4. Source-free fixtures use an exclusively leased stable source path, reconstructing the source and CMake scratch contents completely before each run. They retain all actual source-hidden build/install/runtime assertions. Fixture cleanup removes only the owned source scratch. Native publication retains the current resolved fixture and required evidence, and retires superseded owned stages under the same serialization boundary.
5. CMake invokes Cargo's freshness check for requested Rust products, including lockfile changes. Headless and editor CLAP products use separate private outputs. A shared cache lease extends through the native artifact copy; timestamps alone cannot select a product variant.
6. Migration first seals exact installed/replay binaries, current native fixtures, historical binding records, and coverage profiles with their matching objects. The existing warm primary target is retained. Retired targets are recorded by path, measured size and disposition; current warm tests and one small source edit are timed before claiming the fast path survived.

## Risks / Trade-offs

- A new compiler/dependency graph may need a cold build. Keep ordinary source edits in one generation and report actual reseed cost.
- A current graph may outgrow the budget. Preserve it and fail admission with a useful diagnostic; do not silently wipe hot state.
- Shared mutable intermediates require serialization. Keep the existing one-heavy-producer policy and a lease spanning publication.
- This filesystem uses `noatime`; age-based tools can misclassify frequently reused dependencies. Do not use access-time eviction or unmaintained cargo-sweep.
- Relocated outputs can contain absolute paths. Treat migration as measured reuse, not proof that all old objects remain warm.
- Additional worktrees may introduce distinct package paths. Retire inactive generations and enforce admission; do not mirror all source trees or disguise path identities.

## Migration Plan

Retain the existing primary target; seal legacy evidence; retire obsolete retained targets; adopt and exercise the managed normal/coverage paths; rewire callers; verify fresh/unchanged/edited behavior and real source-free/native boundaries. Source/config rollback is by Git; removed caches are rebuildable, while pinned replay evidence remains intact.

## Qualification And Migration Evidence

On 2026-11-02 the primary warm target was adopted in place. An unchanged managed engine test build reused all 141 artifacts in 0.293 seconds (raw Cargo baseline 0.078 seconds). Real tiny-package tests also proved source-edit dependency reuse, distinct feature output, freshness despite future output timestamps, and fresh coverage after source edits without polluting ordinary artifacts. Source-free build/install/runtime acceptance passed all 13 checks in 54.3 seconds on the warm repeat.

The combined engine-only/native CTest qualification passed all 69 selected checks in 292 seconds. The separately gated installed physical-audio overlap check remains outside this cache task. Qualification exposed a test-oracle defect: GTK/WebKit can retain its own hidden `GtkTooltipWindow`. The native ownership test now excludes only that toolkit type and explicitly proves that an extra application window is counted and released. Actual X11 parenting, callback, weak-reference and disposal assertions remain in place; production editor behavior did not change.

The retired targets yielded approximately 134.9 GiB of measured gross free-space recovery. Before retirement, 1,415 replay/evidence paths were sealed into 256 distinct SHA-256 blobs (1,865,545,967 bytes), read-back verified and reverified before deletion. The primary warm target, installed proof package and required frontend dependencies/bundle remain. Local receipts, exact original paths and the restoration manifest are retained under `.git/codex/cache-consolidation-20261102/`; removal used supported package cleanup followed by identity-checked retirement of the explicitly approved legacy roots.

Fast ownership/refusal tests are calibrated by actual Cargo, CMake, coverage and native-boundary checks. Changed-owner line tracing covered the native fixture helper completely; the cache inventory entry-limit fail-closed diagnostic remains a disclosed coverage gap. Refactoring review retained the small ownership, inventory and command-routing helpers without introducing another lifecycle abstraction. Mutation was excluded by the standing user instruction for this session.
