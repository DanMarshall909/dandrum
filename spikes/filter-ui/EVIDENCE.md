# Filter UI experiment evidence map

The proposed requirements live in [the experimental delta](../../openspec/changes/spike-native-filter-ui-frameworks/specs/native-filter-ui-spikes/spec.md). They have not been synchronized into production specifications. Retained logs are in the [resource report](../../docs/resource-measurements/2026-10-07-filter-ui-spikes/README.md); the existing main-spec coverage ratchet is unchanged.

| Criterion | Executable evidence and asserted outcome |
|---|---|
| Signed stereo filter output | `filter-spike-engine` and both `filter-*-vst3-check` tests compare signed stereo output with an independent RBJ impulse oracle; the engine test changes cutoff and bell gain. |
| Bypass and state restore | `filter-spike-processor` and both VST3 checks assert exact dry samples and equivalent restored filter output. |
| Graph and controls share host state | Both `filter-*-ui-check` tests exercise actual renderer graph/rotary callbacks, visible node positions, host parameter updates and balanced gestures. VST3 checks assert stable parameter IDs and changed rendered controls after host updates. |
| Actual audio drives visual analysis | `filter-spike-processor` asserts known-frequency spectrum peaks, history, meter peaks across oversized callbacks and a known 12 dB bell response. Both UI checks run a paced audio worker while rendering. |
| Independent editor lifetimes | Both UI and VST3 checks open two instances, change independent state, destroy/recreate an editor and assert a usable rendered result. |
| Optional spike launch | `demo-launcher`, `demo-launcher-inventory` and `demo-launcher-boundaries` assert isolated native-only configuration, no npm preparation and actual JUCE artifact paths. The live launcher record captures both launched effect windows. |
| Bounded evaluation evidence | Eight measurement processes emit pass records and JSON with commands, samples, frame distributions and exits. The report names the measured backend, scope, limits and explicit adopt/revise/defer decision. |

| Task | Implementation | Verification |
|---|---|---|
| 1.1 Optional pinned targets | Root CMake, `cmake/FilterUiDependencies.cmake`, `cmake/FilterUiTargets.cmake` | Both Standalone/VST3 builds; opt-in default; launcher boundary checks |
| 1.2 Shared filter/state | `filter.yaml`, `shared/FilterProcessor*`, `shared/FilterAudio.cpp` | Engine, processor and actual VST3 checks |
| 1.3 Capture/analysis/gestures | `shared/FilterViewModel*`, `shared/FilterAnalysis.cpp` | Processor and both UI checks; oversized-block RED log |
| 2.1 JIVE renderer | `jive/` | JIVE UI/VST3 checks and native screenshots |
| 2.2 Slint renderer | `slint/` | Slint UI/VST3 checks, About geometry and native screenshots |
| 2.3 Lifetimes/sizes | Both renderer factories and per-editor models | Two instances, close/reopen, 900×700, 1080×760 and VST3 1280×900 |
| 3.1 Maintained discovery | Launcher, registry, README and Python tests | Recorded RED/GREEN, 114 executable lines, five rejected faults, live launches |
| 3.2 Resources/frame cost | Native UI-check measurement mode and retained observer | Eight sequential runs; 256 process samples; 200 raster samples per editor |
| 3.3 Findings/handoff | Experiment README, renderer notes, resource report | Owning CTest/Python checks, strict OpenSpec validation, explicit outcome |

The isolated renderer experiment uses proportionate functional and output checks rather than production line-coverage or mutation claims. The maintained launcher has recorded executable-line coverage and fault calibration. Rust engine sources were unchanged; normal commit/push hooks still run their existing Rust gates. Physical input latency, device xruns, HiDPI, other operating systems and production deployment remain unverified.
