## 1. Spectral Behaviour Contract

- [x] 1.1 Add a render-level held-note test with a Hann-windowed spectral-centroid helper that specifies the mid-decay brightness and continuing spectral movement requirements.
- [x] 1.2 Run the owning test against the current patch and confirm it fails for the documented spectral collapse rather than fixture or analysis setup.

## 2. Filter Calibration

- [x] 2.1 Reshape the TB-303 patch's cutoff range and lengthen its filter-envelope decay using existing control primitives while leaving the amplitude envelope unchanged.
- [x] 2.2 Add an explicit non-zero resonance control route and a test that proves the loaded patch drives that route during a held note.
- [x] 2.3 Run the owning render tests and retain finite, deterministic output with the spectral-collapse guard passing.

## 3. Accent Regression Coverage

- [x] 3.1 Add a render-level test proving that velocity accent increases mid-decay brightness as well as the already-covered RMS level.
- [x] 3.2 Make the smallest patch adjustment needed for the accent level-and-timbre scenario, then run the accent and legato/slide regression tests.

## 4. Reusable Sound-Implementation Workflow

- [x] 4.1 Add behavior-first tests for reusable Hann-windowed RMS, peak, and band-limited spectral-centroid trajectories, including audible tones and silent frames.
- [x] 4.2 Add a repository-owned, declarative 48 kHz sound fixture for a 16-step acid loop with rests, accents, ties, and overlapping slide transitions.
- [x] 4.3 Add a tested sound-workbench command that renders a fixture to deterministic, finite, non-silent WAV audio and a frame-by-frame metrics CSV, and can analyze an aligned external PCM WAV with the same settings.
- [x] 4.4 Document the listen-measure-adjust-regress workflow, external-reference policy, and the path for later stock/Devil Fish TB-303 and hardware/software MS-20 fixtures.

## 5. Verification And Spec Coverage

- [x] 5.1 Run focused tests, the complete Rust suite, and coverage for the changed test/patch path; inspect the assertions rather than relying on line coverage alone.
- [x] 5.2 Hand-inject representative filter-decay and resonance faults and confirm the owning tests fail for the intended reasons; run targeted mutation analysis only if it adds signal beyond the YAML fault checks.
- [x] 5.3 Validate the OpenSpec change strictly, sync the accepted delta spec, map every new scenario to its proving test in `spec-tests.map`, and run `scripts/check-spec-coverage`.
