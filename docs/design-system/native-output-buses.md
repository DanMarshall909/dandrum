# Native prepared output buses

This increment adapts the OutputBusses presentation for the native editor.
It advances tasks 7.3 and 7.5 of `add-renderer-independent-plugin-ui`; those tasks
and the overall sampler/303 UI goal remain incomplete.

## Presentation and ownership

`NativeOutputBuses` replaces the early stereo-only meter panel. It copies each
prepared bus identity, name, main flag, channel labels and explicit measurement
binding. It retains no engine, source-buffer or JUCE bus pointer. The existing
editor meter session remains the sole native consumer; generation replacement
rebuilds the copied rows and retires old readings.

The editor admits the meter subscription using the same owned document
generation as its rows. If activation overtakes that copy, rejected admission
leaves the next timer to retry. Initial construction follows this same path;
it cannot record a newer subscription generation while retaining older rows.
An unavailable snapshot clears the rows until a current copy is available.

The component uses generated warm surface, text, focus and meter colours, and
embedded Barlow Semi Condensed / JetBrains Mono faces through `NativeUiFonts`.
That helper also serves the existing native knob. Full rows place measurements
beside bus details; compact rows wrap them below feeds. Rows scroll vertically,
and keyboard focus brings the inspected row into view. Clip targets are 28×24,
with 120×4 peak/RMS tracks and at least 84 pixels for channel summaries.

Only an explicit `master` binding with two channels consumes the current meter
stream. Mono, surround, disabled and unbound stereo descriptions retain their
actual channel facts and report unavailable measurements. No new DSP bus support
is implied: the shipped examples still negotiate a stereo main output. Feed
metadata is absent, so the display says “Feed details unavailable.” Routing,
mute and gain controls have no invented bindings or local configuration state.

Waiting, valid silence, live readings and incomplete history remain distinct.
Measured channel names identify bus/tap, channel, generation and peak/RMS values.
Clip actions use the shared channel/generation/ticket acknowledgement; rejected
or obsolete tickets do not clear the local latch.

## Verification boundaries

- `native-meter-characterization` renders literal signed `+1/0` PCM through a
  real processor subscription, then proves accepted and rejected clip tickets.
- `native-output-buses` checks owned heterogeneous metadata, supplied fonts,
  explicit tap availability, independent literal peak/RMS values and pixels,
  both editor sizes, padded clip targets, focus/scrolling, history gaps, measured
  silence, generation retirement and unavailable bindings. Its varied layouts
  are component inputs; `cxx-output-bus-metadata` separately calibrates actual
  JUCE layout capture. They do not establish multichannel DSP rendering.
- `native-output-buses-runtime` opens the original packaged sampler factory and
  its original native editor. Its first bundled kick sample is `-0.5` on each
  channel. It computes peak/RMS from all 64 callback samples, observes delivery
  through the existing editor timer, captures 1200×800 and 820×560, and verifies
  that a real reload to the 303 clears the old generation's readings.
- `native-output-reload-interleaving` uses a Linux test-only linker wrapper
  around the real prepared-document copy. Healthy startup and real activation
  after either meter snapshot must deliver current rows and measured audio,
  with signed `-0.5/-0.5` PCM. A separately supplied absent optional snapshot
  must report unavailable bindings and recover. The original native session
  provides all delivery; there is no second subscriber or production test hook.
- Focused gcov executes all 196 native component lines and all four executable
  font-helper records. Constructor/destructor compiler variants are combined by
  source line; raw function/branch data is retained. This is execution evidence,
  not exhaustive branch or whole-editor coverage.
- Focused editor gcov executes all 13 reconciliation-function lines, including
  absent-snapshot recovery and activation rejecting subscription admission.
- Seven copied-source faults fail named component/host-adapter assertions after
  a healthy baseline. The original factory/timer runtime and full native smoke
  are excluded from those fault runs. This is focused fault calibration, not an
  exhaustive mutation score.

The initial complete builds passed 39 native and 71 Web CTests without skips. One
intermediate native knob smoke run lost typed-entry focus; its unchanged rerun
and the final full suite pass. That failed run remains in the evidence rather
than being counted as a new bus behaviour RED. The new runtime fixture also
initially triggered audio before capture was subscribed; it now observes the
original editor's actual subscription instead of guessing a startup delay.

Independent review found that construction crossing a reload could permanently
retain old rows. Its real-activation probe and the normal-driver regression
both failed on that identity mismatch before the coherent-admission repair.
The repair and current full gates are recorded separately in
`/tmp/dandrum-native-output-buses-repair-evidence`; the initial review/evidence
remain preserved. A missing-snapshot fixture initially intercepted the knob
descriptor instead of the meter copy; that setup failure is retained separately.
Both repaired complete builds pass, with 40/40 native and 71/71 Web CTests
passing without skips. Strict OpenSpec validation and the unchanged baseline
spec-coverage gate also pass.

Raw commands, source hashes, RED/GREEN runs, coverage, faults and inspected
captures are retained in `/tmp/dandrum-native-output-buses-evidence`.
Runtime inspection covers Linux/X11 at 1×. DAW VST3 loading, other display scales,
final native composition (including the existing title/control-count summaries
after reload), native KeyMap/LayerStack, observed pad/alternate/cursor
feedback and supported structural-routing controls retain their own pending
verification and implementation gates. No Rust, FFI, audio callback, automation
identity, analysis worker or browser transport change is included.
