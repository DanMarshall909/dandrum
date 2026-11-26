## Context

Before this change, `decode_pcm_wav_with_expected_rate` in `src/rust-engine/src/audio_loading.rs` accepted only `audio_format == 1`, `bits_per_sample == 16`, `channels ∈ {1, 2}`, and mixed stereo files down to mono `(L + R) * 0.5` in `frames: Vec<f32>` while keeping raw `i16` pairs in `source_pcm: Option<Vec<i16>>` solely for waveform reduction and FFI inspection. Meanwhile, `process_sample_map_player_state` in `src/rust-engine/src/graph_processor/arena_processing.rs` read only `zone.sample.frames()`, duplicated that mono sample to both output channels, and applied pan.

Keeping a 3rd buffer (mono mix + L + R) would inflate stereo PCM16 decoded memory from 2× to 3× encoded bytes and violate `SampleCatalog`'s `2 * MAX_ENCODED_SAMPLE_BYTES` reservation and 512 MiB decoded budget.

## Decisions

### 1. Planar `f32` storage and channel-aware memory accounting

- `LoadedAudio` and `LoadedSample` store channel 0 in `frames: Vec<f32>` and optional channel 1 in `right_frames: Option<Vec<f32>>` (`None` when mono, `Some(vec)` of equal length when stereo).
- `LoadedSample::decoded_capacity_bytes()` reports `frames.capacity() * 4 + right_frames.as_ref().map_or(0, |r| r.capacity() * 4)`, i.e., `4 * frame_count` bytes for mono and `8 * frame_count` bytes for stereo (for exact-capacity decoded WAVs).
- Ratio to encoded size:
  - PCM16 mono/stereo: `4 / 2 = 2×` encoded data bytes (fits within `2 * MAX_ENCODED_SAMPLE_BYTES`).
  - PCM24 mono/stereo: `4 / 3 ≈ 1.33×` encoded data bytes.
  - Float32 mono/stereo: `4 / 4 = 1×` encoded data bytes.
- `LoadedSample::source_sample(channel, frame)` reads directly from `frames` (channel 0) or `right_frames` (channel 1), preserving full 24-bit and float32 precision for waveform inspection (`reduce_waveform`, `dandrum_kernel_prepared_source_copy_channel`).
- `LoadedSample::with_source_channels` remains available (with `right_frames: None`) so defensive tests constructing an incomplete multi-channel `LoadedSample` continue to exercise failure paths.
- `LoadedAudio::mono_mix()` and `LoadedSample::mono_mix()` return `Cow::Borrowed(&self.frames)` for mono and `Cow::Owned((l + r) * 0.5)` on demand for off-audio mono consumers (`sound_matching`, `sound_workbench`, `Convolution`).

### 2. WAV format support and validation

- Container: RIFF/WAVE (`bytes.len() >= 44`, `RIFF` + `WAVE`), iterating chunks with overflow-checked `offset + chunk_size` and even-byte padding (`chunk_size % 2`), borrowing `data` directly from `&[u8]`.
- `fmt ` chunk (`chunk_size >= 16`):
  - `channels ∈ 1..=2`, `sample_rate > 0`, `sample_rate == expected` when `expected_sample_rate_hz` is provided.
  - `audio_format == 1` (`WAVE_FORMAT_PCM`): `bits_per_sample ∈ {16, 24}`.
  - `audio_format == 3` (`WAVE_FORMAT_IEEE_FLOAT`): `bits_per_sample == 32`.
  - `audio_format == 0xFFFE` (`WAVE_FORMAT_EXTENSIBLE`): requires `chunk_size >= 40`, `cb_size >= 22`, `18 + cb_size <= chunk_size`, `wValidBitsPerSample == 0 || wValidBitsPerSample == bits_per_sample`, and `SubFormat` matching `KSDATAFORMAT_SUBTYPE_PCM` (`00000001-0000-0010-8000-00aa00389b71`) for 16/24-bit or `KSDATAFORMAT_SUBTYPE_IEEE_FLOAT` (`00000003-0000-0010-8000-00aa00389b71`) for 32-bit.
  - `block_align == channels * (bits_per_sample / 8)` and `data.len() % block_align == 0`.
- Sample decoding:
  - **PCM16**: `f32::from(i16::from_le_bytes(..)) / 32768.0` (`-32768 → -1.0`, `32767 → 32767/32768`).
  - **PCM24**: sign-extended little-endian 24-bit `i32::from_le_bytes([0, b0, b1, b2]) >> 8`, normalized as `(raw as f32) / 8_388_608.0` (`-8388608 → -1.0`, `8388607 → 8388607/8388608`).
  - **Float32**: `f32::from_le_bytes(..)`; finite headroom outside `[-1.0, 1.0]` is preserved unmodified, while any `NaN`, `+∞`, or `-∞` sample fails decoding explicitly with `"unsupported format; non-finite float32 sample"`.

### 3. Stereo playback and linear stereo balance pan semantics

In `process_sample_map_player_state`:
- Channel 0 and channel 1 (when present) are interpolated at the identical `*position` (`sample_interpolated`), scaled by identical region boundary fades (`fade_in_frames` / `fade_out_frames`), `zone.gain`, and live/group `level`. `*position` advances identically for both channels across `reverse`, `root_note`, `pitch_semitones`, live/group `pitch_ratio`, `start_offset`, and cross-rate SRC (`playback_rate_scale`).
- Mono sources duplicate channel 0 to both L and R before panning.
- **Pan/balance law (`context.output_count() == 2`)**:
  - `pan = (zone.pan + live_pan).clamp(-1.0, 1.0)` (where `zone.pan = (region.pan + zone.pan).clamp(-1.0, 1.0)` and `live_pan = global_pan + group_pan`).
  - `left_out = left * (1.0 - pan.max(0.0))`
  - `right_out = right * (1.0 + pan.min(0.0))`
  - Semantics: **linear stereo balance**. Center (`pan = 0.0`) passes `[left, right]` at unity without cross-feed or cancellation (antiphase `[x, -x]` renders `[x, -x]`); negative pan attenuates the right channel linearly to `0.0` at `-1.0` while keeping left at unity; positive pan attenuates the left channel linearly to `0.0` at `+1.0` while keeping right at unity.
- **Mono output (`context.output_count() == 1`)**:
  - Mono sources emit `left` bit-exact; stereo sources emit `(left + right) * 0.5`.

### 4. Save/restore and failed-import preservation

- Portable state v3 embeds exact encoded WAV bytes with SHA-256 verification. Restoring into a fresh `ParameterController` or plugin instance decodes the embedded PCM16/PCM24/float32 bytes through `SampleSnapshot::decode` without requiring original files on disk.
- `SampleCatalog::begin` / `prepare` / `accept` rejects malformed, non-finite, or surround WAVs atomically before any patch/resource publication, preserving the active playable instrument, parameter values, history, and catalog usage.
