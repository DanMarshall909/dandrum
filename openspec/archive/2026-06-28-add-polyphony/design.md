## Context

The current Rust engine loads YAML patches, validates typed graph routing, and renders blocks deterministically without GUI or plugin coupling. MIDI events currently flow through a single graph instance, and sampler processing is intentionally monophonic, so overlapping notes replace prior playback instead of producing independent voices.

Polyphony must fit the existing modular graph model: named ports, explicit cables, explicit mixers for many-to-one routing, and deterministic offline rendering. The change should not introduce GUI, plugin, or realtime-driver concerns.

## Goals / Non-Goals

**Goals:**

- Let a patch declare voice allocation settings such as `max_voices` and voice-stealing policy.
- Allocate independent voices from MIDI note-on events and release matching voices from note-off events.
- Process each active voice as a voice-local sub-synth with independent module state, local routing, local modulation, and adjustable sub-synth output.
- Let each voice sub-synth route through voice-local controls and a voice-local sub-bus before all active voices are mixed together.
- Keep mixed signal-family interactions explicit through converter, merger, or polymorphic modules.
- Preserve shared/global module processing for outputs, mixers, effects, and patch-level routing.
- Keep offline rendering deterministic for identical patches, settings, assets, and events.
- Validate invalid voice settings and unsupported voice/global routing before rendering.

**Non-Goals:**

- No MPE, channel-per-voice expression, sustain pedal, legato, glide, unison, or voice stacking in this change.
- No GUI voice visualizer or patch editor behavior.
- No dynamic runtime changes to polyphony settings while rendering.
- No automatic implicit summing into ordinary single-value inputs.

## Decisions

### Decision: Patch-level voice settings

Add an optional patch-level `voice_allocation` section rather than embedding polyphony settings in individual modules. The patch is the instrument boundary, and a single allocator can consistently manage note ownership, stealing, and release across all voice-scoped modules.

Alternatives considered:

- Module-local polyphony parameters: flexible, but creates competing allocators and ambiguous note ownership.
- CLI-only voice count: useful for experiments, but hidden from patch validation and not portable.

### Decision: Explicit module execution scope

Classify modules as `voice` or `global` during graph preparation. Voice-scoped modules form a voice-local sub-synth: each allocated voice gets its own state instances, local connections, modulation paths, and sub-synth output routing. Global modules keep one shared state instance per patch. Built-in sound sources, per-note envelopes, per-note pitch conversion, voice-local mixers, and voice-local modulation are voice-scoped; MIDI input, audio output, and shared effects/mixers are global by default.

Alternatives considered:

- Duplicate the entire patch graph per voice: simpler conceptually, but duplicates global outputs/effects and makes voice-to-global routing implicit.
- Make all modules global with voice IDs in event payloads: smaller state change, but pushes polyphony complexity into every module.

### Decision: Deterministic oldest-active voice stealing

When all voices are active and the patch allows stealing, note-on allocation SHALL steal the oldest active voice. Ties use stable voice slot order. This is simple, deterministic, and testable.

Alternatives considered:

- Quietest voice stealing: musically useful, but requires amplitude tracking and can vary with later DSP changes.
- Reject new notes when full: valid as a policy, but too limiting as the only MVP behavior.

### Decision: Voice sub-synths have explicit adjustable outputs

Each active voice may produce one or more voice-local sub-synth outputs before crossing into global processing. Inside the sub-synth, per-voice audio/control outputs may feed voice-local mixers, VCAs, filters, envelopes, and other voice-local controls. The resulting adjustable sub-synth output may then feed global mixer inputs that explicitly accept multiple sources. The engine SHALL NOT implicitly sum multiple voices into arbitrary single-source inputs. This preserves the existing routing rule that many-to-one behavior requires an explicit mixer or summing module.

Alternatives considered:

- Automatically sum every voice at voice/global boundaries: convenient, but conflicts with existing explicit routing semantics.
- Require a new voice bus signal type immediately: precise, but increases scope beyond the minimum needed to prove polyphony.

### Decision: Voice-local sub-synths are graph scope, not hidden channels

Represent a voice sub-synth as ordinary voice-scoped routing and modules rather than a hidden engine channel. This keeps voice behavior inspectable in YAML and lets patch authors build per-voice chains such as oscillator plus sampler into a voice-local mixer, through a voice-local VCA/filter, before the final polyphonic fan-in.

Alternatives considered:

- Hidden engine-managed voice channel: easier for a fixed synth architecture, but conflicts with the modular patch model.
- One global bus per voice slot: deterministic, but leaks implementation slots into patch authoring and makes voice stealing observable in routing.

### Decision: Mixed signal families require explicit modules

Same-type signals may be mixed through matching mixer modules, such as audio mixers for audio, control mixers for control, and event merge modules for events. Different signal families may interact only through modules that explicitly declare conversion, merging, or polymorphic input behavior, such as note-to-rate, velocity-to-control, envelope follower, gate extractor, threshold/comparator, or control-to-audio modules.

Alternatives considered:

- Implicitly coerce compatible-looking signal types: convenient, but creates hidden behavior and surprising patch results.
- Forbid all cross-type interaction: simple, but blocks useful sound-design techniques such as audio envelope followers and event-to-control modulation.

### Decision: Reuse block scheduling with sample-offset events

Voice allocation happens as input events are sequenced into blocks. The allocator preserves event frame offsets so note-on/note-off behavior remains deterministic at the same resolution as the current scheduler.

Alternatives considered:

- Allocate voices only at block boundaries: simpler, but loses existing event offsets and makes overlapping-note tests less precise.
- Add a separate offline-only scheduler: breaks the engine-first model intended for future realtime use.

## Risks / Trade-offs

- Voice/global scope classification may be too coarse for future modules -> keep scope metadata explicit and validated so new modules can choose the correct scope.
- Oldest-active stealing is not always the most musical behavior -> define it as the MVP policy and leave room for additional policies later.
- Per-voice state multiplies memory and CPU cost -> enforce positive finite `max_voices` and keep the initial tests small.
- Voice sub-synth and voice-to-global routing can be confusing -> produce validation diagnostics that identify the crossing and name the required explicit mixer or voice-local output path.
- Cross-type signal interaction can become ambiguous -> require explicit converter, merger, or polymorphic module declarations and reject implicit mixed-type routing.
- Existing monophonic patches should remain valid -> default missing `voice_allocation` to one voice without stealing unless a patch opts in.
