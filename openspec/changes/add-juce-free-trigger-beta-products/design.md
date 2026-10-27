## Context

The CLAP editor foundation is published at `2c7bcaa5`, with all 14 bounded-change tasks and 13 added scenarios verified. It supplies the real React/Wry editor, persistent format-neutral session, capacity-one runtime handoff/reclamation, bounded protocol/parameters and packaged-kit state. The foundation official validator result was **33 successes / 10 skips / 1 failure at fractional 1234.5678 Hz**. The precise-clock candidate now records **34 successes / 10 skips / zero failures**, including the exact varying-sample-rates case, on a copied GUI-enabled release artifact with a freshly built production React bundle. No test exclusions or waiver were used. The retained prior failure, new source/product identities, skip reasons and signed waveform tests are in `docs/migrations/evidence/trigger-host-clap-precise-clocks-2026-10-23/`; independent review and normal publication remain required for this candidate.

On 22 October 2026 the user narrowed the first beta to **CLAP-only**. This supersedes the earlier VST3/standalone beta scope without discarding completed work. At that scope revision, tasks 1.1 and 1.2 had been published; task 1.3's owned-run cleanup helper/tests were still in progress with retained RED/GREEN evidence and had not yet been submitted for independent review. That work was preserved through the scope revision, subsequently passed independent review and was normally published at `c112ed10`.

Selected-sample preparation and fresh portable restoration remain missing. Existing readiness evidence under `/tmp/dandrum-evidence/{selected-sample-control-readiness,beta-sample-loading-readiness,fractional-rate-readiness}` remains useful. Default single-file stacks overlap the kit, New Empty drops the root control surface, and current compatibility compares mutable control defaults. Fake frontend bridges do not establish native acceptance. Version 2 state currently permits only the packaged WAV.

VST3/standalone readiness reports remain historical inputs. No product adapter or wrapper dependency was adopted. Later product preferences and feasibility gates are in `future-products.md`; no VST3 or standalone implementation starts in this increment.

## Goals / Non-Goals

**Goals:**

- Complete the first JUCE-free Trigger CLAP beta on Linux x86_64 X11/WebKitGTK 2.50.4.
- Keep the current CLAP adapter over the persistent format-neutral TriggerHost/session, existing Rust engine and production React/Wry editor.
- Prove actual native sample selection/loading/mapping, signed playback, retained edits/native undo across editor recreation and fresh-instance patch/sample/parameter restoration.
- Prove complete CLAP callback ownership, named-DAW automation/independent instances and installed offline artifacts with complete JUCE-free build/runtime/linkage evidence.
- Resolve the outstanding fractional-rate validator failure while retaining failure atomicity, bounded work, mutation exclusion, normal hooks and owned disposable-output cleanup.

**Non-Goals:**

- VST3 or standalone implementation, standalone device ownership or standalone file Save/Load.
- Replacing the current architecture with a broader plugin framework.
- Windows/macOS/Wayland release support, every DAW/device combination, new DSP primitives, decoder formats or unrestricted plugin/patch architectures.
- Counting a mocked bridge, wrapper return code, source inspection or synthetic host as native or named-DAW acceptance.

## Decisions

### Preserve the existing CLAP projection and one instrument owner

TriggerHost/session stays free of Clack, Wry, GTK, WebKit, VST3 and JUCE types. The existing CLAP edge owns format lifecycle, host parameters/state, native parent/main-loop integration and bounded callback translation. It does not acquire a separate accepted document or history. Audio/MIDI devices remain the host's responsibility; the beta must still prove actual host audio/MIDI callbacks and native input/lifecycle.

VST3 and standalone are later increments. The later VST3 preference is a direct Rust adapter using `coupler-rs/vst3-rs` over TriggerHost, beginning with the bounded proof in `future-products.md`. Reuse the engine/session/runtime/parameter/state/editor machinery. Treat the gain example only as an ABI reference and retain clap-wrapper only as an evidence-justified fallback. Do not begin either product here.

### Own immutable selected sample snapshots in the session resource authority

Native selection returns actual decoded metadata and a bounded authority for owned immutable encoded WAV bytes, not a browser-supplied path assertion. File reading/decoding/retention occurs off audio. Sources admitted into accepted, pending or historical patch states retain their resource ownership independently of the view and original selected path.

Use content identities and canonical resource-relative references. The frontend may refer only to admitted resources; whole-document validation rejects arbitrary paths and external references before preparation. Existing filesystem preparation may consume a private owned materialization, but that is a backing implementation, never the serialized authority or a cwd/executable/checkout fallback. Resource reclamation and temporary-file deletion occur on control/workers after the last relevant owner releases them, never during rendering.

Initial resource budgets to qualify before the first import implementation are 16 files per selection,64 MiB per encoded asset and256 MiB aggregate retained encoded content per instance, with bounded decoded frames, source/region counts and document/manifest sizes. Measure peak memory before fixing decoded limits. Report exact public limits; oversize rejection preserves the accepted instrument. Original path deletion/replacement cannot alter already admitted bytes.

The measured decoder qualification and the first selected-resource contract are recorded in `selected-samples.md`. Implement the task 5.1 foundation together with only the selected-resource admission/status portion of 5.2 and the necessary trusted chooser ingress from 5.4. Keep all three checkboxes open until their complete criteria pass. This dependency slice must start with an executable copied-product chooser/result RED, not a missing-API compiler failure or a parser-only success.

A global mutable cache or serialized absolute path would be smaller but would not establish fresh offline sample restoration or reliable cleanup ownership. Reusing immutable decoded engine handles is appropriate; duplicating whole build caches or retaining every transient sample stage is not.

### Publish interactive patch edits through native history

Expose bounded whole-document preparation/admission/status operations over the same session owner. Interactive imports and related patch edits must have native inverses/history and once-only outcomes; external state replacement may keep its existing fresh-history semantics. Preparation and candidate resource ownership outlive editor destruction after admission. Before admission, cancellation/stale view cleanup releases only view-owned staged work.

Preserve the current product's stable public aliases and physical values. New UI sounds do not manufacture automation aliases. Separate compatible port identity/type/range from mutable control defaults only where tests prove runtime/controller agreement and preservation of nondefault host values. Do not suppress incompatible layouts. New Empty must produce a truly silent accepted patch while preserving the supported product interface, so subsequent import is a real transition. A single-file import must present or choose a valid mapping rather than silently stack across occupied kit keys or overwrite accepted zones.

Extend the versioned transport with explicit supported methods and measured bounds. Keep existing version 1 behavior and retained JUCE comparison consumers compatible; negotiate any larger/new wire version rather than silently enlarging old malformed-input acceptance. Real frontend serialization and native controller tests must meet at this boundary; fake bridges cannot qualify it.

The subsequent bounded full-patch/profile contract and its initial executable
proof are specified in `interactive-patches.md`. Its proposed budgets must be
qualified through the owning tests before advertising a capability; no task or
beta closure follows from negotiation alone.

### Save a bounded self-contained patch, resources and named values

Use a new versioned container carrying the accepted patch, canonical resource manifest, owned encoded sample snapshots/content identities and named physical parameter values. Avoid base64 copies of large audio buffers; bound framing/counts/lengths before allocating or preparing. Validate content identity and the complete candidate before any accepted publication. Retain readable version 2 packaged-kit state and explicitly reject unsupported versions and incompatible surfaces.

CLAP host state uses this authority. A fresh instance must restore after original selected paths and the original instance are gone, without a shared source/cache directory. Accepted output, mappings and values must match independently expected signed PCM. Missing/corrupt/oversized/short-reading state preserves the prior session/history/feedback/audio. Replacing only an external path label or round-tripping the same live object is insufficient.

### Preserve requested fractional host clocks through preparation

Introduce an explicit validated realtime sample-rate representation retaining the requested floating-point clock across configuration identity, session preparation/reconfiguration, sample-rate conversion and runtime creation. Preserve the previous supported integer domain and add supported fractional values including 1234.5678 and 48000.5. WAV source metadata remains integer. Existing DSP numeric precision may remain explicit; integer rounding/truncation of the host clock is forbidden.

Prove signed sample timing with an independent source waveform/rate oracle, failed-reconfiguration rollback, independent instances and the actual official-validator case. Keep device-negotiated rates separate from requested-but-unavailable device formats. Do not cast away precision merely to make activation succeed.

The engine implementation preserves integer `RenderSettings` inputs/serialization and integer C ABI entrypoints. Those inputs adapt to a common `RealtimeRenderSettings` containing a private validated `SampleRate` in the supported domain `1..=u32::MAX` Hz. Compiled roots and poly children store that one effective configuration; preparation contexts must agree exactly, including distinctions below f32 precision. WAV/source metadata stays integer and strict matching-rate resources reject a fractional mismatch explicitly. Source/host division and sample cursors retain f64 precision; audio sample values and musical control values remain f32. The session, controller and CLAP adapter carry this same validated rate through exact configuration matching and atomic reconfiguration. Fractional runtime tests exercise real cross-thread ownership, capacity-one replacement/reclamation, reset and independent sample cursors with zero callback allocations/deallocations. Controller tests retain native mapping/Undo history and physical values, reject failed preparation atomically, and restore packaged version-2 state into a fresh instance at a different precise clock. The actual copied-product official varying-sample-rates case now passes; native selected-sample, installed and named-DAW release acceptance remain separate later gates.

### Treat actual native CLAP and DAW behavior as release gates

Use the production bundle and actual X11 input/parent lifecycle, not browser method calls alone, for selected-file and control flows. Verify destruction/recreation with continuing native audio, retained edits/history and exactly-once note/gesture cleanup. Preserve separate synthetic-host and physical/native evidence levels.

Qualify a named DAW with actual CLAP support and record its version/configuration and host-loop behavior. The earlier Qtractor candidate is not an assertion of CLAP support. Exercise real CLAP automation and two independent instances, including editor-absent service, project save/fresh reopen and selected PCM. If a candidate cannot provide a required contract, retain the limitation and qualify a named alternative; a synthetic host does not replace DAW acceptance.

The complete CLAP render callback, including adapter event translation, must remain allocation/deallocation/lock/file/serialization/browser/destruction free. The host owns devices; CLAP/editor/worker shutdown must release only instance-owned resources. Retain physical/native host callback evidence separately from synthetic-host proof and never stop unrelated user services to make a test pass.

### Keep verification storage bounded and hooks honest

All build/coverage targets are physical disk. Reuse compatible targets/dependencies; one heavy job at a time with 8 GiB MemAvailable reserve and pre-run disk/target-size checks. Retain source/tool/config identities, raw machine results, compact logs and required reusable baselines outside disposable runs. Supervisors must stop/verify owned workers and clean exact owned scratch output after success, failure and cancellation, with recovery after controller death. Unknown ownership stops deletion; reusable targets/caches still need explicit direction for eviction.

Mutation remains excluded. Before any changed-engine publication, make the maintained normal-hook policy honor that recorded exclusion without bypassing hooks or weakening ordinary tests/coverage. Test any policy/driver change behavior-first. Do not allow the existing changed-engine pre-push selector to launch mutation contrary to user direction.

## Risks / Trade-offs

- [Unqualified CLAP DAW/native host loop] → Qualify the actual format support and host services, retaining exact limitations before native acceptance.
- [Frontend documents differ from frozen host assumptions] → Real selected-file/empty/nondefault-value tests at serialization and host admission; preserve aliases and reject true incompatibility atomically.
- [Large state/decoded samples consume RAM or disk] → Measured advertised bounds, streaming framing, one owner for retained resources and terminal/recovery cleanup.
- [View-dependent host callbacks stall accepted work] → Explicit stopped/editor-absent tests and native host-loop ownership; no browser/audio-thread workaround.
- [Installed files accidentally fall back to checkout or caches] → Exact artifact/resource identities, isolated offline source-hidden runs and decoy/missing-resource rejection.
- [Broad beta proof confused with earlier component PASS] → Requirement-to-test/native/DAW/artifact matrix, independent material reviews and final full-goal audit.

## Migration Plan

Keep existing CLAP identities/fixtures, version 1 transport consumers and version 2 packaged-kit state readable. Extend the shared session and current CLAP projection, then install the CLAP beta only after real acceptance gates pass. Failed preparation/state loads preserve accepted state. Commit and normally publish verified boundaries, preserve the verified cleanup boundary and keep main unchanged at `68dd655`.

VST3 and standalone require later OpenSpec increments. Removing their active tasks records deferral, not completion. Existing task identifiers for retained CLAP work remain stable; the scope revision retained 17 active tasks, with only the qualification/hook tasks complete at that point. Current implementation checkboxes are recorded in `tasks.md`; later publication evidence preserves each verified boundary.

## Open Questions

Qualify a named DAW's actual CLAP support, selected native host/callback fixture and resource/decoded-memory/transport framing limits before dependent work. Keep the fractional-rate validator failure explicit until the clock implementation and exact official case pass. Later VST3 feasibility, dependency pinning and standalone device choices do not block this CLAP beta and must not be started as part of it.
