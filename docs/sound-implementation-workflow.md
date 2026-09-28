# Sound implementation workflow

Dandrum sounds are implemented with a repeatable loop: define one stimulus,
render it, listen, measure how it changes over time, adjust the patch or a
general-purpose primitive, and turn the useful observation into a regression
test. The checked-in TB-303 fixture is the proof of concept; the workflow is
not specific to that instrument.

## 1. Define the stimulus

Create a YAML fixture under `examples/sound-design/`. A fixture pins:

- the Dandrum patch;
- sample rate, block size, and render duration;
- the event timeline, including velocity, gate, ties, rests, and overlaps; and
- the spectral frame size, hop size, frequency band, and silence threshold.

Keep one event pattern authoritative. The TB-303 fixture stores one 16-step bar
and repeats it four times, so the audition loop and spectral report always use
the same musical input.

## 2. Render the current implementation

From the repository root:

```bash
$HOME/.cargo/bin/cargo run \
  --manifest-path src/rust-engine/Cargo.toml \
  --bin dandrum-sound-workbench -- \
  render examples/sound-design/tb303-acid-poc.yaml \
  --output-wav /tmp/tb303-dandrum.wav \
  --output-metrics /tmp/tb303-dandrum.csv
```

The WAV is for listening. The CSV contains one row per overlapping analysis
frame:

- `start_frame` and `time_seconds` locate the frame;
- `rms` describes short-term energy;
- `peak` exposes transients and clipping risk; and
- `spectral_centroid_hz` describes brightness within the fixture's declared
  band.

The analyzer applies a Hann window before the FFT. It omits centroid for frames
below the declared RMS threshold, because a frequency computed from silence or
the noise floor is misleading.

## 3. Analyze an external reference

Align and trim a reference to the same time origin as the fixture, then create a
mono or stereo 16-bit PCM WAV at the fixture's sample rate. Preserve the
original high-resolution recording separately; the converted file is only an
analysis copy for the current proof-of-concept loader.

```bash
$HOME/.cargo/bin/cargo run \
  --manifest-path src/rust-engine/Cargo.toml \
  --bin dandrum-sound-workbench -- \
  analyze examples/sound-design/tb303-acid-poc.yaml \
  /private/references/tb303-aligned.wav \
  --output-metrics /tmp/tb303-reference.csv
```

Plot or inspect the candidate and reference CSV files on the same time axis.
Start with trajectory questions rather than a single aggregate score:

- Does the initial spectrum open too brightly?
- Does centroid fall too quickly, too far, or stop moving?
- Does RMS close before the useful filter motion can be heard?
- Does an accent change level, brightness, and decay in the expected direction?
- Do overlapping notes produce the intended pitch and envelope transition?

Do not normalize every file independently; that erases meaningful level and
accent differences. Establish one fixed gain alignment for a reference source
and retain it for that fixture family.

## 4. Make one falsifiable change

Turn the observed mismatch into the narrowest behavioral test before changing
DSP. Prefer trajectory milestones or relationships that explain the audible
problem—for example, “the 500 ms centroid remains above the collapse floor and
continues falling by 750 ms”—over a brittle sample-for-sample waveform match.

Change YAML composition first when existing reusable primitives express the
behavior. Add or change a built-in DSP primitive only when the missing behavior
is reusable, realtime-stateful, performance-sensitive, and awkward to compose.
Re-render the fixture after each change and keep level and spectral diagnoses
separate so one does not conceal the other.

## 5. Promote stable observations to regression guards

Early proof-of-concept thresholds should reject obvious failures without
claiming hardware accuracy. Promote tighter reference-relative tolerances only
after repeated captures reveal reference variation and measurement uncertainty.
Keep the fixture, its owning tests, and the corresponding OpenSpec scenario in
sync.

## Later hardware and software reference projects

For a controlled studio capture, retain the raw dry masters privately and
record at least:

- instrument identity, serial or revision where appropriate, and warm-up time;
- stock versus modified state, with stock TB-303 and Devil Fish configurations
  treated as different reference sources;
- every knob, switch, trim, tuning, and software-version setting;
- MIDI/event data, tempo, sample rate, bit depth, interface, input impedance,
  gain staging, and any clocking or resampling;
- repeated takes for estimating analog variation; and
- hashes for the raw and aligned analysis files.

The same fixture/report approach can cover an MS-20 hardware session and Korg's
software instrument. Keep hardware and software renders as separately labelled
evidence, feed both the same event intent, and analyze aligned copies with the
same settings. Public demos and free clones are useful listening references for
prototyping, but they are not numerical ground truth unless their settings,
processing, format, and provenance are controlled.

Do not commit third-party audio or proprietary software content without clear
redistribution permission. It is sufficient to commit Dandrum-owned fixtures,
analysis code, non-infringing derived metrics, and test thresholds.

The Din Sync 303 set now has a local provenance record, per-note measurements,
and a [follow-up calibration task](issues/open/sound-design/tb303-001-dinsync-reference-calibration.md).
