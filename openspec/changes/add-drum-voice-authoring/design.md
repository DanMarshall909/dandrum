## Context

Dandrum's graph model already supports a broad set of reusable primitives: oscillator, decay/envelope, gain/multiply, filter, saturator, impulse, mixer, curve mapper, note-to-control, and audio output modules, composed through the existing composite-module (`module_definitions`) mechanism. The `synthetic-808-kick.yaml` example already demonstrates a full drum voice (tuned body oscillator, pitch-decay envelope, click transient, sub-oscillator layer) built entirely from these primitives, with public parameters exposed through `preset_surface.parameters`.

This change formalizes and extends that approach into an initial 808/909-style drum library, explicitly preferring primitive composition over bespoke per-drum DSP modules, and treats free/open reference synths as a source of initial parameter seed values rather than ground truth.

## Goals / Non-Goals

**Goals:**
- Prove that convincing 808/909-style drum voices can be built from Dandrum's existing primitive graph model.
- Identify and close any real primitive gaps (e.g. oscillator waveform selection, frequency-oriented tuning) before reaching for a bespoke drum module.
- Seed initial public parameter defaults from documented, free, or open reference instruments, with provenance recorded.
- Establish an offline (non-realtime) workflow for later spectral/envelope-based tuning against reference samples.

**Non-Goals:**
- No new bespoke `808_kick`/`909_snare`-style module types unless the primitive graph genuinely proves impractical, too slow, or sonically inadequate for a specific voice.
- No copying of proprietary drum-machine samples, plugin source code, or preset banks into the repository.
- No realtime spectral fitting or reference-sample analysis inside the plugin audio callback — all comparison/tuning work is an offline tool.
- No commitment to matching any specific commercial machine's exact sound; reference values are tuning hints, not a compliance target.

## Decisions

### Primitives first, bespoke modules last

Each drum voice is authored as an ordinary Dandrum instrument graph. Before adding any drum-specific module type, the author must show the voice is impractical, too slow, or sonically inadequate using existing primitives (oscillator, decay, gain, filter, saturator, impulse, mixer). This mirrors the pattern already established by `synthetic-808-kick.yaml`.

Where primitives are genuinely insufficient (e.g. a control path that only makes sense generalized, like frequency-oriented oscillator tuning), the fix is a general-purpose primitive improvement, not a drum-specific module.

### Sampler assets are a fallback, not a default

909-style hats, crash, and ride are the one case where sampler-backed assets are explicitly permitted as the default, since primitive synthesis of high-frequency metallic transients is a known hard problem. Kick, snare, tom, and clap voices are expected to be synthesizable from primitives across both 808 and 909 styles.

### Reference values are seeds, not sources of truth

Free/open synths, public documentation, and permissively inspectable presets may inform initial `preset_surface.parameters[*].default` values, but:
- values are converted into Dandrum's own parameter ranges/units, never copied verbatim as code or assets;
- the source and rationale are documented near the authored instrument or in implementation notes;
- disagreement between sources is resolved conservatively, favoring musically useful defaults.

### Offline spectral comparison tunes later, not now

A later, separate offline tool renders candidate voices and compares them against reference samples (spectral, amplitude-envelope, transient, decay-tail metrics), proposing or applying parameter adjustments. This tool operates entirely outside the realtime audio callback and only ever changes public parameter values or authored YAML defaults — never hidden graph structure.

## Risks / Trade-offs

- **Primitive gaps may still surface**: some 808/909 voices (e.g. resonant handclap noise bursts, complex FM-like kick pitch sweeps) may reveal real primitive gaps only once authoring is attempted. Treat these as primitive-improvement tasks first; only fall back to a bespoke module if a primitive fix is proven impractical.
- **Reference-value provenance risk**: seeding from public/free synth documentation requires care to avoid inadvertently copying licensed preset banks or proprietary sample content. Provenance notes are required precisely to make this auditable.
- **No objective quality bar yet**: without the offline spectral-comparison tool (a later task in this same change), initial seeded voices are tuned by ear only. This is acceptable for an initial library but should not be treated as final tuning.
