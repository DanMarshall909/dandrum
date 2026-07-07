## 1. Primitive Inventory and Gaps

- [x] 1.1 Inventory existing primitives usable for 808/909-style drum voices and document which voices can be built without new DSP modules.
- [x] 1.2 Add or refine reusable primitive support needed for drum voices, prioritising oscillator waveform selection and frequency-oriented tuning before bespoke drum modules.
- [x] 1.3 Ensure decay/envelope parameters exposed through `preset_surface.parameters` can update mapped runtime targets without rewriting YAML.

## 2. 808-Style Voices

- [x] 2.1 Author initial 808-style kick, snare, tom/conga, clap, and cowbell instrument graphs from primitives where practical.

## 3. 909-Style Voices

- [x] 3.1 Author initial 909-style kick, snare, tom, and clap instrument graphs from primitives where practical.
- [x] 3.2 Use sampler-backed assets for 909-style hats, crash, and ride only where primitive synthesis is not accurate or practical.

## 4. Reference Parameter Seeding

- [x] 4.1 Research documented, free, or open 808/909-style synths and public references for seed values such as tune, decay, sweep, click, snappy/noise, tone, and drive.
- [x] 4.2 Convert accepted reference values into Dandrum parameter ranges and defaults, preserving source/provenance notes in implementation documentation.
- [x] 4.3 Explicitly avoid copying proprietary samples, source code, or preset banks into the repository.

## 5. Tests and Offline Tuning

- [x] 5.1 Add tests proving seeded drum instruments load, expose their declared public parameters, and render non-silent output from MIDI trigger events.
- [x] 5.2 Add an offline spectral/envelope comparison plan for tuning synthesized drum voices against reference samples outside the realtime callback.
