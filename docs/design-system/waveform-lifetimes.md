# Prepared waveform lifetimes

Task 5.2 is verified for the current Linux build. Analysis jobs belong to an
editor session and retain their own immutable source storage. Closing an editor
cancels its pending work; reloading retires old-generation results. Neither
operation requires audio to wait for waveform analysis.

## Tests

| Contract | Evidence |
| --- | --- |
| Closing a native editor cancels its active job without cancelling another session | `native-waveform-lifetime` holds the real reducer, closes the original editor, checks distinct owners and exact cancelled/running states, then checks surviving and reopened displays |
| A source remains readable after its old engine is destroyed | The same test rewrites the WAV at the same path, reloads through the real processor, observes engine destruction before releasing analysis, and reads the original signed samples afterward |
| Late work cannot replace current data | The previous job remains stale/cancelled with no result after completion; the replacement has a new generation and its own distinct samples. Shared worker and Web transport tests also cover delayed results |
| Audio continues while analysis is held | Four callbacks run on a separate thread and return before the reducer barrier is released. Every left/right sample is checked against `+0.25/0` or `-0.5/0`, including prefilled output buffers |
| Analysis reduction and source/engine cleanup stay off these callbacks | GNU link wrappers observe the real FFI entry points; their audio counters stay zero. The held source is reduced and destroyed on the worker thread |
| Queued cancellation, bounded admission, failure and supported retry | `cxx-plugin-waveform-service`, `cxx-plugin-editor-bridge` and `prepared-waveform-web-transport` retain their focused service/adapter contracts |

The hand-encoded mono PCM16 source runs at 48 kHz. Its four original values are
`[-0.75, 0.5, 0.25, -0.125]`; replacement values are
`[0.125, -0.5, 0, 0.75]`. Each bucket spans one source frame, and both extrema
must equal the input literal bit for bit. A separate `control_to_audio` child
produces known signed audio while the worker is held; the waveform test does
not require sample playback or infer audio from analysis data.

The native test uses the shipped sampler factory, processor and editor in a
JUCE test process. Wrappers call the actual Rust reduction/destruction functions;
they add observation and a deterministic worker barrier, without a fake editor,
fake sample owner or production test hook. Observation timeouts are failure
guards, not fixed delays used to establish ownership safety.

## Calibration and coverage

A byte-preserved native editor copy passes both scenarios. Removing only its
destructor's session cancellation causes the same test to fail
`native editor teardown failed to cancel only its active analysis`. Restoring
the original source passes again. The normal source and warm build artifacts
remain unchanged by this calibration.

Focused GCC 11 gcov runs execute all **149/149** executable records in
`InstrumentUiWaveformService.cpp`. An instrumented native editor copy observes
four destructor cancellations, one generation cancellation and three result
deliveries. Its whole-module coverage is partial; this is not complete branch,
JUCE, processor or Rust coverage. No runtime code or extracted production
module changes in this slice.

Full native and Web builds pass, including sampler/TB-303 Standalone and VST3
targets. CTest passes **21/21 native** and **44/44 Web**, without skips. Strict
OpenSpec validation and the existing spec map pass. Raw command logs, hashes,
coverage and calibration are retained in `/tmp/dandrum-native-waveform-evidence`.

## Remaining boundaries

The deterministic native lane currently requires Linux GNU link wrapping and
a display; this run uses Xvfb `:98`. It is not VST3 loading inside a DAW,
Windows/macOS verification, callback deadline measurement or a complete
allocator/locking audit. Task 8 retains the broader callback and stress gates.

The existing engine reload uses a fixed delay. These tests reload between audio
calls and prove retained analysis ownership; they do not establish structural
rebuild quiescence. Section 9's safe handoff and automatic muted rebuild remain
pending. Task 5.3 still owns renderer coordinates and runtime comparison at the
full and compact sizes.
