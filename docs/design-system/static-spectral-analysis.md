# Prepared spectral analysis

`InstrumentUiSpectralService` supplies asynchronous numeric data to either
renderer through the processor's `requestPreparedSpectrum`, status and
cancellation methods. Call these APIs off the audio thread. Retaining the
source briefly uses the existing reload lock; PCM copying, FFT, cache work and
worker resource cleanup run independently of `processBlock`.

## Measurement contract

| Setting | Initial contract |
| --- | --- |
| Input | One explicitly selected prepared source channel, at its source sample rate |
| Window | Periodic Hann: `w[n] = 0.5 * (1 - cos(2*pi*n/1024))` |
| FFT | 1,024 source frames, no audio downsampling |
| Hop | 256 frames, increased by an integer multiple for long regions to keep at most 1,024 columns |
| Frequency grid | 513 linear bins, `frequency[k] = k * sourceRate / 1024`, including DC and Nyquist |
| Magnitude | One-sided peak amplitude: `abs(FFT[k]) * factor / sum(w)`, factor 1 at DC/Nyquist and 2 elsewhere |
| Scale | `max(-120, 20 * log10(amplitude))` dBFS; exact zero maps to -120 |
| Tail | Zero pad beyond the selected region; keep full-window normalization and publish the actual input end frame |

The effective hop is `256 * ceil(regionFrameCount / (256 * 1024))`. Columns
begin at `regionStart + columnIndex * hop` while inside the region. Their input
is a contiguous window of up to 1,024 frames. A sparse column grid can skip
time between windows on long recordings; it does not resample audio, change
the analysis rate or concatenate separated samples. Frame coordinates remain
64-bit integers. Convert relative frame positions to seconds using the source
rate. Host rate and transport timing do not change these static coordinates.

A bin-centred sine with peak amplitude 0.5 gives -6.020599913 dBFS at its
dominant bin (test tolerance 0.002 dB). The periodic Hann's adjacent bin gives
-12.041199826 dBFS (0.004 dB tolerance). Tests independently check source-rate
coordinates, DC/Nyquist scaling, literal silent floors and partial-window
normalization. Determinism is checked with a fresh service, beyond cache reuse.

## Ownership and bounds

The existing opaque retained-source handle owns a Rust `Arc<LoadedSample>`.
The new FFI operation copies contiguous channel frames without borrowing a
reloadable engine. Invalid copies leave the caller's buffer untouched. A job
can finish reading after its engine and source file have gone away.

One worker admits up to four queued jobs, retains sixteen status records and
four cached results. A result contains at most 1,024 columns of 513 floats
(about 2 MiB of magnitudes). The service's status/cache references can therefore
retain at most twenty distinct results (about 40 MiB of magnitudes plus frame/
frequency coordinates, identifiers and container overhead). An in-flight job
can add one result during computation. A caller keeping extra snapshots owns their
retention cost. Input sources remain shared with prepared audio; this service
does not duplicate whole sample files.

Cache keys include immutable content revision, source/region identity, selected
channel, source rate, frame bounds and declared settings. Identical queued work
reuses completed numeric data. Changed content at the same path cannot reuse
the previous source's result. Cancellation and generation retirement happen
without waiting for a running FFT. Late results cannot overwrite terminal or
current-generation status. Failed reads, nonfinite PCM or reader exceptions
produce failed status with an error and permit retry. Worker destruction joins
off audio before its storage is released.

## Verification boundaries

`cxx-plugin-spectral-service` owns the measurement, cache, bounded admission,
history, cancellation, failure and retained-source contracts using real PCM16
preparation and the real channel-copy boundary. `cxx-plugin-spectral-job` uses
the original sampler factory/processor in both build configurations. Linux
link wrappers hold the real reader while callbacks assert literal signed audio,
then observe replacement generation, retained old PCM and worker cleanup.

The service supplies shared numbers. Native/Web drawing, time/frequency labels
and overlays belong to task 6.2; live analysis and its gap/backlog policies
belong to 6.3/6.4. This static job work does not implement structural editing's
automatic mute/rebuild/resume ownership handoff (tasks 9.1-9.7).
