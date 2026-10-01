## 1. Baseline And Design Reference

- [x] 1.1 Resolve the active-work gate and choose the integrated sampling base or explicit dependency on `3b313a7`; verify worktrees, upstreams, PRs and the repeated status snapshot before repository mutations.
- [ ] 1.2 Import the reviewed design-system revision with its SHA-256 and provenance; verify archive integrity, local reference links and any installed skill frontmatter without executing bundled development tooling.
- [x] 1.3 Characterize current shared/per-pad parameters, host gestures, note admission and reload behaviour with focused C++/JavaScript tests; record a known signed sampler render and current coverage before production changes.
- [ ] 1.4 Specify native KeyMap, LayerStack and OutputBusses dimensions, focus, read-only interactions and capability states; review the delivered native handoff against the new JSX/type references and existing external-authoring contract.

The sampler export required for 1.2 and 1.4 is not present in the current Downloads folder. Source-independent metadata and command work in section 2 can proceed while that input is located; the renderer component tasks remain dependent on its verified import.

## 2. Shared State And Commands

- [x] 2.1 Add typed prepared UI documents and scoped capabilities; tests must prove actual 63/64 snare boundaries, parameter scopes, zone-selection semantics and absent capability reporting.
- [x] 2.2 Add the minimum read-only metadata access needed across FFI using retained/copy ownership off audio; lifetime tests must survive reload, released source inputs and a stalled reader without touching freed storage.
- [x] 2.3 Extract shared host command handling with generation and finite-value validation; run the same accepted/rejected command contract tests through native and Web adapters.
- [x] 2.4 Implement begin/update/end gestures and timer-observed parameter updates; tests must prove one drag equals one host gesture, keyboard/typed entry works, stale echoes are ignored and listeners do not post messages from audio.
- [ ] 2.5 Add bounded editor note-release/session cleanup semantics; saturation tests must prove queued note-on plus release/disconnect cannot leave a gated note active or affect unrelated host MIDI.
- [ ] 2.6 Expose accepted job IDs and queryable reload/analysis status; tests must prove early acceptance, failed reload preservation, reconnect recovery and stale-job rejection.

## 3. Two Renderer Foundations

- [ ] 3.1 Split common UI, native editor and Web adapter build targets; verify a native-only configure/build/run with browser support disabled and without Node or WebView/WebKit dependencies.
- [ ] 3.2 Introduce one maintained token source and CSS/C++ generation; verify corresponding semantic values and document font licensing/provenance.
- [ ] 3.3 Build the production web asset pipeline and embedded resource serving; verify packaged assets work offline outside the source checkout, with no CDN scripts, runtime Babel or development server.
- [ ] 3.4 Implement one design-system knob in each renderer against shared commands; tests must prove identical host gestures, authoritative values, slot identity and known signed audio for identical schedules.

## 4. Meter Telemetry Vertical Slice

- [ ] 4.1 Prepare bounded master/output meter capture with a single consumer per queue; saturation and callback instrumentation must prove bounded memory/work, no forbidden callback operations and unchanged signed PCM.
- [ ] 4.2 Implement sample-weighted peak/RMS aggregation and independent clip latching; tests must cover signed stereo, unequal blocks, silence, dropped history and generation/reset races.
- [ ] 4.3 Add subscription lifetime, sequence/generation handling and bounded Web acknowledgements; tests must prove hidden/stalled/reopened editors cannot grow browser message queues or block audio.
- [ ] 4.4 Render the same meter data in native and WebView components; deterministic view-model tests and actual runtime inspection must verify levels, clipping, visibility and timing-based decay.

## 5. Prepared Waveform Vertical Slice

- [ ] 5.1 Add asynchronous per-channel min/max reduction and content-keyed cache admission; tests must prove the -0.75/0.5 extrema fixture, narrow transients, region boundaries and same-path content invalidation.
- [ ] 5.2 Add job cancellation and safe source retention through reload/editor teardown; tests must prove stale results cannot replace current data and audio never joins or frees worker resources.
- [ ] 5.3 Render the same envelope and prepared overlays in native JUCE and Canvas; verify source/host rate distinctions and matching marker coordinates at both target sizes before expanding the full UI.

## 6. Spectral And Live Analysis

- [ ] 6.1 Implement shared static spectral jobs with declared FFT/window/hop/scaling/floor and numeric results; tests must prove a known bin-centred sine, finite silence floor, deterministic results and cache reuse.
- [ ] 6.2 Add native and Web spectral views consuming the same result; runtime inspection must verify time/frequency labels and overlays without transferring a browser-specific image as the shared contract.
- [ ] 6.3 Add subscribed live scope and spectral capture with fixed memory and tap/channel limits; tests must prove queue overflow cannot block audio and discontinuities reset partial FFT windows.
- [ ] 6.4 Bound worker count, backlog, cache bytes and column publication; tests must prove stalled consumers, cancelled jobs and hidden views retain bounded resources and recover with current sample coordinates.

## 7. Capability-Aware Sampler Composition

- [ ] 7.1 Implement prepared KeyMap inspection/audition in both renderers; tests must prove actual velocity ranges, alternate/layer distinction, read-only boundaries and host-driven note feedback.
- [ ] 7.2 Implement supported LayerStack inspection and public parameter bindings; tests must reject implicit module addition/removal/reordering and keep undeclared internals unavailable.
- [ ] 7.3 Implement actual bus/channel enumeration and OutputBusses display; tests must cover stereo-only and non-stereo named layouts without invented output pairs or structural routing changes.
- [ ] 7.4 Add capability-backed pad/alternate/cursor feedback with preallocated bounded taps; tests must observe real host and editor note playback, generation changes and missing capabilities without simulated audio state.
- [ ] 7.5 Compose full/compact native and web layouts with keyboard/focus/value entry; inspect actual runtimes at 1200x800, 820x560 and common display scales, recording any platform-specific differences.
- [ ] 7.6 Adapt the supplied TB-303 React panel to the shared asset and command path; tests must prove all seven actual public controls, host-backed note audition, authoritative reload updates and explicit unavailability of unsupported waveform, pattern and transport editing.

## 8. Integration And Evidence

- [ ] 8.1 Run relevant Rust, C++, JavaScript and packaging checks plus CMake/CTest in the task worktree; record focused and full results separately and do not claim runtime proof from compilation.
- [ ] 8.2 Compare disabled/enabled/stalled telemetry with identical timed inputs and known signed outputs; instrument callback allocation/locking/posting paths and record supported-platform limitations.
- [ ] 8.3 Run native and Web stress workloads at 44.1/48/96 kHz, small/normal/oversized blocks and multiple instances; record baseline/enabled callback distributions, observed deadlines, drops, CPU and memory while resizing, gesturing, hiding, reloading and cancelling.
- [ ] 8.4 Review coverage and refactoring opportunities after behaviour is specified; newly extracted modules require full coverage, and changed critical paths receive focused mutation testing after checking disk/build-output sizes.
- [ ] 8.5 Map every new scenario to a proving test in `spec-tests.map` when syncing main specs; review fingerprints explicitly, run `scripts/check-spec-coverage`, and pass strict OpenSpec validation before archive.
- [ ] 8.6 Inspect the final diff and repeated repository inventory; preserve other work, commit verified logical slices with Why/What/Verification/Constraints bodies, and publish or integrate only under the applicable authorization.
