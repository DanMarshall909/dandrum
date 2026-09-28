# Din Sync TB-303 recording analysis

Source and archive details are in [README.md](README.md). The derived
[analysis.csv](analysis.csv) has one row for each of the 400 notes in the 25
recorded sets. The raw WAVs are in the local, Git-ignored `wav/` directory.

## Method

- Each WAV is stereo, 16-bit PCM at 44.1 kHz, with 2,166,412 frames (49.125 s).
  The channels are almost identical (correlation above 0.999996), so the
  measurements use their arithmetic mean. No file was normalized.
- The 16 notes occur roughly every three seconds. With a 10 ms RMS window and
  a 0.0005 full-scale threshold, their audible starts are about 1.13-1.14 s,
  4.13-4.14 s, and so on. The analysis uses 1.14 + 3n seconds as each onset.
- RMS and peak cover the first 1.3 s after onset. Early and late RMS and
  spectral centroids use 0.03-0.25 s and 0.85-1.07 s after onset, respectively.
  Centroids use a Hann-window periodogram over 30 Hz to 10 kHz. These
  centroids describe recorded brightness; they are not filter cutoff values.
- The source page gives approximate knob percentages. `analysis.csv` records
  those separately from whether the note itself is accented. Each four-note
  position is unaccented saw, accented saw, unaccented square, accented square.

## Findings

The following measurements use the **unaccented saw** note in each position of
the C sets. The other four knobs are at approximately 50%; the named knob
changes from 25% to 100%.

| Varied knob | Early centroid (Hz) | Late centroid (Hz) | Note RMS |
| --- | ---: | ---: | ---: |
| Cutoff | 127 to 332 | 91 to 267 | 0.0169 to 0.0278 |
| Resonance | 109 to 333 | 89 to 198 | 0.0273 to 0.0234 |
| Envelope modulation | 162 to 230 | 133 to 80 | 0.0224 to 0.0200 |
| Decay | 141 to 203 | 122 to 154 | 0.0207 to 0.0250 |
| Accent knob | 172 to 172 | 124 to 124 | 0.0221 to 0.0221 |

- Cutoff changes both loudness and brightness. With the other knobs at zero
  (`SET-A1`), the unaccented saw's RMS rises about 13.1 dB from 25% to 100%
  cutoff; its early centroid rises from 74 to 111 Hz.
- Resonance becomes highly context dependent. With the other knobs at 100%
  (`SET-E2`), the unaccented saw's early centroid rises from 269 to 1,569 Hz
  over the resonance sweep. Its RMS falls sharply and then rises slightly, so
  matching resonance by loudness alone would be misleading.
- More envelope modulation increases early brightness while reducing late
  brightness in `SET-C3`. More decay in `SET-C4` keeps the unaccented note
  brighter late. The accented note responds differently: across the `SET-C4`
  decay sweep, its late centroid stays around 121-122 Hz.
- The accent knob leaves unaccented saw notes nearly unchanged in `SET-C5`.
  The accented/unaccented saw RMS difference changes from -0.6 dB at 25%
  accent to +2.5 dB at 100%. With the other knobs at zero (`SET-A5`), the
  difference at 100% accent is +9.5 dB. Accent therefore needs comparison
  across more than one base setting.
- The all-50% unaccented saw setting occurs in all five C sets. Across these
  repeated captures, RMS spans 1.8% of its mean and early centroid spans
  4.1%. That is a useful estimate of recording/analog variation before
  setting reference-matching tolerances.
- The largest absolute sample peak across the WAV channels is 0.531 full scale; these files
  do not show digital clipping.

## Use for Dandrum tuning

Compare one waveform and knob position at a time with the same low-C note
timing. Preserve relative levels across files. Render or resample at 44.1 kHz
before spectral comparison: the current `tb303-acid-poc.yaml` fixture is a
48 kHz musical phrase, so it is not a like-for-like stimulus. Prioritize the
cutoff, resonance, envelope-modulation, decay, and accent trajectories above;
these recordings do not isolate slide behavior.

The [follow-up calibration task](../../../../docs/issues/open/sound-design/tb303-001-dinsync-reference-calibration.md)
records the comparison and verification work still needed.
