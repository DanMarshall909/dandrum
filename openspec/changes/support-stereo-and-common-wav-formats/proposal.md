## Why

Native Trigger currently decodes only 16-bit PCM WAV (`audio_format = 1`) and averages stereo channels to mono (`(L + R) * 0.5`) before mapped playback while retaining a secondary `i16` buffer for waveform inspection. That loses stereo placement and phase relationships (e.g., antiphase or hard-panned material) and rejects common 24-bit PCM and 32-bit IEEE float WAV files.

This change delivers GitHub `#32` ("Feature: preserve stereo and support common WAV formats in native Trigger"), establishing the planar channel and WAV format contract before subsequent native sampler roadmap slices (`#33`, `#34`, `#35`).

## What Changes

- Replace the legacy mono-downmix + `source_pcm: Option<Vec<i16>>` dual storage in `LoadedAudio` / `LoadedSample` with planar `f32` channel storage (`frames` for channel 0 and optional `right_frames` for channel 1), retaining full PCM16, PCM24, and float32 precision and channel-aware decoded memory accounting (`4 * frames` bytes for mono, `8 * frames` bytes for stereo).
- Expand WAV decoding to accept mono and stereo PCM16 (`format = 1`, 16-bit), PCM24 (`format = 1`, 24-bit), IEEE float32 (`format = 3`, 32-bit), and `WAVE_FORMAT_EXTENSIBLE` (`format = 0xFFFE`) for those exact subformats, with explicit validation of `block_align`, chunk lengths, channel bounds (`1..=2`), and rejection of non-finite (`NaN`/`±inf`) float32 samples.
- Preserve independent mono and stereo channels through `sample_map_player` (`process_sample_map_player_state`) with consistent trim, fades, reverse, pitch/SRC, gain, and documented **linear stereo balance** pan semantics (`left * (1 - max(pan, 0))`, `right * (1 + min(pan, 0))` for 2-channel output; `(left + right) * 0.5` downmix only when the module itself is configured with 1-channel output).
- Keep non-mapped mono analysis/IR consumers (`sound_matching`, `sound_workbench`, `Convolution`) on an on-demand `mono_mix()` projection without allocating a third buffer on `LoadedSample`.
- Prove native Trigger host selection, channel-aware catalog capacity limits, failed-import preservation, portable state v3 save/fresh-restore without original files, and end-to-end CLAP plugin restoration.

## Capabilities

### Modified Capabilities

- `advanced-sampling-options`: WAV decoding for PCM16/PCM24/float32 + `WAVE_FORMAT_EXTENSIBLE`, non-finite/malformed rejection, planar channel-aware decoded storage, stereo preservation in `sample_map_player`, and documented linear stereo balance pan semantics.
- `trigger-sample-resources`: Native selection, channel-aware decoded capacity limits, failed-import instrument preservation across PCM16/PCM24/float32 and invalid WAVs, and portable save/fresh-restore preserving channels and decoded formats without original files.

## Impact

- `src/rust-engine/src/audio_loading.rs`, `src/rust-engine/src/sample.rs`, `src/rust-engine/src/compiled_patch.rs`, `src/rust-engine/src/wav.rs`: Planar `f32` audio storage, lossless `source_sample` / `channel_frames` / `mono_mix`, multi-format WAV decoding and test writers.
- `src/rust-engine/src/graph_processor/arena_processing.rs`: True stereo interpolation and linear stereo balance pan in `process_sample_map_player_state` with zero audio-thread allocation.
- `src/rust-engine/src/sound_matching.rs`, `src/rust-engine/src/sound_workbench.rs`, `src/rust-engine/src/graph_processor/state.rs`, `src/rust-engine/src/preparation.rs`: Preserve mono-mix semantics for non-mapped consumers.
- `src/trigger-host/`, `src/clap-adapter/`, `src/vst3-adapter/`: Existing stereo-downmix fixture expectations updated to true stereo; new host/catalog/CLAP tests for PCM24, float32, channel-aware limits, and fresh-instance restore.
- Out of scope: compressed formats, surround (>2 channels), disk streaming, and a new asset-management system.
