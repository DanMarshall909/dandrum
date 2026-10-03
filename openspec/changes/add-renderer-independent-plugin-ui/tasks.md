## 1. Baseline And Design Reference

- [x] 1.1 Resolve the active-work gate and choose the integrated sampling base or explicit dependency on `3b313a7`; verify worktrees, upstreams, PRs and the repeated status snapshot before repository mutations.
- [x] 1.2 Import the reviewed design-system revision with its SHA-256 and provenance; verify archive integrity, local reference links and any installed skill frontmatter without executing bundled development tooling.
- [x] 1.3 Characterize current shared/per-pad parameters, host gestures, note admission and reload behaviour with focused C++/JavaScript tests; record a known signed sampler render and current coverage before production changes.
- [x] 1.4 Specify native KeyMap, LayerStack and OutputBusses dimensions, focus, inspection interactions and capability states; review the delivered native handoff against the new JSX/type references. The original external-only boundary is superseded by the automatic structural rebuild specification; implementation is tracked in section 9.

The design reference was imported on main at `d02ee1b` and is now available in `docs/design-system/reference/`. All 161 reference files match the recorded SHA-256 values. Native component specifications and production adaptations remain separate implementation tasks.

## 2. Shared State And Commands

- [x] 2.1 Add typed prepared UI documents and scoped capabilities; tests must prove actual 63/64 snare boundaries, parameter scopes, zone-selection semantics and absent capability reporting.
- [x] 2.2 Add the minimum read-only metadata access needed across FFI using retained/copy ownership off audio; lifetime tests must survive reload, released source inputs and a stalled reader without touching freed storage.
- [x] 2.3 Extract shared host command handling with generation and finite-value validation; run the same accepted/rejected command contract tests through native and Web adapters.
- [x] 2.4 Implement begin/update/end gestures and timer-observed parameter updates; tests must prove one drag equals one host gesture, keyboard/typed entry works, stale echoes are ignored and listeners do not post messages from audio.
- [x] 2.5 Add bounded editor note-release/session cleanup semantics; saturation tests must prove queued note-on plus release/disconnect cannot leave a gated note active or affect unrelated host MIDI.
- [x] 2.6 Expose accepted job IDs and queryable reload/analysis status; tests must prove early acceptance, failed reload preservation, reconnect recovery and stale-job rejection.

## 3. Two Renderer Foundations

- [x] 3.1 Split common UI, native editor and Web adapter build targets; verify a native-only configure/build/run with browser support disabled and without Node or WebView/WebKit dependencies.
- [x] 3.2 Introduce one maintained token source and CSS/C++ generation; verify corresponding semantic values and document font licensing/provenance.
- [x] 3.3 Build the production web asset pipeline and embedded resource serving; verify packaged assets work offline outside the source checkout, with no CDN scripts, runtime Babel or development server.
- [x] 3.4 Implement one design-system knob in each renderer against shared commands; tests must prove identical host gestures, authoritative values, slot identity and known signed audio for identical schedules.

Web progress on 3.4 (2026-10-03): both React apps now use one reference-derived knob, admitted actual ranges and loaded defaults, typed popup values, pointer/keyboard/wheel gestures and generation-safe metadata requests. Original factory/WebKit tests verify eleven balanced gestures on the stable host slot, actual-range readouts and host updates at both sizes. The sampler additionally proves release reconciliation after automation while held; a deterministic component end/write barrier proves an older closure cannot overwrite newer local input. Copied complete executables pass with hidden checkouts, fresh application data and no network route. Full Web CTest passes 42/42 and native-only CTest passes 18/18. Focused policy suites execute all V8 ranges; the real React component lane executes all 42 knob functions, with three defensive null fallback fragments still uncovered. Seven copied-source faults are rejected by named assertions. [Knob integration evidence](../../../docs/design-system/react-knob-integration.md) records test boundaries and remaining scope. Native drawing and equivalent signed-audio schedules remain pending, so 3.4 stays unchecked. No structural runtime completion is claimed.

Verification for 1.4 and 3.2: `docs/design-system/native-display-specs.md` records source dimensions, focus, release behavior, capability boundaries and absent capability states for the three displays. It explicitly addresses local mutation still present in reference LayerStack/OutputBusses and the reference KeyMap's simulated playback flashes. `ui/design-system/tokens.json` generates all 146 CSS/C++ values from one maintained source. `design-token-generation` changes colour/spacing tokens and alias chains, compiles fixture and production C++ headers, checks deterministic output and invalid-source preservation, and verifies pinned unmodified font/license hashes. `design-tokens-current` was calibrated through CTest by changing a native colour: it rejected `DesignTokens.h` by name; restoring it passed. The generator's focused V8 run covered all 89 executable source lines. Full Web CTest passed 36/36. React font embedding and offline WebKit asset/adapter inspection are now verified below. Native typography, generated colour/spacing use and actual component adaptation remain open. That evidence does not prove automatic structural rebuild behavior.

Font progress on 3.3 at `8845af1` (2026-10-02): both React bundles inline eight pinned TTF faces into CSS and retain the three unchanged OFL notices in JavaScript. `react-asset-packaging` checks font bytes, weights, notices and the three embedded routes; its weight fault was rejected by name. Linux WebKit asset/adapter checks load all faces, obtain the actual 7/23 host surfaces, observe a processor value of 0.37 and inspect 1200×800/820×560 layouts. Copied executables passed with both checkouts hidden, fresh application-data directories and an isolated network namespace. A compact TB-303 overlap regression failed before the frame used the rendered panel height and passed afterward. That slice passed full Web CTest 39/39 without skips and native-only CTest 18/18. [Packaging evidence](../../../docs/design-system/react-font-packaging.md) defines the harness boundary and platform limits. Supplied SVG icon integration was left open for the follow-up below. No automatic structural rebuild implementation is claimed.

Icon progress on 3.3 at `147eaa4`: both React apps use the supplied stroke icons on existing labelled views. `design-icon-rendering` checks all 33 names and SVG shapes through React's static renderer, plus decorative/labelled accessibility, colour inheritance and all supplied sizes. A changed `level` path was rejected by name; the shared icon module's executable ranges are covered. `react-asset-packaging` verifies required geometry is compiled into both apps without extra routes. Linux WebKit verifies visible icons at both sizes with their correct identities, view box, size, stroke, accessible exclusion and non-focusability. That slice passed Web CTest 40/40 and native-only CTest 18/18. Its separate browser fixture left the original-editor offline gate open for the verification below. Native icon use and supplied component fidelity remain open.

Completion of 3.3 (2026-10-03): the two runtime checks now use the shipped processor and editor factories and show the original editor/browser, with no extra browser, native report function or manual host-state publication. They verify all eight fonts, supplied icons, actual 7/23 controls and exact full/compact viewport dimensions. Starting with a host value of 0.21, they observe 0.37 at full size and a fresh 0.63 update at compact size through the normal editor timer. Temporarily stopping that timer leaves fonts loaded but fails the named full-size host-update assertion; restoring the source byte for byte and rebuilding passes. Copied complete executables passed outside both hidden checkouts, with fresh redirected application data and no network route; full/compact screenshots were inspected. Web CTest passes 40/40 without skips and native-only CTest passes 18/18. [Packaging evidence](../../../docs/design-system/react-font-packaging.md) states that these are original factory/editor checks in a JUCE test application, not VST3 loading inside a DAW or complete lifecycle/stress proof. Tasks 3.4, 7.5 and 8 retain their own component, layout, host and timing gates. No structural runtime implementation is claimed.

Reset progress on 3.4 (2026-10-03): copied prepared parameters and their Web
serialization now include `normalisedDefaultValue`, derived from the loaded
descriptor's default and range independently of the current value. The native
primary control uses it for double-click reset through the existing host gesture
path. `cxx-plugin-editor-bridge` checks declared kick, cutoff and sample-group
defaults and proves ordinary edits do not overwrite them; `cxx-sampler-plugin-host`
checks ownership across sampler-to-synth reload. `native-editor-smoke` dispatches
the real control's double-click handler, checks one gesture and the stable slot,
and asserts signed output at defaults 0.5 and 0.2 after a reload. Retained
documents keep their old default, and unavailable instruments cannot reset.
Fixed host-slot defaults and objects remain unchanged. This is the reset binding
prerequisite: faithful knob geometry, typography, popup/value entry, complete
input mappings, Web reset interaction and renderer parity remain open, so 3.4
remains unchecked. Full Web/native builds pass, with CTest 40/40 and 18/18
without skips. Focused gcov runs execute all 12 added executable C++ source
lines; this is not whole-module or exhaustive branch coverage. Separate source
copies using a fixed 0.5, the host-slot default or the current live value fail
their named reset assertions, and healthy reruns pass. No Rust, FFI, asset or
host parameter layout change is included.

Native progress on 3.4 (2026-10-03): the primary control uses the supplied
64-pixel cap/arc, embedded caption/value faces, actual-value popup and the React
input mappings. Tests prove loaded defaults, precision-preserving entry,
automation reconciliation, unavailable ranges, reload and gesture closure.
Synchronous host-listener teardown is protected by component lifetime checks
and shared gesture ownership handoff; focused ASan catches the original defect
and passes the repair. Native CTest passes 19/19. The
[knob integration record](../../../docs/design-system/react-knob-integration.md)
states coverage gaps, eleven focused faults, physical input evidence and limits.
The paired signed-audio proof was pending at this increment. No structural
runtime completion is claimed.

Paired completion of 3.4 (2026-10-03): `native-knob-parity` and
`web-knob-parity` compile one schedule with eleven points into both original sampler
factory paths. Real native/TextEditor and shipped React handlers produce the
same ordered host events, authoritative values, stable slot/object and exact
signed stereo PCM at 64-frame boundaries. A wrong signed-source fixture fails
the named frame-128 assertion in both renderers; healthy recovery passes.
Full native/Web builds pass and CTest passes 20/20 and 44/44 without skips.
The integration record retains synthetic-input/DAW limits and earlier coverage
gaps. Structural runtime and other tasks remain pending.

## 4. Meter Telemetry Vertical Slice

- [x] 4.1 Prepare bounded master/output meter capture with a single consumer per queue; saturation and callback instrumentation must prove bounded memory/work, no forbidden callback operations and unchanged signed PCM.
- [x] 4.2 Implement sample-weighted peak/RMS aggregation and independent clip latching; tests must cover signed stereo, unequal blocks, silence, dropped history and generation/reset races.
- [x] 4.3 Add subscription lifetime, sequence/generation handling and bounded Web acknowledgements; tests must prove hidden/stalled/reopened editors cannot grow browser message queues or block audio.
- [x] 4.4 Render the same meter data in native and WebView components; deterministic view-model tests and actual runtime inspection must verify levels, clipping, visibility and timing-based decay.

Verification: `cxx-plugin-meter-display` checks elapsed-time peak/RMS decay, identity resets and clip state; `cxx-plugin-construction` checks processor delivery during audio and after callbacks stop; `native-editor-smoke` checks visible JUCE peak/RMS pixels and independent clip controls. The packaged React panel was rendered in Chromium at 1200×800 and 820×560 with a representative host packet: both sizes kept the meter in view without page overflow, showed 65% peak/42% RMS on the left and a latched right clip. The same view model and browser transport passed their Node tests. The native JUCE snapshot was inspected at 820×560 with 0.5/0.4 left peak/RMS, 0.25/0.125 right peak/RMS and a right clip latch.

## 5. Prepared Waveform Vertical Slice

- [x] 5.1 Add asynchronous per-channel min/max reduction and content-keyed cache admission; tests must prove the -0.75/0.5 extrema fixture, narrow transients, region boundaries and same-path content invalidation.
- [x] 5.2 Add job cancellation and safe source retention through reload/editor teardown; tests must prove stale results cannot replace current data and audio never joins or frees worker resources.
- [x] 5.3 Render the same envelope and prepared overlays in native JUCE and Canvas; verify source/host rate distinctions and matching marker coordinates at both target sizes before expanding the full UI.

Verification for 5.1: Rust `sample` and FFI tests prove signed extrema, channel separation, narrow transients and source-frame offsets; `cxx-sampler-plugin-host` proves prepared region bounds and signed kick extrema; `cxx-plugin-waveform-service` proves asynchronous completion, duplicate work sharing, bounded queue/history/cache and same-path content invalidation. The new C++ service reached 100% source-line coverage in a focused gcov run. Native CTest passed 17/17 and Web CTest passed 29/29. Cancellation and renderer display remain tracked by 5.2 and 5.3.

Progress on 5.2: waveform requests retain an editor session ID. Closing native or Web editor sessions cancels their active and queued requests without waiting for the worker; the focused worker test keeps one reduction stalled and proves another session continues. The Web bridge test requests a waveform from an editor, checks another editor cannot cancel it, then closes the owner and queries its terminal status. A browser transport now coalesces rapid region changes, cancels late accepted or wrong-generation jobs, retires stale terminal status and cancels on closure with 100% JavaScript source-line coverage. The native waveform view requests with its editor session and clears the old result on reload; a deterministic native teardown test with an active job remains open.

Completion of 5.2 (2026-10-03): `native-waveform-lifetime` uses the original
sampler factory, processor and native editor, holding the real Rust reducer at
a link-wrapper barrier. Closing one editor immediately cancels its running
job while another session remains pending and later displays current data; a
reopened editor reuses the cache. A separate same-path reload retires the old
engine while its source remains retained, verifies the original four signed
buckets after release, and delivers the replacement's distinct buckets under
the new generation. Cancelled/stale jobs keep terminal status without results.
Four independent callbacks return before barrier release, with all channel
samples matching literal `+0.25/0` or `-0.5/0`; wrapped reduction/source/engine
cleanup never occurs on those audio calls. A copied native editor omitting
teardown cancellation fails the named assertion; byte-identical recovery
passes. Focused service gcov executes all 149 source records, and native gcov
observes the actual destructor/generation cancellation and delivery paths.
Full native/Web builds pass; CTest passes 21/21 and 44/44 without skips.
[Lifetime evidence](../../../docs/design-system/waveform-lifetimes.md) records
the Linux integration boundary and coverage/instrumentation limits. This does
not prove DAW timing, complete callback allocation safety, task 5.3 renderer
coordinates or section 9's structural engine ownership handoff.

Progress on 5.3: the Web bridge exposes bounded prepared waveform requests and exact numeric job results (source-frame buckets, source sample rate and content revision). A copied coordinate model places signed buckets and prepared fade/loop/slice markers at full and compact widths using the source rate; the focused C++ geometry test reached 100% source-line coverage. The native editor paints the prepared numeric envelope and markers at 820×560 and 1200×800, and its smoke test verifies visible PCM and clears the waveform after a reload to a sample-free instrument. A renderer-neutral Web Canvas painter now consumes the same prepared document/job fields, with 100% JavaScript source-line coverage for signed extrema, source-rate timing, 64-bit frame offsets, markers at both sizes, stale-result rejection and edge clipping. React mounting, runtime Canvas inspection and cross-renderer comparison remain open.

Completion of 5.3 (2026-10-03): `native-waveform-parity` and
`web-waveform-parity` compile one literal stereo fixture and bitmap oracle in
both modes. The original sampler factory, processor, editors and Web transport
remain intact. All 512 real buckets retain exact signed extrema and source
bounds; actual JUCE and mounted React Canvas pixels prove four signed quarters
and ten region/fade/loop/slice columns at 1200×800 and 820×560. A 48 kHz source
in a 44.1 kHz host proves source-rate fade timing and the 0.256 s duration.
The test first rejected JUCE's truncated slice-end position, then passed after
its four pixel coordinates adopted Canvas's rounding rule; focused gcov
executes all four changed lines. Full builds pass, with final CTest 22/22 native
and 45/45 Web without skips. [Renderer evidence](../../../docs/design-system/waveform-parity.md)
records the inspected plots, composite build receipts and Linux/1x limits.
Full reference styling, other display scales, spectral views, live cursors and
structural rebuilding remain their respective pending tasks.

## 6. Spectral And Live Analysis

- [x] 6.1 Implement shared static spectral jobs with declared FFT/window/hop/scaling/floor and numeric results; tests must prove a known bin-centred sine, finite silence floor, deterministic results and cache reuse.
- [x] 6.2 Add native and Web spectral views consuming the same result; runtime inspection must verify time/frequency labels and overlays without transferring a browser-specific image as the shared contract.
- [ ] 6.3 Add subscribed live scope and spectral capture with fixed memory and tap/channel limits; tests must prove queue overflow cannot block audio and discontinuities reset partial FFT windows.
- [ ] 6.4 Bound worker count, backlog, cache bytes and column publication; tests must prove stalled consumers, cancelled jobs and hidden views retain bounded resources and recover with current sample coordinates.

Task 6.1 completion evidence (2026-10-03): the shared C++ worker supplies
periodic-Hann 1024-point, one-sided peak dBFS spectra with a -120 dBFS floor,
selected source channels, explicit source-frame windows, source-rate frequency
bins and a declared bounded hop. The sole Rust addition copies contiguous PCM
from an independently retained prepared source; analysis stays outside Rust.
`cxx-plugin-spectral-service` proves absolute sine/window/endpoint/tail scaling,
silent floor, determinism, content-aware caching, bounded admission/history,
failure retry, session cancellation and reload retirement. The original sampler
factory's `cxx-plugin-spectral-job` proves source-index selection and retained
worker reads/cleanup while callbacks produce known signed audio. Both build
configurations pass: native CTest 24/24, Web CTest 47/47, without skips; Rust
tests, strict validation and the existing main-spec coverage gate pass.
Source coverage reaches all 173 new service executable lines, all 37 new
processor method/publication lines, and all new FFI function regions. Nine
manual service faults and four manual FFI faults are rejected; no exhaustive
mutation or whole-processor coverage claim is made. See
`docs/design-system/static-spectral-analysis.md` for settings, ownership and
verification limits. Spectral drawing/live analysis (6.2-6.4), callback profiling
and automatic structural rebuild runtime remain pending.

Task 6.2 verification (2026-10-03): both original sampler editor paths paint
the shared numeric spectrum with source-rate logarithmic frequency/time axes,
finite floor and ten prepared overlays at 1200×800 and 820×560. Actual runtime
tests prove Wave/Spectral switching, hide/show, real PCM-read failure and retry,
and clearing after reload to the sample-free 303. Bridge tests compare exact
numeric pages; bounded browser assembly rejects partial/stale/incoherent data.
Full builds pass, with CTest 26/26 native and 51/51 Web without skips, sampler
typecheck/assets and strict validation. The
[spectral view record](../../../docs/design-system/spectral-views.md) states
changed-line/branch coverage limits, eleven focused faults and Linux/1x runtime
limits. No Rust, engine preparation, host parameter identity, live capture,
final layout or automatic structural runtime change is included.

Task 6.3 capture progress (2026-10-03): the fixed master-output PCM queue
preserves full-rate signed selected channels in 64×256-frame chunks. Component
tests prove copied ownership, bounded losses, explicit gaps, current resumed
coordinates and rapid hide/show selection revisions; all 51 new executable
header records run, and seven copied faults fail named assertions. The
[live analysis record](../../../docs/design-system/live-analysis.md) distinguishes
this foundation from pending worker, scope/FFT, processor and subscription
integration. Task 6.3 stays unchecked; no actual callback, renderer or timing
completion is claimed by this increment.

Task 6.3 numeric progress (2026-10-03): a worker-owned accumulator now emits
complete 1024-frame windows with 128 signed scope buckets and shared periodic
Hann/256-hop/one-sided peak-dBFS spectra. Tests prove literal stereo levels,
frame/frequency coordinates, unequal chunks, finite overrange PCM and recovery
after nine continuity changes and ten invalid-frame cases. The prepared service
uses the same covered measurement while retaining its padded-tail contract.
Focused coverage executes 78/78 new numeric records; fourteen copied faults
fail named assertions. An encountered prepared Canvas paint race is fixed and
proved by a read-only observer in the original packaged Web runtime. Complete
builds pass, with CTest 28/28 native and 53/53 Web without skips. The live analysis
record retains failed runs and evidence limits. Worker/session/backlog,
processor and live-renderer integration remain pending; 6.3 stays unchecked.
No callback timing or automatic structural runtime completion is claimed.

Task 6.3 worker progress (2026-10-03): one actual off-audio worker consumes
copied PCM and owns four fixed session slots, each with one outstanding plus
one replaceable latest packet. Tests prove signed measurements, acknowledgement
identities, visibility/reload retirement, bounded recent backlog, overflow
recovery, generation/selection races, gap retention and stop-aware teardown.
Final focused coverage executes 139/139 implementation records; ten copied
faults fail named assertions after repairing an initially surviving channel-union
fault. Full builds pass, with CTest 29/29 native and 54/54 Web without skips.
The live analysis record distinguishes component evidence from pending processor
capture, shared commands/adapters, actual live views, engine-output parity,
callback instrumentation and timing. At that increment the service was linked only to its registered
test target. Tasks 6.3/6.4 stay unchecked; no structural runtime completion is
claimed.

Task 6.3 processor progress (2026-10-03): both plugin builds now own the live
service. The real processor/engine fixture proves exact signed capture at
44.1/48/96 kHz, live host identity, current hide/show and muted coordinates,
and bounded overflow while the actual worker is held. Linux callback guards
observe zero C++ allocation, directly linked locks, joins, FFT and kernel
lifecycle calls; eight copied faults fail their named assertions. Focused gcov
executes 24/24 changed processor records, not the whole module. Both builds
pass; CTest passes 30/30 native and 55/55 Web without skips. The live analysis
record states the instrumentation and factory/DAW limits. Concurrent reload
identity awaits task 9.2's safe ownership handoff; the existing fixed delay is
not its proof. Shared adapters, actual live views and browser delivery remain
pending, so 6.3/6.4 and structural runtime remain unchecked.

Task 6.3 live identity progress (2026-10-03): the registered live-handoff
regression exercises actual reload, 48-to-96 kHz host reprepare and state
restore with a held render and subscribed worker. Old packets retain their
identity; replacement windows require complete contiguous new PCM and reset
frame origin/stream. Stable host objects and signed `+0.25`/`-0.5` outputs are
asserted. The handoff/capture production code is unchanged from `54f8b4f`; the
[live analysis record](../../../docs/design-system/live-analysis.md) records
coverage, five calibrated faults, setup correction and controlled-host limits.
Shared adapters, actual live views and browser delivery remain pending; tasks
6.3/6.4 and automatic structural runtime stay unchecked.

Task 6.3/6.4 renderer progress (2026-10-03): registered Web commands now consume
real processor-owned packets; one browser controller bounds requests through
acknowledgement and releases obsolete/hidden demand. Native and sampler React
Scope/Live FFT render independent signed L/R output, actual rate/settings and
output-stream frame bounds. Original factory/editor tests prove full/compact
plots, reload, hidden resumption and explicit four-slot admission rejection with
recovery. Focused coverage executes 99/99 changed bridge and 124/124 changed
native records; both new shared JS modules execute all V8 lines/functions/
branches. Twenty-two copied faults fail named assertions after healthy baselines;
failed harness runs, a survivor and the Web admission-test timing correction are
retained. Final builds include Standalone/VST3; CTest passes 34/34 native and
62/62 Web without skips. The [live analysis record](../../../docs/design-system/live-analysis.md)
defines the controlled runtime/protocol boundaries. Consolidated resource/stress,
long-duration browser lifecycle/React stalls and callback timing remain pending;
6.3/6.4 and section 9 stay unchecked. No structural runtime, complete sampler/303
redesign, DAW hosting or spec synchronization is claimed.

Task 6.3/6.4 sustained-consumer verification (2026-10-03): the actual live
service now has a four-consumer repeated stall regression with zero observed
ordinary C++ new calls after warmup, eight delivery positions, bounded batch
work and current signed recovery. The original packaged Web editor's registered
stress lane holds real packet/acknowledgement replies for five seconds, then
proves hide/show, reload document retirement and close/reopen recovery while
literal signed PCM continues. Three copied component faults fail named
assertions; a separate extra original browser request calibrates its request
counter. No production source or asset changes are included. The
[live analysis record](../../../docs/design-system/live-analysis.md) specifies
fixed-storage measurements, coverage, retained setup/harness failures and the
AT-SPI/delayed-Promise limits. Prepared cache byte accounting, broader resource
workloads, event-loop stalls and callback timing remain pending; 6.3/6.4 and
section 9 stay unchecked.

This verification slice passes complete native/Web builds, including
Standalone/VST3, and CTest **34/34 native** and **63/63 Web**, with no skips.
It leaves task checkboxes and implemented-baseline scenario mapping unchanged.

Task 6.4 host-state progress (2026-10-03): the Web bridge now bounds timer
parameter notifications to one unacknowledged delivery per document. The shared
bootstrap acknowledges actual state receipt and current-state query replies;
hidden ancestors suppress publication and reload retires its ticket. Fast
registered-command tests prove repeated stalls, editor isolation, exact/stale
acknowledgements, current values and unchanged signed live audio. The original
packaged WebKit lane stops its actual JavaScript event loop for two seconds,
then observes one obsolete event and current knob/Scope recovery while native
processor calls continue. Focused coverage executes 19/19 new C++ records and
all bootstrap V8 functions/ranges; six copied component faults fail assertions.
The [live analysis record](../../../docs/design-system/live-analysis.md) retains
RED/setup evidence and states receive-acknowledgement and controlled-host limits.
Complete native/Web builds include Standalone/VST3; CTest passes **34/34 native**
and **64/64 Web**, with no skips. Tasks 6.3/6.4 and section 9 remain unchecked;
cache accounting, full layouts and structural jobs stay open.

## 7. Capability-Aware Sampler Composition

- [ ] 7.1 Implement prepared KeyMap inspection/audition in both renderers; tests must prove actual velocity ranges, alternate/layer distinction, capability-gated structural commands with automatic rebuilding, and host-driven note feedback.
- [ ] 7.2 Implement supported LayerStack inspection and public parameter bindings; tests must route supported structural changes through section 9's transaction, reject unsupported local mutation and keep undeclared internals unavailable.
- [ ] 7.3 Implement actual bus/channel enumeration and OutputBusses display; tests must cover stereo-only and non-stereo named layouts without invented output pairs, and route supported internal structural routing edits through automatic rebuilding.
- [ ] 7.4 Add capability-backed pad/alternate/cursor feedback with preallocated bounded taps; tests must observe real host and editor note playback, generation changes and missing capabilities without simulated audio state.
- [ ] 7.5 Compose full/compact native and web layouts with keyboard/focus/value entry; inspect actual runtimes at 1200x800, 820x560 and common display scales, recording any platform-specific differences.
- [x] 7.6 Adapt the supplied TB-303 React panel to the shared asset and command path; tests must prove all seven actual public controls, host-backed note audition, authoritative reload updates and explicit unavailability of unsupported waveform, pattern and transport editing.

Progress toward 7.1–7.3: the Web bridge now supplies the same copied prepared document used by the native side, including scoped controls, source/region/slice frames, map/zone selection metadata and capabilities. Browser frame and seed values are decimal strings to retain their full 64-bit values. The renderer components can now use the imported design-system reference; their production adaptation remains open.

Progress on 7.1 and 7.5: the sampler React host app groups prepared round-robin alternatives without claiming simultaneous layers, displays the actual 1–63/64–127 snare ranges and per-pad control scopes, sends note audition through the shared command bridge, and mounts the prepared Canvas waveform and master meter. A mock-host Chromium run inspected 1200×800 and 820×560 layouts and exercised a pad note on/off, a complete slider gesture, typed Escape cancellation, and rejection reconciliation. Native KeyMap, observed host-playback feedback, final design components, and plugin-host runtime inspection remain open.

The sampler input controller now coalesces continuous pointer and keyboard values while a host write is pending, retains the last value before ending a gesture, and wraps typed/keyboard edits in begin/end boundaries. Focused Node tests cover host stalls and rejection cleanup with 100% source-line coverage for the new controller. Prepared-document requests are coalesced and only refreshed when the instrument generation advances.
Review repairs added successive-drag, keyboard-after-release and typed-during-close regressions, registered the gesture suite in CTest, and connected pad/window blur to note release. A Chromium mock-host DOM run observed note-on/note-off pairs for both pad blur and window blur and cleared pressed feedback; actual plugin-host WebView focus behaviour remains to inspect.

## 8. Integration And Evidence

- [ ] 8.1 Run relevant Rust, C++, JavaScript and packaging checks plus CMake/CTest in the task worktree; record focused and full results separately and do not claim runtime proof from compilation.
- [ ] 8.2 Compare disabled/enabled/stalled telemetry with identical timed inputs and known signed outputs; instrument callback allocation/locking/posting paths and record supported-platform limitations.
- [ ] 8.3 Run native and Web stress workloads at 44.1/48/96 kHz, small/normal/oversized blocks and multiple instances; record baseline/enabled callback distributions, observed deadlines, drops, CPU and memory while resizing, gesturing, hiding, reloading and cancelling.
- [ ] 8.4 Review coverage and refactoring opportunities after behaviour is specified; newly extracted modules require full coverage, and changed critical paths receive focused mutation testing after checking disk/build-output sizes.
- [ ] 8.5 Map every new scenario to a proving test in `spec-tests.map` when syncing main specs; review fingerprints explicitly, run `scripts/check-spec-coverage`, and pass strict OpenSpec validation before archive.
- [ ] 8.6 Inspect the final diff and repeated repository inventory; preserve other work, commit verified logical slices with Why/What/Verification/Constraints bodies, and publish or integrate only under the applicable authorization.

## 9. Automatic Structural Rebuilding

Contract and acceptance cases: [structural-authoring-acceptance-tests.md](structural-authoring-acceptance-tests.md) and [plugin-structural-authoring](specs/plugin-structural-authoring/spec.md). This section is newly specified work; existing reload tests do not complete it. Follow RED → GREEN → coverage/refactor for each behavior before connecting structural controls.

- [ ] 9.1 Characterize current reload/muting/host-slot behavior, then write executable edit-admission and stalled-worker tests first; prove immediate muting before validation, exact-zero callbacks and asynchronous acceptance without Apply or confirmation.
- [ ] 9.2 Implement and instrument the explicit engine ownership handoff; prove an in-flight reader retains valid storage, later callbacks access no engine, a no-callback host period cannot strand rebuilding, and preparation/destruction/cleanup occur off audio without fixed-delay assumptions.
- [ ] 9.3 Implement one processor-owned rebuild at a time and serialize engine ownership with reload, file watching, state restore and host preparation; tests must reject concurrent structural requests as busy without queued edits, survive editor closure/reconnect, and reclaim worker resources safely at processor shutdown.
- [ ] 9.4 Implement automatic candidate activation and failure recovery; tests must cover validation, missing assets, compilation, worker startup and activation failures, known signed audio from the recovered configuration, unchanged working generation on failure, visible errors and automatic control re-enabling.
- [ ] 9.5 Preserve existing host parameter objects, IDs, count/order, slots and automation bindings; test success/failure/repeated rebuilds and rejection of incompatible exposed surfaces. Ordinary parameters must remain live without rebuilding, and values changed during preparation must reach either resumed engine.
- [ ] 9.6 Connect only supported structural controls in both renderers; tests must prove automatic edit submission, rebuilding/disabled states, recovery reconciliation and no draft/Apply/confirmation/unapplied workflow. Keep unsupported reference callbacks inert.
- [ ] 9.7 Verify real DAW transport continuity and independent instances, then map every structural acceptance scenario to executable tests when syncing specs; run focused callback safety/coverage/mutation and full relevant gates before claiming implementation complete.

Specification verification (2026-10-02): strict OpenSpec validation passes; the
13 acceptance test descriptions cover all 14 new structural scenarios. Local
links in the 11 changed documents resolve. `scripts/check-spec-coverage` passes
for the unchanged implemented baseline (559 scenarios, 320 mapped, 239 existing
todos). No structural runtime tests have been implemented or run by this
specification update, and no new baseline scenarios or fingerprints were synced.

Task 9.2 ownership progress (2026-10-03): engine installation, host reprepare
and state restoration now use a bounded lock-free callback reader guard.
Off-audio replacement closes admission and waits for acknowledged reader
release before changing engine/slot storage and retiring the old engine;
the two fixed retirement sleeps are removed. Registered baseline/held-reader
tests use the real processor and engine, literal signed output and host object
identity, silent later callbacks at 1/64/512/2048 frames, actual retirement
observation and focused callback guards. No future callback is required for
quiescent replacement. The original held-reader test failed on premature
retirement before implementation. This advances the S3/S4 prerequisite;
immediate structural admission, autonomous rebuilding, recovery, renderer
commands and shutdown during pending structural work remain pending.
The additional live-identity regression is recorded under task 6.3 above. Section 9 and tasks 6.3/6.4 remain unchecked. The
[live analysis record](../../../docs/design-system/live-analysis.md) states
the measured evidence and limits.
