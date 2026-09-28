# TB303-001: Calibrate the TB-303 patch against Din Sync reference notes

Status: open. This is a follow-up to the completed
`improve-tb303-dsp-fidelity` proof of concept. It does not change the current
303 patch or its acceptance thresholds yet.

## Evidence to carry forward

- Din Sync's [TB-303 reference recordings](http://www.dinsync.info/2010/02/tb-303-reference-recordings-for-x0xb0x.html)
  document approximate cutoff, resonance, envelope modulation, decay, and
  accent settings for 25 WAV sets. Each set has 16 low-C notes: four knob
  positions, each played as unaccented saw, accented saw, unaccented square,
  and accented square.
- The local archive is `examples/assets/references/tb303/x0x-reference.zip`
  (SHA-256 `f6aaeb3361f87ed1f215af1e17905562a11f978b8ed350a8e9e2be514917a450`).
  The extracted WAVs are in its Git-ignored `wav/` directory. The source URL,
  format, and handling policy are in the [asset README](../../../../examples/assets/references/tb303/README.md).
- [Analysis notes](../../../../examples/assets/references/tb303/ANALYSIS.md) give the
  measurement method and initial findings; [per-note metrics](../../../../examples/assets/references/tb303/analysis.csv)
  preserve all 400 measurements. These are derived evidence, not a claim of
  exact hardware equivalence.
- The five repeated all-50% unaccented saw recordings span 1.8% in RMS and
  4.1% in early spectral centroid. The source says knob positions are
  approximate; use those variations when judging small mismatches.
- With the other knobs near 50%, raising envelope modulation makes the saw
  brighter early but darker late; raising decay keeps it brighter late.
  Resonance and accent responses vary substantially with the other settings.

## Work to do

- [ ] Create a separate 44.1 kHz single-note Dandrum stimulus for the same
  low-C note, waveform, gate, and accent variants. Document the mapping from
  approximate hardware knob percentages to the patch's public parameters.
  Keep the existing 48 kHz acid-loop fixture and its event timing intact.
- [ ] Compare aligned, fixed-gain renders with representative positions from
  `SET-C1` through `SET-C5`, using the same early/late windows, RMS, and
  band-limited spectral-centroid method as the reference analysis. Preserve
  relative levels; do not normalize each note or file independently.
- [ ] Inspect `SET-A1`, `SET-A5`, and `SET-E2` as boundary cases for cutoff,
  accent, and high-resonance interactions. Report audible differences as well
  as measured trajectories, including cases where the current model cannot
  match the recording through public parameters alone.
- [ ] Make only evidence-backed patch or reusable-primitive changes. Specify
  each changed behavior with a failing render-level test first, then verify
  the 303 accent/slide regressions, the existing acid fixture, and the full
  relevant test suite. Record the before/after comparison and any remaining
  mismatch.

## Completion criteria

A future tuning report identifies the matched stimulus, source WAV positions,
parameter mapping, gain/alignment choice, metric windows, reference variation,
and before/after results. Any new regression threshold is wider than measured
capture variation and proves a useful behavior rather than a sample-exact
match. Slide remains a separate calibration task because these recordings do
not provide different-pitch slide examples. Third-party WAVs stay out of Git
until redistribution permission is established.
