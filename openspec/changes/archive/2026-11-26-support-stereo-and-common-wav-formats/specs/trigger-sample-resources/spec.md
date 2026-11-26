## ADDED Requirements

### Requirement: Native Trigger supports stereo and common WAV formats through selection, limits, and state restore

Native Trigger selection, catalog accounting, and portable state SHALL support mono and stereo PCM16, PCM24, and finite float32 WAV files (including `WAVE_FORMAT_EXTENSIBLE` equivalents). Source metadata (`channelCount`, `frameCount`, `sampleRateHz`), decoded planar representation, and channel-aware decoded memory limits (`frame_count * channel_count * 4` bytes) SHALL agree. Invalid, non-finite, or >2-channel WAV imports SHALL fail atomically while preserving the current playable instrument. Saving portable state and restoring into a fresh instance SHALL preserve channels and decoded behavior without the original selected files.

#### Scenario: Stereo PCM24 and float32 selections agree on metadata, channel limits, and signed playback

- **WHEN** native Trigger selects mono and stereo PCM16, PCM24, and float32 WAVs, checks channel-aware decoded capacity limits, and fails an import with a non-finite float32 or surround (>2ch) WAV
- **THEN** valid imports SHALL report truthful `channelCount`, `frameCount`, and `sampleRateHz`, enforce channel-aware decoded byte limits (`4 * frames` vs `8 * frames`), render signed stereo L/R audio through the native mapped path, and failed imports SHALL preserve the live instrument and catalog capacity

#### Scenario: Save and fresh-instance restore preserve stereo channels and WAV formats without source files

- **WHEN** a native Trigger instrument and loaded CLAP plugin with mapped stereo PCM16, PCM24, or float32 samples are saved to portable state, the original instance and files are removed, and a fresh instance loads that state
- **THEN** the fresh instance SHALL restore truthful channel metadata, named physical values, and independently expected signed stereo L/R output without the original files
