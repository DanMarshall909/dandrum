## ADDED Requirements

### Requirement: Shared audio file loading module
The engine SHALL provide a `audio_loading` module that encapsulates PCM WAV file loading and asset preparation logic, reusable by the sampler, convolution, and any future audio-file-consuming modules.

#### Scenario: audio_loading provides load_pcm_wav
- **WHEN** calling `audio_loading::load_pcm_wav(path, target_sample_rate)` with a valid PCM WAV file path
- **THEN** the function SHALL return the loaded sample data (sample rate, frames) or an error diagnostic

#### Scenario: audio_loading provides asset preparation
- **WHEN** calling `audio_loading::prepare_audio_assets(patch, base_dir, module_type_filter)` 
- **THEN** the function SHALL load all assets referenced by modules matching the filter and return a map of module ID to loaded audio data

### Requirement: Sampler delegates to shared loading
The existing sampler module SHALL delegate to `audio_loading` for WAV loading and asset preparation, retaining its public API through re-exports for backward compatibility.

#### Scenario: Sampler uses shared loading internally
- **WHEN** the sampler module calls `prepare_sampler_assets()`
- **THEN** the implementation SHALL delegate to `audio_loading` internally

#### Scenario: Existing sampler API remains unchanged
- **WHEN** existing code calls `prepare_sampler_assets()`, `load_pcm_wav()`, or `LoadedSample`
- **THEN** the API and types remain available from the same module path (re-exported)

### Requirement: Convolution uses shared loading
The convolution module SHALL use `audio_loading` to load IR WAV files, avoiding code duplication with the sampler.

#### Scenario: Convolution loads IR through audio_loading
- **WHEN** a convolution module loads its IR asset
- **THEN** it SHALL use `audio_loading::load_pcm_wav` internally

### Requirement: Unsupported formats are clearly reported
#### Scenario: Unsupported audio file is reported
- **WHEN** a module references a file that is not a supported PCM WAV format
- **THEN** the error SHALL identify the file path and the reason the format is unsupported
