# Prepared waveform renderer parity

Task 5.3 is verified on Linux at 1x display scaling. Native JUCE and the shipped
React Canvas display the same prepared signed extrema and source-frame
overlays. Both place coordinates at the nearest pixel, clipping region endpoints
to the plot. JUCE previously truncated fractional coordinates; the runtime
regression caught its misplaced slice-end column before that was corrected.

## Actual renderer test

`PluginWaveformParityTest.cpp` compiles as `native-waveform-parity` and
`web-waveform-parity`. Each starts with the original sampler factory and reloads
a deterministic patch through the real processor. The native path reads the
actual component snapshot. The Web path observes the original WebKit editor,
shipped React mount, native bridge and existing Canvas PNG. It supplies neither
a substitute host nor generated placeholder peaks.

The fixture is a stereo PCM16 source at **48 kHz**, prepared in a **44.1 kHz**
host. Region `signed.body` spans source frames **768–13056**, a duration of
**0.256 s**. Its 512 buckets each contain 24 source frames. Four successive
quarters have exact extrema `[-0.75, 0.5]`, `[-0.25, 0.75]`, `[-0.5, 0.25]` and
`[-0.125, 0.125]`; the unrelated right channel is constant `0.0625`.

| Outcome | Assertion |
| --- | --- |
| Numeric source identity, channel, bounds and signed extrema | The real asynchronous job retains 48 kHz/channel 0 and all 512 exact source-frame buckets; minima/maxima match literal float bits |
| Signed envelope reaches the final renderer | Each actual bitmap has the expected positive/negative extent in every quarter, with background immediately outside those bounds |
| Prepared region, fade, loop and slice markers align | Ten marker columns are checked near both ends of their height against independently derived source fractions |
| Source rate differs from host rate | The processor reports 44.1 kHz; fade columns use the 48 kHz source, and the mounted React heading displays `48000 Hz · 0.256 s` |
| Both sizes remain visible and current | The original editors resize from 1200×800 to 820×560; Canvas bounds remain inside the viewport and the prepared generation stays unchanged |

The plots have different dimensions because layout composition remains a
separate task. Matching means the same source coordinates and rounding rule in
each viewport, rather than identical outer layouts.

| Editor size | JUCE plot | Canvas plot |
| --- | --- | --- |
| 1200×800 | 944×398 | 602×180 |
| 820×560 | 564×158 | 390×180 |

Expected fractions come from the original WAV/YAML facts, independently of both
production coordinate models. Region bounds are 0/1; fades end at 5/512 and
497/512; loop bounds are 1/8 and 7/8; the two slices span 1/4–11/32 and 5/8–3/4.
The test samples envelope buckets away from these overlay columns.

## Verification and limits

Full native and Web build graphs pass, including both Standalone and VST3
targets. After the Web observation harness was adjusted for navigation
readiness, its owning target was rebuilt through CMake. Final full CTest runs
pass **22/22 native** and **45/45 Web**, without skips. The existing geometry,
Canvas, transport, service and lifetime tests remain in those suites.

Focused GCC 11 gcov runs execute all four changed native paint-coordinate lines
before and after the correction. This is changed-line coverage, not whole
native editor, branch or automated mutation coverage. The original slice-end
failure provides a named regression calibration. No new production module is
extracted: source conversion remains in the existing geometry, and final pixel
placement stays in each renderer.

Raw commands, source hashes, RED/setup logs, coverage and the four inspected
actual plot captures are retained in `/tmp/dandrum-waveform-parity-evidence`.
The Linux runtime uses Xvfb `:98`, software rendering and WebKit at 1x scaling.
Observation retries accommodate navigation and asynchronous React resizing;
the timeout still requires a valid, visible, correctly painted Canvas.

This slice does not verify Windows/macOS, DAW loading, audio deadlines or other
display scales. Full supplied `WaveformPanel` styling/layout, spectral views and
real playback cursor feedback remain later tasks. Rust, FFI, engine preparation,
parameter identities and analysis service production code are unchanged.
Section 9's automatic muted structural rebuild remains pending; the existing
fixed-delay reload is not a safe structural ownership handoff.
