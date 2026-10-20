# Independent completion review — FAIL

Candidate manifest SHA256: `0fcb69ff5c9f6a741b7b2e521191807605a253034998ee633eb280217c264c77`.

This is a fresh independent read-only review of the task 3.3 optional frontend-service slice. No source, test, index, Git, build or cache changes were made. Reviewer outputs are confined to the reviewer evidence directory. Root explicitly authorized one bounded public-API Node probe with timers disabled and ordinary close cleanup; it used no native host, server or network. Approval was not delegated.

## Finding F1 — accepted voice-limit refresh stays stale without metering

**Severity:** medium. **Outcome:** correction required before publication.

**Owner:** `web/trigger/engine/native-live.mjs:15–25`; consumer: `engine/telemetry-reader.mjs:6–7`, `components/AppHeader.jsx:23`.

**Requirement:** accepted design “Declare optional editor services without fabricating native success” and delta scenario “Explicitly unavailable optional services remain honest”: refresh from the real accepted session while measurements remain unavailable. The candidate also expressly claims the actual accepted voice limit in its initial adapter snapshot.

`NativeLive` initializes `telemetry.voices.max` from the first patch. Its only subsequent assignment is inside the meter-packet callback. The newly introduced early return when `metering` is false prevents that callback from ever existing. `changed()` resets notes/cache and calls `resetMeter()` without publishing accepted metadata. The Store's telemetry reader retains its initial cloned snapshot until a telemetry event, so even rerendering from a new patch cannot repair the value.

**Independent reproduction:** initialize the declared all-false profile using the maintained native fixture (initial YAML `max_voices: 8`); substitute `max_voices: 12` in the accepted next-generation session on the same connection; deliver `parameterStateChanged` generation 8 and await completion. Both `engine.getPatch().voicePolicy.limit` and `store.getSnapshot().patch.voicePolicy.limit` are 12. `store.getTelemetry().voices.max` remains 8, including after `pollTelemetry()`. The header's actual expression therefore yields `Voices — / 8`, rather than `Voices — / 12`. Only two `getSamplerSession` calls occur. The captured result is `voice-limit-probe.json`.

The first probe attempt from integration could not import `yaml`; this was a setup failure and supplied no behavioral evidence. The completed probe ran from the author frontend checkout after checking every one of its 16 changed-source hashes against the frozen candidate. It uses the real NativeEngine/Store public paths and the existing session fixture. It proves frontend refresh behavior, not a Rust/Wry/native boundary.

**Required outcome:** when an accepted session/patch changes, publish the accepted voice limit through the adapter telemetry snapshot/event path independently of optional meter availability. Keep `active: null`, `voiceCountAvailable: false` and unmeasured meters honest. Add a behavior-first regression with distinct initial and replacement limits that asserts the Store telemetry seen by the header, including notification to subscribers, and zero optional RPCs. Cover relevant accepted refresh paths without depending on a meter packet; retain mapping/undo/parameter/note behavior and supported-service regressions. Do not repair this by inventing measured voice counts or starting a meter service.

## Task-to-implementation matrix

| Task or claim | Actual implementation/evidence | Assessment |
|---|---|---|
| Task 3.3 optional bootstrap profile | native-transport validates four boolean fields, defaults omitted fields independently, copies and freezes normalized profile; NativeEngine freezes its capability view | Satisfied |
| Task 3.3 optional service lifecycle | NativeEngine gates title and meter timer; NativeLive skips absent meter owner, safely polls/closes, rejects async waveform access; required paths remain real invoke calls | Satisfied except F1 refresh metadata |
| Task 3.3 honest presentation | Presenter supplies flags and no-op host-size commands; controller avoids waveform requests; header/output/waveform components show explicit absence | Initial display satisfied; updated voice limit fails F1 |
| Task 3.1 comparison behavior retained | Legacy JUCE, explicit injected backend, generic versioned descriptors and silent preview preserved; nearby owner tests retained | Satisfied within frontend scope |
| Task 3.2/3.3 complete native attachment, Wry, CLAP/audio acceptance | Tasks remain unchecked; 7/14 status unchanged; native snapshot facade and full destruction proof excluded | Correctly open; not credited as completed |
| Portable verification and publication | Exact source/evidence manifest and finite metadata envelope supplied | Reproducible candidate, but publication blocked by FAIL |

## Criterion-to-test matrix

Test names below are exact maintained test identities or distinctive exact prefixes from the named file. Results come from retained raw evidence; the reviewer did not rerun complete suites.

| Criterion | Test and independently meaningful oracle | Assessment |
|---|---|---|
| All unavailable services permit initialization and cleanup | `native-transport.test.mjs`: “an explicitly absent optional profile initializes the accepted native session without optional calls” pins accepted name, parameter .25, exact getSamplerSession-only trace and listener removal | Satisfied |
| Omitted metadata/fields preserve services for any ID; independently false fields stay independent | “capability omission retains full services for every backend identity and invalid metadata fails before native calls” tables juce-free/comparison/future-host, omitted/empty/partial profiles, each isolated false flag; exact title/subscription traces | Satisfied |
| Invalid present metadata fails before native invocation | Same table exercises undefined/null/array/scalar profile and non-boolean values for all four fields, throws and empty native-call trace; main.jsx already renders initialization errors | Satisfied |
| Profile cannot mutate after normalization | “selected capability metadata is immutable and independent of later descriptor mutation” checks copied values and TypeError; initial absent-profile test checks engine profile mutation TypeError | Satisfied |
| Direct absent waveform/resize requests preserve state | “explicit absent waveform and resize requests preserve accepted state and report capability errors without native requests” pins structured code+method, patch/history/events and getSamplerSession-only trace | Satisfied |
| Absent metering starts unknown, no fabricated measurements | “absent metering starts with unknown native voices and keeps mapping undo refresh and notes usable” pins null active, max 8, unavailable flag and empty meters | Satisfied initially |
| Accepted metadata remains current after refresh | Same lifecycle test refreshes generation but keeps the same voice limit; no assertion could detect stale max. Independent public-API probe changes 8→12 and observes stale 8 | **FAIL F1** |
| Required mapping/undo, parameters, notes, heartbeat and close survive absence | Same lifecycle test asserts map 36→60→36, native history/no local journal, exact parameter generation/value, heartbeat, one noteOff, listener removal and no subsequent calls after close; explicit filter rejects all optional RPCs | Satisfied for claimed adapter contract |
| Host controls size without optimistic local layout/performance | `presentation.test.mjs`: “host-controlled native sizing preserves editor layout without optimistic changes or resize requests” checks unchanged editor values after imperative calls and no extra RPC | Satisfied |
| Actual renderer presents absence without fake peaks | `browser/native-transport.spec.mjs`: “explicit absent services render accepted controls with unavailable measurements and host-controlled size” checks initial Voices — / 8, meters/host labels, hidden size actions, 4 loop placeholders/no SVG, 1 slice placeholder, routing labels, public knob, no alert/page error and exact native call trace | Satisfied for initial render; F1 remains outside this fixture |
| Existing supported-service failures/retries retained | `native-engine.test.mjs`: window-title failure preserves accepted patch; real meter packets acknowledged/stale dropped/native waveform .75; stale waveform rejects; meter-subscription and generation-refresh cases; presentation native resize rejection remains error | Satisfied within existing tested adapter boundary |
| Preview and comparison selection stay honest | Existing native-transport tests exercise legacy/injected/precedence and structured failure; browser tests prove unsupported version/rejected snapshot display errors and ordinary browser remains silent | Satisfied |

## Candidate and evidence reconstruction

- Base/current/integration-base and declared `origin/feat/juce-free-host-adapters` all resolve to `0f02f575ff2419082370f4d35986d086c03b5d0b`; branch is `lane/trigger-integration-20261020`; main remains `68dd6553a2f07e71e2fe09ee1d7094d518559937`.
- The isolated integration branch has no configured `@{u}`. The packet explicitly declares the publication upstream; the original feature branch configuration independently names origin and the matching refs/heads feature. This is not hidden as configured tracking on the integration lane.
- All 28 changed paths are bytewise ordered and exactly match current status. Independently checked canonical modes, SHA256 bytes, staged blob IDs, blob/worktree equality and regular file types; no extra changed path or unmerged index.
- All ten portable SHA256SUMS entries validate. All 236 raw archive files match author-artifacts.json byte length, SHA256 and stored mode. All 16 final-source copies match candidate files and the source manifest.
- Raw RED logs show expected product assertion failures: extra optional calls; missing invalid-profile rejection; missing unsupported-access rejection; initial 0 instead of null; missing host-size flag; missing rendered unavailable meter label; missing immutable engine-profile TypeError. These are distinct from disclosed setup failures.
- Retained final raw coverage reports 108/108 passing; browser log reports 32/32. Type/component check and both browser/native bundle builds exit 0. Native bundle reports the disclosed existing approximately 808 KB chunk warning. Native bundle build is a Vite bundle build, not a source-free Rust/native/CLAP build.
- Root retained strict scoped OpenSpec validation, spec-coverage and diff-check records have exit 0; no baseline spec/task mutation is part of this slice. No changed source-free fixture/native src/tests/Cargo/CMake inputs are present.
- Final source bindings are credible and exact. Inner RED phase source hashes were not captured; logs cannot retroactively establish each intermediate file snapshot. This is explicitly disclosed, not treated as exact per-phase provenance.

## Coverage, architecture and alternatives

The changed adapter owners have 100% measured line execution in retained fast coverage; transport and telemetry-reader have 100% measured branches/functions. NativeEngine/NativeLive retain disclosed branch/function gaps. ShellPresenter is 91.34% line-covered with inherited ranges listed. The browser artifact records transformed Chromium V8 ranges and generated-source hashes. Its positive enclosing counts support execution of the selected changed JSX occurrences, not original-JSX 100% line/branch coverage. None of these percentages proves F1's missing state-transition assertion.

Tests mostly use public observable state, exact known call inputs and meaningful failure identities. Tables reduce duplication for a single metadata invariant; the composed lifecycle case earns its cost by covering required behavior with optional services absent. Fast/slow lanes remain Node owner tests and Playwright. New frontend browser test is under a second in the retained run; full browser suite is about 77 seconds. No lane migration or mutation claim is warranted. Mutation is explicitly excluded.

The design keeps transport selection separate from engine/live ownership and renderer flags, without new dependencies, vendor edits, extracted modules or backend-name heuristics. Existing omission compatibility has real version-1/JUCE/injected consumers and explicit accepted design authority. Direct unavailable service errors retain structured identities and avoid mutation. Required calls remain actual adapter invocations, while fixture-backed browser tests are correctly limited to adapter/renderer evidence. No Wry facade, Rust snapshot, CLAP automation/audio or complete attachment-destruction proof is inferred.

A materially simpler correction for F1 should update known accepted session metadata at the existing refresh owner and emit a snapshot independently of optional meter transport. A new telemetry subsystem or duplicate UI-derived voice-policy cache is unnecessary. No other material design alternative requires widening this slice.

## Publication and disposition

Verdict is **FAIL**, not a conditional PASS. No proposed post-PASS metadata transformation is approved for this failing candidate. Repair F1 using the normal TDD and proportionate verification workflow, regenerate the complete source/evidence manifest, and return the whole frozen candidate to this same reviewer. The exact five-path metadata envelope can be assessed after a subsequent PASS; normal hooks, unchanged-upstream fetch and non-force publication remain required then.

Reusable lesson candidate: service suppression can accidentally suppress publication of accepted non-measured metadata if both share one callback. Evidence is F1's 8→12 transition. Proposed owner is this frontend's review/test notes; future trigger is disabling a telemetry service. Add a distinct accepted-state refresh assertion. Downside is redundant tests if repeated at every UI layer, so keep the policy assertion at the adapter/Store boundary. Disposition: recorded here only, no standing rules or memory changed.
