# React editor performance, 7 October 2026

TB-303 control command latency fell from **69/135 ms median/p95 to 2/2 ms** in the embedded Linux WebKit editor. Idle CPU fell from **103% to 6.4% of one logical core**. The sampler's idle CPU fell from **34% to 4.9%**; its active CPU and control latency remain similar. WebView memory remains substantial.

| Embedded editor | Idle CPU before → after | Active CPU before → after | Idle PSS before → after | Active PSS before → after |
| --- | --- | --- | --- | --- |
| TB-303 | 103.1% → 6.4% | 106.9% → 42.5% | 277.5 → 266.5 MiB | 288.9 → 295.5 MiB |
| Sampler | 33.9% → 4.9% | 97.1% → 92.4% | 297.7 → 255.1 MiB | 310.0 → 287.7 MiB |

CPU percentages include the processor and its WebKit process tree; 100% means one logical core. Active phases include audio rendering and a requested 30 keyboard events per second. TB-303 previously admitted only **19 writes in ten seconds**; the optimized editor admitted **296**, with no rejections. Sampler admitted all offered writes: 297 before and 297 after. The table therefore also reflects increased useful work in TB-303.

| Keyboard handler → native write acknowledgement | Before median / p95 | After median / p95 |
| --- | --- | --- |
| TB-303, 60 changes | 69 / 135 ms | 2 / 2 ms |
| Sampler, 60 changes | 5 / 7 ms | 5 / 7 ms |

Sampler runs earlier in the session ranged from 4–6 ms median and 9–15 ms p95, and 94–99% of one core before versus 92–97% after during active use. The active figures overlap across runs; this report establishes the idle improvement. These measurements do not establish a sampler latency improvement. They measure command admission through the actual React component and JUCE bridge, using dispatched keyboard events; they do not measure physical input, screen presentation or sound at the speakers.

The change removes whole-panel scaling, keeps the centered TB-303 frame at its measured unscaled height, shares bounded asynchronous parameter refresh, suppresses unchanged parameter/meter reconciliation, and retires completed prepared-analysis timers. It retains the panel's shadows, fonts, gesture ordering, host automation and generation checks. The interactive knob is below 200 lines; its drawing component is shared separately.

The height observer matters: the initial responsive implementation improved idle CPU but still took about 70 ms per control command. Diagnostic overrides in [ablation](ablation/) isolated shadow repainting and automatic frame sizing: removing all shadows produced 5/8 ms, while fixing the frame height produced 2/3 ms. The implementation updates the frame only when its panel geometry changes, preserving the appearance and supported widths.

## Measurement method

The baseline is `fc3a681060c53efb7320ff5e26902d4f538e3990`. The optimized source is the candidate working tree based on integration commit `84f023c6ba8e712e9a8287744ebe5c1548447fdc`; [artifact identities](artifact-identities.json) identify the binaries and production bundles measured. Both probes instantiate the real processor, original embedded JUCE browser, bundled React assets and Rust engine. All four before/after cases were rerun after the queued-origin ownership repair and relinking both optimized probes. No CSS override or skipped bridge command is used in the final before/after runs.

Environment: AMD Ryzen 9 5900X, 24 logical CPUs, Pop!_OS 22.04, x86-64, GTK 3.24.33, WebKitGTK 2.50.4, Node 22.19.0, npm 10.9.3, remote X11 display `:10`. CMake build type is unset with JUCE's `NDEBUG` definitions; Rust is built in release mode and React uses production Vite bundles. This is the existing demo configuration, not a claim about an optimized C++ release build.

Each resource probe runs sequential engine-only, editor-idle, sounding/control-active and editor-closed phases. The observer samples `/proc` CPU ticks and `smaps_rollup` for the full process tree every 0.5 seconds, excluding the first two and final one seconds of each phase. PSS apportions shared pages; summing RSS would double-count them. The known 32 MiB allocation measured 32.01 MiB and a busy logical core measured 100.00% in [calibration](calibration.json).

The audio harness renders offline at 48 kHz with 64/128/256/512-frame blocks. At 128 frames the TB-303 median render took 0.040 ms and sampler 0.019 ms after the change. Both report zero added host latency; their first nonzero samples retain their baseline positions. Audio devices, DAW scheduling, hardware buffer latency, Xorg/compositor/GPU memory and multiple plugin instances are outside this measurement. Background machine activity and remote rendering can affect timing; these figures apply to this session and platform.

Closing the editor reduced process-tree PSS to about 62–65 MiB in the optimized builds; an engine-only process used 17–19 MiB. Closing does not unload every native/WebKit library and cache.

## Verification and limits

- Both React type checks, production bundles, embedded standalone/test targets and both VST3 packages build successfully.
- [149 JavaScript checks](validation/javascript.log) and [44 selected CTest checks](validation/ctest.log) pass, including native publication suppression, four native layout widths, real waveform/spectrum transport completion, live renderer/stalled replies, assets and demo inventory. The spec-map acceptance entrypoint additionally runs 34 of the component/helper tests in separate Node workers.
- [Focused coverage](validation/focused-coverage.log) is 100% lines/functions for the new controller, hook, drawing, reconciliation, height and polling modules. All have 100% branch coverage except the parameter controller at 98.41%. The existing gesture serializer has 100% lines/functions and 86.44% branches. Existing HostKnob coverage is 97.62% lines; no whole-editor coverage claim is made.
- [31 selected faults](validation/selected-mutations.json) are caught by helper/component tests. This is a focused mutation pilot, not a repository-wide mutation score; end-to-end tests are excluded.
- Completion review exposed a late rejected gesture closure reclaiming a newer drag. The [composed RED](validation/knob-late-rejection-red.log) fails at 0.75 versus 0.85; the repaired [GREEN](validation/knob-late-rejection-green.log) and two selected ownership faults prove directly submitted command failures retain their submitting interaction. The queued-origin [RED](validation/knob-queued-owner-red.log) and [GREEN](validation/knob-queued-owner-green.log), serializer ordering assertions and six further selected faults prove ownership is retained before any wait, through coalescing, and consumed before native invocation.
- Native layout verifies 760, 820, 1180 and 1500px widths, unscaled readable controls and synchronized frame height. At short heights the footer/hint can require vertical scrolling.
- **Native pointer verification remains incomplete.** `sampler-web-runtime` and `web-knob-parity` fail waiting for trusted mouse movement in this remote desktop. A one-off parity diagnostic confirms an enabled knob, valid pointer target and `pointer:null` before the first action; its [patch](validation/pointer-diagnostic.patch) only adds diagnostics and locates the fixture for the temporary executable. The sampler failure also reproduces with the original cached production assets and processor, explicitly selecting its sampler factory. Keyboard, wheel, typed values and host automation pass before that wait; component tests cover drag ownership, rejection, cancellation and late snapshots. No production workaround or weakened pointer assertion was introduced. See [candidate pointer log](validation/pointer-candidate.log), [diagnostic log](validation/pointer-diagnostic.log) and [original pointer log](validation/pointer-original.log).

Readable validation transcripts trim terminal padding for Git whitespace checks; [raw log archive](validation/raw-log-archive.json) preserves byte-exact originals of every affected log with hashes and base64.

[Summary JSON](summary.json), raw resource samples/logs, latency traces, source probes and exact compile/link arguments are retained here. These are archived one-off measurement tools: their recorded absolute worktree/build paths must be adjusted for another checkout, and a before build requires the baseline source/assets. `build_probes.py` records how the optimized probes were linked against the native build; `calibrate.py` records observer calibration. Use the maintained `./demo react303` and `./demo react-sampler` launchers to run the optimized editors.
