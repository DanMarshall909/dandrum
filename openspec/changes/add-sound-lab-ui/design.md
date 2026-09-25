## Context

The existing `dandrum-sound-workbench` owns the deterministic fixture render, WAV encoding, and overlapping Hann-window analysis. The JUCE editor is an embedded web view whose JavaScript calls named native functions and receives native events. The audio callback has a strict realtime contract, so a two-second offline render and thousands of FFT frames cannot run there or synchronously block the editor message thread.

The first UI slice is a proof-of-concept tool for the maintained TB-303 fixture. It should establish the same reusable presentation boundary that later stock/Devil Fish TB-303 and MS-20 fixtures can use.

## Goals / Non-Goals

**Goals:**

- Invoke the real Rust sound-workbench render from the existing editor.
- Keep render, FFT, WAV construction, and file access off the audio and message threads.
- Plot time-aligned RMS, peak, and spectral centroid and audition the exact analyzed render.
- Make idle/running/success/failure states explicit and retryable.
- Keep the native boundary small, owned, and independently testable.

**Non-Goals:**

- Realtime spectrum analysis of the plugin output.
- Editing a patch graph or fixture from the plugin.
- Importing or aligning external reference recordings in this first UI slice.
- Applying the editor's current knob values to the repository-owned fixture.
- Replacing the CLI or changing any DSP behavior.

## Decisions

### Return an owned workbench artifact through an opaque C handle

Rust will expose an opaque sound-fixture-render result whose lifetime is explicit. Read-only getters provide the sample rate, duration, metric frames, error text, and PCM WAV bytes. The artifact is created once, so C++ does not rerun the expensive render when it first asks for sizes and then copies data.

This is preferred over returning a large JSON string because the ABI stays typed and JavaScript-specific serialization remains in the editor boundary. It is preferred over reimplementing FFT or WAV generation in C++ because CLI and UI outputs must remain identical.

### Put asynchronous ownership in a UI-independent C++ controller

`SoundLabController` will own a `std::jthread`, state transitions, and the immutable completed artifact copied from Rust. The editor starts it and polls only a cheap atomic generation counter; completed metrics/audio are read through an immutable shared result.

This is preferred over putting a thread directly in `PluginEditor` because the controller can be exercised without a browser or display server. It also makes editor destruction safe: the joining thread is owned by the controller and cannot outlive its state.

### Serve the rendered WAV as a dynamic web resource

The editor's existing JUCE resource provider will serve `/sound-lab.wav` from the completed in-memory bytes. JavaScript sets the audio element source only after the ready event. The plotted metrics and WAV therefore come from the same Rust render artifact.

This is preferred over base64 audio in a native-event payload because a four-bar render is several megabytes and does not belong in the JSON/`juce::var` state message.

### Plot analysis in the embedded page with no frontend dependency

The existing page is a single embedded HTML/CSS/JavaScript asset with no package-manager build. A canvas plot will draw normalized RMS and peak against a logarithmic 30 Hz–20 kHz centroid axis. The event payload carries raw measured values so later visual changes do not alter analysis data.

### Keep the first panel fixture-bound and read-only

The panel renders `examples/sound-design/tb303-acid-poc.yaml`. It labels that maintained fixture and does not imply that plugin knob edits have been captured. Later work can add saved fixture variants and reference overlays without changing the result-handle or controller boundary.

## Risks / Trade-offs

- **The proof panel is not a live analyzer.** → Label the maintained fixture explicitly and keep realtime analysis out of scope.
- **Closing the editor during a render may wait for completion.** → The current fixture completes in roughly two seconds; own it with `std::jthread` so shutdown is safe. Add cooperative cancellation later only if fixture duration grows materially.
- **Thousands of metric points can make native events large.** → Emit a completed report only when the generation changes, not on every timer tick; canvas drawing remains linear in the roughly four-thousand-frame fixture.
- **A source-tree fixture path is a development-only packaging assumption.** → Resolve it through the same upward-search convention as existing example patches and report a visible error when absent. Bundling fixtures for installed releases is deferred.
- **The plugin's older v1 spec describes native controls while this branch now uses a web editor.** → This change extends the implemented web-editor surface only; reconciling that broader historical spec drift is outside this focused slice.
