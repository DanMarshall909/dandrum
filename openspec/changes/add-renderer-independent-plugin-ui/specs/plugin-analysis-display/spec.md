## Purpose

Supply renderer-independent waveform and spectral analysis from safely retained prepared audio or bounded live capture without making analysis part of the realtime rendering dependency chain.

## ADDED Requirements

### Requirement: Analysis results are shared asynchronous data

Waveform and spectral requests SHALL execute away from audio and UI threads and return coherent results with instrument generation, source identity, channel, frame coordinates, sample rate and analysis settings. Both renderers SHALL consume the same numeric results. Requests SHALL have bounded admission, cancellable jobs and retrievable terminal status. Sample ownership SHALL remain valid throughout analysis across reload and editor closure.

#### Scenario: Both renderers request the same region

- **WHEN** native and WebView consumers request the same prepared source, region and analysis settings
- **THEN** they SHALL obtain equivalent numeric analysis data independent of their drawing APIs
- **AND** cached work SHALL be reusable without rereading the source file for each repaint

#### Scenario: Reload replaces a source during analysis

- **WHEN** a source is replaced while a worker is analyzing its previous prepared audio
- **THEN** that worker SHALL finish or cancel without accessing freed storage
- **AND** its result SHALL not overwrite the replacement instrument's current display

#### Scenario: Analysis is cancelled or fails

- **WHEN** analysis is cancelled, rejects a request or fails while an instrument is playing
- **THEN** its status SHALL identify the affected request and permit a supported retry
- **AND** the audio callback SHALL continue without waiting for analysis or worker shutdown

### Requirement: Prepared waveforms preserve signed extrema and coordinates

A prepared waveform SHALL describe per-channel signed minimum and maximum samples over explicit source-frame buckets. Region, fade, loop and slice overlays SHALL use the same coordinate system. Live cursor updates SHALL not require regeneration of static waveform data. Cache identity SHALL include source content or immutable revision, region, channel and reduction settings.

#### Scenario: A narrow signed transient remains visible

- **WHEN** one waveform bucket covers samples 0, -0.75, 0.5 and 0
- **THEN** its envelope SHALL contain minimum -0.75 and maximum 0.5
- **AND** display width changes SHALL preserve correct source-frame alignment of overlays

#### Scenario: Source bytes change at the same path

- **WHEN** a reload prepares changed sample content under the same filename and region name
- **THEN** the previous content's waveform and spectrogram SHALL not be reused as the new result

### Requirement: Spectral displays declare their measurement settings

Prepared and subscribed live spectral analysis SHALL define the FFT window, size, hop, sample rate, channel policy, magnitude scaling, frequency-bin mapping and display floor. The shared result SHALL retain time/frequency coordinates and magnitude data; colour mapping and image caching SHALL be renderer concerns. Any downsampling SHALL apply appropriate anti-alias filtering and publish the effective analysis sample rate.

#### Scenario: Known frequency produces the expected spectral band

- **WHEN** a deterministic sine at a documented FFT-bin frequency is analyzed
- **THEN** its dominant band and amplitude scaling SHALL agree with the documented window and normalization within an explicit tolerance
- **AND** native and WebView views SHALL consume the same magnitude values

#### Scenario: Silent input has a finite display floor

- **WHEN** zero-valued input is analyzed
- **THEN** spectral values SHALL map to the documented finite floor without NaN or infinite coordinates

### Requirement: Live analysis can lose history without corrupting time

Live waveform and spectral streams SHALL use bounded preallocated capture and bounded worker output. Frames SHALL carry sequence and audio-sample positions, and a worker SHALL discard stale backlog only through its authorized consumer path. A capture gap SHALL be explicit and SHALL reset any analysis window that would otherwise splice noncontiguous audio together. Rendering SHALL remain independent of worker availability.

#### Scenario: Capture drops part of a spectral window

- **WHEN** saturation creates a discontinuity between captured sample ranges
- **THEN** analysis SHALL identify the gap and restart window accumulation from contiguous samples
- **AND** it SHALL NOT report a spectrum formed by joining samples from opposite sides of the gap

#### Scenario: Visual subscriptions end

- **WHEN** the last live-analysis subscriber disconnects or hides its view
- **THEN** capture and analysis for that subscription SHALL become inactive without joining a worker from the audio callback
- **AND** resumed analysis SHALL begin with current generation and sample coordinates
