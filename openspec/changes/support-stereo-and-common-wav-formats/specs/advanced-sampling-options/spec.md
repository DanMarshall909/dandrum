## ADDED Requirements

### Requirement: Common WAV formats are decoded with strict validation and planar channel preservation

The engine SHALL decode mono and stereo WAV sources in PCM16 (`format = 1`, 16-bit), PCM24 (`format = 1`, 24-bit), and IEEE float32 (`format = 3`, 32-bit) encodings, as well as `WAVE_FORMAT_EXTENSIBLE` (`format = 0xFFFE`) wrapping those exact PCM16, PCM24, or float32 subformats. Decoding SHALL validate RIFF/WAVE chunk structure, `block_align`, positive sample rate, channel count (`1..=2`), and complete frames, SHALL preserve finite float32 headroom, and SHALL reject non-finite (`NaN`, `+inf`, `-inf`) float32 samples, surround (>2 channels), unsupported bit depths/tags, and truncated data. Decoded samples SHALL store planar `f32` channels (`frames` for channel 0 and optional `right_frames` for channel 1) without mixing stereo sources to mono, and SHALL account decoded capacity by `frame_count * channel_count * 4` bytes.

#### Scenario: PCM16, PCM24, and float32 WAVs preserve extrema, interleaving, and headroom

- **WHEN** mono and stereo WAV sources in PCM16, PCM24, IEEE float32, and valid `WAVE_FORMAT_EXTENSIBLE` encodings contain positive/negative extrema, distinct interleaved L/R channels, and finite float32 headroom
- **THEN** decoding SHALL preserve source channel count, sample rate, frame count, exact planar channel values, and channel-aware decoded byte capacity

#### Scenario: Malformed and non-finite WAVs are rejected safely

- **WHEN** a WAV source contains non-finite float32 samples (`NaN`, `+inf`, `-inf`), zero or more than two channels, invalid `block_align`, unsupported bit depths or extensible subformats, truncated chunks, or incomplete frames
- **THEN** decoding and preparation SHALL fail with an explicit unsupported-format or invalid-sample diagnostic without panicking or rendering corrupted audio

### Requirement: Mapped sample playback preserves stereo and applies linear stereo balance

`sample_map_player` with 2-channel audio output SHALL render mono sources symmetrically to both output channels and SHALL preserve independent left and right channels for stereo sources without mono downmixing. Region/zone trim, `start_offset`, `fade_in_ms`, `fade_out_ms`, `reverse`, `root_note`, `pitch_semitones`, live/group `pitch_ratio`, sample-rate conversion, `region.gain_db`, `zone.gain_db`, and live/group `level` SHALL process both channels identically. Pan (`region.pan + zone.pan + live_pan + group_pan`, clamped to `[-1.0, 1.0]`) SHALL apply linear stereo balance: `left * (1.0 - pan.max(0.0))` and `right * (1.0 + pan.min(0.0))`. A 1-channel `sample_map_player` output SHALL downmix stereo sources as `(left + right) * 0.5`.

#### Scenario: Stereo topologies and common WAV formats render signed expected L/R output

- **WHEN** `sample_map_player` triggers mono, left-only, right-only, unequal-channel, and antiphase stereo sources across PCM16, PCM24, float32, and cross-rate sample-rate conversion
- **THEN** the two-channel output SHALL match the independently expected signed left and right samples with zero mono cancellation on antiphase material

#### Scenario: Stereo trim, fades, reverse, tuning, gain, and linear balance pan behave consistently

- **WHEN** `sample_map_player` renders stereo regions with trim/region overrides, `start_offset`, fades, reverse, pitch/tuning, gain/level, and pan across `[-1.0, 1.0]`
- **THEN** both channels SHALL share identical timing, fades, pitch, and gain scaling, two-channel pan SHALL apply linear stereo balance without channel cross-feed, and one-channel output SHALL downmix `(left + right) * 0.5`
