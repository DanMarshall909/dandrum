## Context

The CLAP increment is published at `2c7bcaa5`, with all 14 tasks and 13 added scenarios verified. It provides the real React/Wry editor, a persistent format-neutral session, capacity-one runtime handoff/reclamation, bounded protocol/parameters and packaged-kit state. Its official validator result remains 33 successes/10 skips/1 failure at fractional 1234.5678 Hz. That failure is not waived by completing the bounded editor increment.

Readiness evidence is retained under `/tmp/dandrum-evidence/{vst3-current-readiness,selected-sample-control-readiness,beta-sample-loading-readiness,fractional-rate-readiness}`. The wrapper candidate has not been built as this product. Its VST3 parameter/state implementations exist, but Linux idle/timer/flush servicing depends on an attached IRunLoop and GUI methods ignore failures; standalone has separate parameter-service stubs and argv0/resource limitations. The cached official SDK documentation lists a newer compiler/platform than currently installed; actual qualification remains necessary.

Selected-sample preparation is also absent. The frontend can author a retained sampler patch with the kit's 23 aliases, but a default single-file stack overlaps the kit, New Empty drops its root control surface, and nondefault values are baked into root defaults that current runtime compatibility compares literally. Existing frontend fake bridges do not establish real acceptance. Version2 state deliberately allows only the packaged WAV.

## Goals / Non-Goals

**Goals:**

- Complete the full first JUCE-free Trigger beta on the selected Linux X11/WebKitGTK platform.
- Reuse the same persistent host/session and realtime ownership laws across CLAP, VST3 and standalone.
- Make real native selection, owned sample loading, patch publication/mapping/history, signed playback and fresh portable restoration work through the production editor.
- Prove device/callback ownership, native input/lifecycle, named-DAW automation/two instances, and installed offline artifacts without JUCE.
- Keep failure atomicity, bounded work, mutation exclusion, normal hooks and predictable disposable-output cleanup.

**Non-Goals:**

- Windows/macOS/Wayland release support or every possible DAW/device combination.
- New DSP primitives, new sample decoder formats, unrestricted plugin/patch architectures or automatic new per-sound automation banks.
- Treating a wrapper return code, mocked bridge, cached source inspection, synthetic pacing or map-existence check as native acceptance.

## Decisions

### Keep one instrument owner; qualify concrete product edges first

The host/session remains free of Clack, Wry, GTK, WebKit, VST3 and JUCE types. VST3 and standalone adapt lifecycle, parameter/state events, views and device ownership at the edge. They do not acquire separate accepted documents/history or reinterpret the engine's signed output.

First run a bounded actual loader/state/parameter/PCM probe of the pinned wrapper candidate and MIT SDK, with source/tool/license identities and one build configuration at a time. The candidate pins are clap-wrapper v0.16.0 `1cca996e96f29ab2be7ae9f8cfe532bbc92e1dd6` and SDK v3.8.0_build_66 `9fad9770f2ae8542ab1a548a68c1ad1ac690abe0`; transitive sources and compiler qualification are still required. A successful subset build on an older compiler is evidence of that exact combination, not the SDK's documented platform combination.

Adopt only a route that proves exact product/resource identity, real signed audio, parameters and fresh state. Investigate or repair project-owned edge behavior for stopped/editor-absent control service and real GUI failure propagation before broad integration. Direct SDK/ABI projection is an alternative if the candidate cannot meet these contracts; it must pass the same tests, not change the format-neutral instrument design. Standalone device ownership is qualified separately; inspected standalone stubs are not accepted behavior.

### Own immutable selected sample snapshots in the session resource authority

Native selection returns actual decoded metadata and a bounded authority for owned immutable encoded WAV bytes, not a browser-supplied path assertion. File reading/decoding/retention occurs off audio. Sources admitted into accepted, pending or historical patch states retain their resource ownership independently of the view and original selected path.

Use content identities and canonical resource-relative references. The frontend may refer only to admitted resources; whole-document validation rejects arbitrary paths and external references before preparation. Existing filesystem preparation may consume a private owned materialization, but that is a backing implementation, never the serialized authority or a cwd/executable/checkout fallback. Resource reclamation and temporary-file deletion occur on control/workers after the last relevant owner releases them, never during rendering.

Initial resource budgets to qualify before the first import implementation are 16 files per selection,64 MiB per encoded asset and256 MiB aggregate retained encoded content per instance, with bounded decoded frames, source/region counts and document/manifest sizes. Measure peak memory before fixing decoded limits. Report exact public limits; oversize rejection preserves the accepted instrument. Original path deletion/replacement cannot alter already admitted bytes.

A global mutable cache or serialized absolute path would be smaller but would not establish fresh offline sample restoration or reliable cleanup ownership. Reusing immutable decoded engine handles is appropriate; duplicating whole build caches or retaining every transient sample stage is not.

### Publish interactive patch edits through native history

Expose bounded whole-document preparation/admission/status operations over the same session owner. Interactive imports and related patch edits must have native inverses/history and once-only outcomes; external state replacement may keep its existing fresh-history semantics. Preparation and candidate resource ownership outlive editor destruction after admission. Before admission, cancellation/stale view cleanup releases only view-owned staged work.

Preserve the current product's stable public aliases and physical values. New UI sounds do not manufacture automation aliases. Separate compatible port identity/type/range from mutable control defaults only where tests prove runtime/controller agreement and preservation of nondefault host values. Do not suppress incompatible layouts. New Empty must produce a truly silent accepted patch while preserving the supported product interface, so subsequent import is a real transition. A single-file import must present or choose a valid mapping rather than silently stack across occupied kit keys or overwrite accepted zones.

Extend the versioned transport with explicit supported methods and measured bounds. Keep existing version 1 behavior and retained JUCE comparison consumers compatible; negotiate any larger/new wire version rather than silently enlarging old malformed-input acceptance. Real frontend serialization and native controller tests must meet at this boundary; fake bridges cannot qualify it.

### Save a bounded self-contained patch, resources and named values

Use a new versioned container carrying the accepted patch, canonical resource manifest, owned encoded sample snapshots/content identities and named physical parameter values. Avoid base64 copies of large audio buffers; bound framing/counts/lengths before allocating or preparing. Validate content identity and the complete candidate before any accepted publication. Retain readable version 2 packaged-kit state and explicitly reject unsupported versions and incompatible surfaces.

Host state and standalone patch Save/Load use this authority. A fresh instance must restore after original selected paths and the original instance are gone, without a shared source/cache directory. Accepted output, mappings and values must match independently expected signed PCM. Missing/corrupt/oversized/short-reading state preserves the prior session/history/feedback/audio. Replacing only an external path label or round-tripping the same live object is insufficient.

### Preserve requested fractional host clocks through preparation

Introduce an explicit validated realtime sample-rate representation retaining the requested floating-point clock across configuration identity, session preparation/reconfiguration, sample-rate conversion and runtime creation. Preserve the previous supported integer domain and add supported fractional values including 1234.5678 and 48000.5. WAV source metadata remains integer. Existing DSP numeric precision may remain explicit; integer rounding/truncation of the host clock is forbidden.

Prove signed sample timing with an independent source waveform/rate oracle, failed-reconfiguration rollback, independent instances and the actual official-validator case. Keep device-negotiated rates separate from requested-but-unavailable device formats. Do not cast away precision merely to make activation succeed.

### Treat actual native and DAW behavior as release gates

Use the production bundle and actual X11 input/parent lifecycle, not browser method calls alone, for selected-file and control flows. Verify destruction/recreation with continuing native audio, retained edits/history and exactly-once note/gesture cleanup. Preserve separate synthetic-host and physical/native evidence levels.

The named-DAW target is Qtractor, subject to qualification of its actual build/plugin support and host loop; record the tested version/configuration. Exercise real VST3 automation and two independent instances, including editor-absent service, project save/fresh reopen and selected PCM. If that target cannot provide a required contract, retain the exact limitation and qualify a named alternative without silently relabelling a synthetic host as a DAW.

Standalone owns explicit audio/MIDI device handles and the app main loop. Its complete render callback, including adapter event translation, must remain allocation/deallocation/lock/file/serialization/browser/destruction free. Device/editor/worker shutdown must have deterministic owners; never stop unrelated user services to make a test pass.

### Keep verification storage bounded and hooks honest

All build/coverage targets are physical disk. Reuse compatible targets/dependencies; one heavy job at a time with 8 GiB MemAvailable reserve and pre-run disk/target-size checks. Retain source/tool/config identities, raw machine results, compact logs and required reusable baselines outside disposable runs. Supervisors must stop/verify owned workers and clean exact owned scratch output after success, failure and cancellation, with recovery after controller death. Unknown ownership stops deletion; reusable targets/caches still need explicit direction for eviction.

Mutation remains excluded. Before any changed-engine publication, make the maintained normal-hook policy honor that recorded exclusion without bypassing hooks or weakening ordinary tests/coverage. Test any policy/driver change behavior-first. Do not allow the existing changed-engine pre-push selector to launch mutation contrary to user direction.

## Risks / Trade-offs

- [Unqualified SDK/compiler or wrapper lifecycle] → One actual small boundary probe first; retain exact failures and choose/repair a proven edge before dependent integration.
- [Frontend documents differ from frozen host assumptions] → Real selected-file/empty/nondefault-value tests at serialization and host admission; preserve aliases and reject true incompatibility atomically.
- [Large state/decoded samples consume RAM or disk] → Measured advertised bounds, streaming framing, one owner for retained resources and terminal/recovery cleanup.
- [View-dependent host callbacks stall accepted work] → Explicit stopped/editor-absent tests and native host-loop ownership; no browser/audio-thread workaround.
- [Installed files accidentally fall back to checkout or caches] → Exact artifact/resource identities, isolated offline source-hidden runs and decoy/missing-resource rejection.
- [Broad beta proof confused with earlier component PASS] → Requirement-to-test/native/DAW/artifact matrix, independent material reviews and final full-goal audit.

## Migration Plan

Keep existing CLAP identities/fixtures, version 1 transport consumers and version 2 packaged-kit state readable. Add opt-in product targets, qualify each edge, then enable the beta artifact set only after its real acceptance gates. Rollback preserves prior installed artifacts and accepted state; failed preparation or state loads never partially publish. Commit and normally publish verified material boundaries, keeping main unchanged.

## Open Questions

Task 1 resolves the actual compiler/dependency layout, wrapper adoption and supported DAW/device fixture availability. Resource/decoded-memory and transport framing limits must be measured before import implementation. These investigations do not authorize weaker loader, native, persistence or no-JUCE acceptance; unresolved prerequisites block only dependent implementation while independent ready work continues.
