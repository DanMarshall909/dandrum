# Prepared spectral views

Native JUCE and the shipped React Canvas consume the shared prepared spectral
job's numeric result. Each renderer owns its image and colour mapping; no
browser image crosses the shared analysis contract. Rust analysis, engine
preparation, host parameter identities and audio processing are unchanged by
this display slice (task 6.2).

## Measurement and presentation

The existing worker supplies periodic-Hann, 1024-point spectra, one-sided peak
dBFS magnitudes and a finite **-120 dBFS** floor. The source sample rate defines
frequency bins and source frames define time. The view uses a logarithmic
frequency axis from the first positive bin to Nyquist; DC is explicitly omitted.
When several bins occupy a pixel row, their maximum preserves narrow bands.
Region, fade, loop and slice overlays reuse prepared waveform coordinates.

The five-colour ramp uses Ink 0, Ink 5, low Ember, Ember and Paper 1, normalised
from the declared floor to 0 dBFS. Wave/Spectral controls switch the prepared
view; failure has a visible unavailable state and an analysis retry. Hiding,
switching, closing or reloading retires obsolete work without waiting for it.
Returning to the view requests current data. Native image caching and browser
Canvas painting remain outside the analysis worker.

The Web adapter publishes at most **16 numeric columns per page**, with exact
decimal 64-bit frame/job identities, declared settings and source revision.
Browser assembly accepts at most **1024 columns**, one admission and one poll at
a time, and publishes only a complete, coherent result for the current selection
and generation. The transport's fake-host tests are calibrated against the
actual bridge's successful pages, metadata and failure status within that scope.

## Actual renderer evidence

`PluginSpectralParityTest.cpp` compiles into `native-spectral-parity` and
`web-spectral-parity`. Both use the original sampler factory, processor and
editor. Web observation reads the mounted production Canvas; it supplies no
substitute host or placeholder spectrum.

The PCM16 fixture has a **48 kHz** source in a **44.1 kHz** host. Region
`signed.body` spans frames **768–13056**, or **0.256 s**. Its left channel has a
0.5-amplitude sine at FFT bin 64, **3000 Hz**; the right channel and the samples
outside the region differ. The real job has 48 columns with a 256-frame hop.
The first 45 complete windows assert **-6.0206 dBFS ± 0.002** at bin 64 and
**-120 dBFS** at a distant positive bin. Zero-padding the final partial windows
produces expected leakage at the right edge.

| Outcome | Executable evidence |
| --- | --- |
| Known band, scaling and floor reach both renderers | Literal 3 kHz row, RGB `238,212,184` band and Ink 0 floor pixels in actual JUCE/Canvas bitmaps |
| Prepared overlays retain source coordinates | Ten independently derived region/fade/loop/slice columns at both sizes |
| Labels describe the source measurement | Actual 47 Hz/24 kHz limits, 0.016/0.272 s endpoints, FFT/hop/floor/channel/DC labels; Web labels visible and unobscured |
| Switching and visibility retire old work | Actual Wave/Spectral round trip and native window/browser document hide/show recover current known pixels |
| Failure permits a real retry | Linux link wrapper fails the real PCM reader; actual worker and both views report failure, then recover the known band after retry |
| Reload clears old sample state | Reload to the real sample-free 303 clears the spectrum, labels and old source metadata |
| Exact large frame offsets and invalid results | Geometry/Canvas unit tests use frame 9007199254740993, literal log rows, finite floor and rejected stale/incoherent data |

| Editor size | JUCE plot | Canvas plot |
| --- | --- | --- |
| 1200×800 | 896×364 | 554×128 |
| 820×560 | 516×124 | 342×128 |

Outer layout composition remains task 7.5. These plots share source coordinates
and measurement values; the different panel dimensions are deliberate test
inputs. Actual plot captures and a compact Web editor capture were inspected.

## Verification and limits

Full native and Web builds pass, including Standalone and VST3 targets. CTest
passes **26/26 native** and **51/51 Web** without skips, including Rust, waveform,
knob, bridge, sampler and 303 regressions. Sampler TypeScript checking, production
asset building, strict OpenSpec validation and the existing baseline spec
coverage gate pass. Delta specs are not synced by this slice.

Focused gcov executes **44/44** executable lines in the new C++ geometry and
**115/115** changed bridge lines. The actual native runtime executes **143/144**
changed editor lines. Its remaining line is the defensive incoherent-result
error in `NativePreparedSpectrum::setResult`; the supported worker supplies
fixed, typed coherent results and this fallback was not injected through the
private component. No whole-editor or exhaustive branch coverage is claimed.

All 14 Node model/transport/waveform tests pass. V8 reports 100% source lines and
functions for the three production modules; branch coverage is 95.83% for the
spectral model, 95.80% for transport and 90.91% for waveform. This does not
measure the whole React component. Eleven individually applied copied-source
faults are rejected by named assertions. A doubled column cap initially survived
a malformed test that also violated the frame count; a coherent 1025-column
fixture now independently rejects that fault. This is focused calibration,
not an exhaustive mutation score.

Runtime RED cases also caught JUCE UTF-8 label decoding, an obscured Web time
label and a misleading Preparing header after failure. Their corrected tests
pass. Web observation permits at most two outstanding evaluations and filters
stale phase/viewport/generation reports; undefined observations are not success.
Earlier observer timeouts and the initial cap survivor remain in the evidence.
Their unproved underlying causes are not claimed resolved by a screenshot.

Raw commands, source hashes, complete outputs, RED cases, gcov/V8 data, fault
copies and inspected captures are retained in
`/tmp/dandrum-spectral-view-evidence`. Runtime proof is Linux/Xvfb/WebKit at 1x
scaling. PCM failure injection is Linux-specific. Windows/macOS, DAW timing,
other display scales, live capture (6.3), full resource/stress budgets (6.4) and
final reference composition remain pending. Section 9's automatic structural
mute/rebuild/resume runtime is also pending; existing fixed-delay reload tests
do not prove the required ownership handoff.
