# Filter UI resource experiment — 2026-10-07

The same native harness, Rust stereo filter, analysis model, 48 kHz/64-frame paced worker, 900×700 editor size and 30 Hz UI updates were used for JIVE 1.2.0 and Slint 1.18.1 software rendering. Linux 7.0.11, AMD Ryzen 9 5900X, Rust 1.96, GCC 11.4, CMake 3.30.9 and JUCE 8.0.6; remote X11 at scale 1. Exact source and binary hashes are in [environment.json](environment.json).

## Results

| Backend | Editors | Audio | PSS median MiB | CPU mean, one core | Raster median ms | Raster p95 ms |
|---|---:|---|---:|---:|---:|---:|
| JIVE | 1 | silent | 28.7 | 12.2% | 2.55 | 3.10 |
| JIVE | 1 | active | 28.7 | 12.5% | 2.31 | 3.46 |
| JIVE | 2 | silent | 33.7 | 26.4% | 2.53 | 2.90 |
| JIVE | 2 | active | 33.2 | 25.1% | 2.34 | 3.32 |
| Slint | 1 | silent | 31.1 | 17.7% | 3.09 | 3.89 |
| Slint | 1 | active | 31.1 | 18.4% | 2.73 | 4.17 |
| Slint | 2 | silent | 38.5 | 37.2% | 3.19 | 3.76 |
| Slint | 2 | active | 38.7 | 32.3% | 2.87 | 4.26 |

For two editors, raster columns describe the first editor's median and the maximum per-editor p95. Memory and CPU cover the complete process. No empty-host subtraction was performed. Extra-editor active PSS was roughly 4.5 MiB for JIVE and 7.6 MiB for Slint in this harness. Those are two-point incremental estimates, not DAW instance budgets. All measured process swap-PSS samples were zero; minimum system available memory was 15,226 MiB, above the 4096 MiB reserve.

JIVE had lower process CPU and raster cost in this implementation. Slint's software adapter includes full-frame rendering and RGB-to-JUCE image copying. Their styles and plot dimensions differ; these are comparable feature workloads, not pixel-identical scenes. Both continue repainting and FFT/history processing at 30 Hz during silence, so neither is an idle-optimised production editor.

## Method and reproduction

Eight sequential fresh processes covered 1/2 editors × silent/active audio × both backends. Each announced a 20-second steady phase. A separate observer sampled `/proc/<pid>/smaps_rollup` and CPU ticks every approximately 0.5 s; summaries retain 32 samples from phase seconds 2..18. PSS is proportional shared memory; CPU is percentage of one core. Active input used distinct 440/880 Hz stereo sines and opposite-sign noise. Numeric spectrum/history uses the mono fold, so the opposite-sign noise cancels there; channel meters/audio still use stereo.

After the steady phase, each editor performed 200 full offscreen `createComponentSnapshot` paints at 900×700. These record raster cost, exclude UI timer pumping between samples, and are not physical input or screen-presentation latency. The audio worker continued. Its scheduling-overrun counts were 0..9; these are paced-thread wake-up observations, not audio-device xrun measurements. No device underrun claim is made.

Build the targets using the [experiment instructions](../../../spikes/filter-ui/README.md), then run:

```bash
build/filter-ui-spikes/dandrum-filter-jive-ui-check /tmp/filter-jive-active --measure active 1
build/filter-ui-spikes/dandrum-filter-slint-ui-check /tmp/filter-slint-idle --measure idle 2
```

Use an available X11 display. Every run writes `metrics.json` and PNGs; stdout is NDJSON with measuring/pass/fail records. [collector.py.txt](collector.py.txt) preserves the exact one-off observer used here, including its absolute workspace/output paths. [comparison.json](comparison.json) contains all 256 raw process samples, per-editor 200-paint distributions, metrics, commands, images' filenames and exit codes. The retained launcher-suite text log removes trailing whitespace; its original and the full disposable run logs/PNGs remain at `/tmp/dandrum-filter-spike-evidence`; warm native builds remain intact.

## Verification

[Canonical CTest output](verification/ctest-visible.log) records 9/9 owning checks. The actual VST3 host loads both bundles, proves signed output/state/bypass/nine IDs, opens two editors, checks independent host state and control-region pixel changes, closes/reopens and resizes. Linux captures read only task-owned native windows, including embedded children. Linked-source editor checks additionally verify drag/callback gestures, layout bounds, Slint About and 900×700/1080×760 images. Physical OS input and other platforms remain outside this evidence.

[Python suite](verification/python-all.log) records 34/34 tests. The maintained launcher's [verification record](verification/launcher-verification.json) retains RED failures, 114/114 executable-line coverage (branch coverage was not collected), real CMake/JUCE boundary calibration and five rejected deliberate faults. The [live launcher record](verification/live-launcher.json) confirms both actual Standalone effect windows launched from `./demo`, then closed gracefully with exit 0 and no remaining owned processes. Their default input was muted; physical audible playback was not verified. [Oversized-block RED](verification/oversize-red.log) guards whole-callback meter peaks; the final processor CTest passes. Strict [OpenSpec validation](verification/spec-validation.log) and the existing [spec-coverage gate](verification/spec-coverage.log) pass; the baseline ratchet backlog remains 239 and is unrelated to this experimental delta.

## Decision and implementation findings

Adopt JIVE for the next native UI experiment; revise Slint's custom embedding/backend before production adoption; defer production migration for both. JIVE reuses JUCE's component/event model and has no extra platform boundary. Its declarative C++ ValueTree syntax reduces layout work, while custom graphics still require C++. Slint places more visuals and interaction in a typed declarative language, at the cost of custom C++ platform/presentation/input code in this host.

Both UI agents produced initial source in roughly 15 minutes, before integrating builds. This is not a controlled productivity benchmark. Integration exposed JIVE's required `JIVE_IS_PLUGIN_PROJECT` compile definition and explicit startup size, unsupported CSS gap authoring, and 8-bit String conversion for Unicode labels. Slint exposed an updated image API, nested-conditional generated-C++ precedence, persistent repeater identity during dragging, and fixed Window bindings causing cropped viewport rendering. Each fault was repaired and owning checks rerun. Cold Slint runtime/compiler builds added minutes and dependencies; the cache is retained for future work.

Slint's software renderer has restricted shadows/transforms/gradient Paths, and this adapter has no demonstrated HiDPI or complete keyboard/IME/accessibility bridge. No Slint GPU renderer, general DAW/platform deployment or migration of production UIs was evaluated. Source evidence proves this bounded experiment, not universal framework performance or readiness. Renderer details and pinned upstream licence/attribution links remain in the respective [Slint](../../../spikes/filter-ui/slint/NOTES.md) and [JIVE](../../../spikes/filter-ui/jive/DEVELOPMENT.md) notes.
