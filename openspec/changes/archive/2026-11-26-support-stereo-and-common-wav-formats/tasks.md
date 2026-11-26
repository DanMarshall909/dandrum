## 1. Specification & Test Writers

- [x] 1.1 Create OpenSpec change `support-stereo-and-common-wav-formats` and validate with `openspec validate --strict`.
- [x] 1.2 Add `write_wav_channels_i24` and `write_wav_channels_f32` test writers and unit tests in `src/rust-engine/src/wav.rs`.

## 2. Planar Audio Storage & Multi-Format WAV Decoder

- [x] 2.1 Write failing unit tests in `audio_loading.rs` and `sample.rs` for PCM16, PCM24, float32, `WAVE_FORMAT_EXTENSIBLE`, extrema/headroom, non-finite/surround/malformed rejection, and channel-aware decoded capacity.
- [x] 2.2 Replace `source_pcm: Option<Vec<i16>>` with planar `right_frames: Option<Vec<f32>>` in `LoadedAudio`, `LoadedSample`, and `SampleResourceHandle`, implement `new_stereo`/`channel_frames`/`mono_mix` and multi-format WAV decoding, and update non-mapped mono consumers (`sound_matching`, `sound_workbench`, `Convolution`, `preparation`).

## 3. Stereo Mapped Playback & Linear Balance Pan

- [x] 3.1 Write failing rendered tests in `src/rust-engine/src/kernel/document/tests.rs` for mono, left-only, right-only, unequal, antiphase stereo, PCM16/PCM24/float32, cross-rate SRC, trim/fades/reverse/tuning/gain consistency, and linear stereo balance pan (plus 1-channel downmix).
- [x] 3.2 Update `process_sample_map_player_state` in `src/rust-engine/src/graph_processor/arena_processing.rs` to preserve independent L/R channels with zero audio-thread allocation and linear stereo balance pan.

## 4. Native Host, Catalog & End-to-End Restore Verification

- [x] 4.1 Update existing stereo-downmix assertions in `rust-engine`, `trigger-host`, `clap-adapter`, and `vst3-adapter` to match true stereo preservation.
- [x] 4.2 Add `trigger-host` catalog and `ParameterController` + `RuntimeAudio` tests covering channel-aware decoded limits, failed-import preservation (non-finite/surround), stereo PCM24/float32 SRC playback, and portable save/fresh-restore without source files, plus an end-to-end CLAP plugin stereo PCM24/float32 restore test.

## 5. Sync & Final Verification

- [x] 5.1 Sync delta specs into `openspec/specs/advanced-sampling-options/spec.md` and `openspec/specs/trigger-sample-resources/spec.md` and update `spec-tests.map`.
- [x] 5.2 Run all Rust crates' tests (`rust-engine`, `instrument-session`, `trigger-host`, `clap-adapter`, `vst3-adapter`), `./scripts/check-rust-coverage`, `./scripts/check-spec-coverage`, `openspec validate --strict`, and JS Trigger tests.
