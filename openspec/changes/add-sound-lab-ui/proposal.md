## Why

The sound workbench already produces a deterministic acid-loop audition and frame-by-frame level/spectral metrics, but using it still requires leaving the instrument UI and manually inspecting files. A compact Sound Lab panel will make that proof-of-concept workflow directly usable while tuning a sound, without putting offline FFT or file work on the audio callback.

## What Changes

- Add a Sound Lab panel to the existing TB-303 web editor with an explicit render/analyse action.
- Render the repository-owned TB-303 proof-of-concept fixture through the same Rust workbench path used by the CLI, on a background thread.
- Plot RMS, peak, and spectral-centroid trajectories and provide browser playback of the exact rendered audition loop.
- Report idle, rendering, ready, and error states through the JUCE native-function/event bridge.
- Add a narrow C FFI artifact handle for reading workbench metrics and WAV bytes without duplicating analysis in C++ or JavaScript.

## Capabilities

### New Capabilities

- `sound-lab-ui`: Runs a maintained sound fixture from the instrument editor and presents its audition audio and spectral/level trajectories.

### Modified Capabilities

- (none)

## Impact

- Rust engine: workbench-to-C FFI result handle and boundary tests.
- JUCE plugin editor: background render ownership, native bridge methods, dynamic WAV resource, and analysis state events.
- Embedded web UI: functional Sound Lab controls, plot, status, and audition player.
- Build/spec coverage: UI contract assertions and acceptance-criteria mappings; no new third-party dependency.
