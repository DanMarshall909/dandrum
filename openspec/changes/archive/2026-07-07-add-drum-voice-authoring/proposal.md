## Why

Dandrum's initial drum library should prove that convincing 808/909-style voices can be built from the existing primitive graph model, rather than requiring bespoke machine-specific DSP modules. Existing free/open synths and public references can provide useful seed values for public parameters, but those values should be treated as initial tuning hints rather than ground truth. Later offline spectral and envelope analysis against reference samples can tune the synthesized voices more systematically.

This was originally scoped as part of `add-juce-plugin-integration` (task section 13), but it is primarily content/research work (inventorying primitives, researching reference synths, authoring instrument graphs, seeding parameter values) rather than plugin-runtime engineering. Splitting it into its own change lets it proceed on its own pace/review cadence, independent of the plugin shell's implementation schedule.

## What Changes

- Inventory existing Dandrum primitives usable for 808/909-style drum voices, and document which voices can be built without new DSP modules.
- Add or refine reusable primitive support needed for drum voices, prioritising oscillator waveform selection and frequency-oriented tuning before any bespoke drum modules.
- Ensure decay/envelope parameters exposed through `preset_surface.parameters` update mapped runtime targets without rewriting YAML.
- Author initial 808-style kick, snare, tom/conga, clap, and cowbell instrument graphs from primitives where practical.
- Author initial 909-style kick, snare, tom, and clap instrument graphs from primitives where practical.
- Use sampler-backed assets for 909-style hats, crash, and ride only where primitive synthesis is not accurate or practical.
- Research documented, free, or open 808/909-style synths and public references for seed values (tune, decay, sweep, click, snappy/noise, tone, drive).
- Convert accepted reference values into Dandrum parameter ranges and defaults, preserving source/provenance notes in implementation documentation.
- Add an offline spectral/envelope comparison plan for tuning synthesized drum voices against reference samples, outside the realtime callback.

## Capabilities

### New Capabilities
- `drum-voice-authoring`: primitive-first 808/909-style drum voice authoring, reference-seeded parameter defaults, and an offline spectral-comparison tuning workflow.

### Modified Capabilities
- (none)

## Impact

- Requires seeded 808/909-style instrument definitions (YAML) to exercise the primitive graph model and public parameter surface.
- Requires later offline analysis tooling to compare rendered drum voices against reference samples and tune public parameter values.
- Requires tests proving seeded drum instruments load, expose their declared public parameters, and render non-silent output from MIDI trigger events.
- Explicitly excludes copying proprietary drum-machine samples, plugin source, or preset banks into the repository.
