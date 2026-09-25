## Context

`examples/patches/tb303-acid.yaml` is a monophonic subtractive voice assembled from general-purpose modules. Its saw oscillator feeds the existing Moog filter, but the patch currently drives cutoff with an almost full-range envelope plus a large velocity offset, uses a 250 ms filter decay, and leaves resonance at the filter primitive's zero default. A held-note comparison against a genuine TB-303 recording showed an initial spectral centroid around 5 kHz followed by a collapse to roughly 155 Hz by 250 ms, while the hardware reference remained spectrally active across seconds.

Free software clones and available hardware recordings can guide listening during this proof of concept, but they do not define exact acceptance values. A later project can record controlled fixtures from a real TB-303 with a Devil Fish modification. This change therefore separates the reusable stimulus-and-analysis workflow from reference-specific thresholds.

## Goals / Non-Goals

**Goals:**

- Remove the patch's immediate bright-then-static filter collapse.
- Drive the resonant filter deliberately and keep gain staging finite and usable.
- Preserve the existing monophonic MIDI, velocity-accent, and legato-slide behaviour.
- Establish a deterministic fixture, audition render, and frame-by-frame spectral/level report that can later consume controlled clone and hardware observations.
- Keep third-party audio, installers, credentials, and licence material out of the repository.

**Non-Goals:**

- Claim sample-accurate equivalence with either a physical TB-303 or ABL3.
- Treat the Devil Fish modification as equivalent to a stock TB-303.
- Add a product-specific Rust primitive or reverse-engineer proprietary DSP.
- Finalise oscillator nonlinearity, accent accumulation, slide timing, VCA clicks/noise, saturation, or aliasing in this first calibration slice.
- Change the plugin UI or add a sequencer.

## Decisions

### Tune the YAML composition before changing primitives

The existing oscillator, ADSR, curve mapper, gain, and Moog filter already express the first required correction. The patch will scale its filter envelope into a narrower cutoff-control range, retain a positive late cutoff floor, lengthen the filter decay within the ADSR's supported runtime range, and route an explicit non-zero control signal to `filter.resonance`.

This keeps the work in the acceptance patch and makes each signal path visible. A new `tb303` primitive would violate the primitive decision framework and make later improvements less reusable.

### Use conservative proof-of-concept guards before reference-relative tolerances

The first render-level test will measure Hann-windowed spectral centroid during a fixed held note. It will require useful mid-decay high-frequency energy and continuing downward spectral movement. These thresholds are deliberately broad: they reject the current regression without pretending that unmeasured ABL3 values are specifications.

Once controlled reference fixtures exist, measured repeatability and hardware evidence can supplement these broad guards with reference-relative scorecards.

### Make the stimulus and analysis report the reproducibility boundary

A versioned YAML sound fixture will name the maintained patch, exact render settings, event timeline, and analysis settings. The workbench command will render that fixture to an audition WAV and a CSV trajectory containing frame position, time, RMS, peak, and Hann-windowed spectral centroid. A second command path will analyze a pre-aligned PCM WAV with those same analysis settings, allowing software and hardware references to share the report format.

The 303 fixture will contain a 16-step loop at 120 BPM with unaccented and accented notes, rests, ties, and overlapping note transitions. This gives listening and measurement a shared input while exercising the behaviours most likely to expose an acid voice regression. CSV is intentionally simple so later tools can plot or compare the report without introducing another dependency.

### Keep filter and VCA diagnosis separate

This slice changes filter-envelope mapping and resonance while leaving the amplitude envelope substantially unchanged. That prevents a VCA change from concealing whether cutoff falls too quickly or too far. Gate/release calibration will be a later criterion after filter motion is measurable.

### Treat external reference audio as optional evidence

The repository may contain original event manifests, parameter manifests, analysis code, hashes, and derived numerical results. It will not contain streamed demonstrations, proprietary installers, licence material, or third-party audio without explicit redistribution permission. Free clones are useful informal listening references for the proof of concept. Controlled hardware recordings with documented settings become the strongest later calibration evidence, while Devil Fish and stock configurations remain separately labelled. The same approach applies to later instrument families such as hardware and Korg software MS-20 references; each source and configuration remains separately identified.

### Preserve stable behaviour through render-level tests

The owning tests load the public YAML fixture, validate it, render MIDI events through the normal offline engine path, and assert audible output properties. This protects behaviour across refactors of the internal graph representation. Existing note-to-control and accent tests continue to protect adjacent slide and loudness behaviour.

## Risks / Trade-offs

- **The initial thresholds are not a fidelity score.** → Label them as proof-of-concept guards and revise only after repeatable controlled measurements are available.
- **Resonance can alter level as well as timbre.** → Measure spectral and amplitude behaviour separately and avoid per-render normalisation.
- **The generic Moog ladder is not a documented diode-ladder model.** → Improve composition first; require stronger comparative evidence before changing filter topology.
- **A single hardware recording has unknown knob positions.** → Use it to identify broad trajectory defects, not to infer exact parameter mappings.
- **Long FFT windows can blur fast envelope motion.** → Use a fixed Hann window for deterministic regression tests and a shorter-hop analysis pipeline for detailed calibration.

## Open Questions

- Which stock and Devil Fish settings should the later controlled hardware-capture project include?
- Does the current filter topology need a reusable diode-ladder option after composition and gain staging are corrected?
- Which reference-relative metrics and tolerances become useful after repeated hardware captures quantify natural variation?
