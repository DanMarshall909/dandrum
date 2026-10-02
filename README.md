# Dandrum

Headless-first OSS virtual instrument experiment.

## First Sound

The first milestone is deliberately tiny: prove the JUCE wrapper can open the default audio device while Rust owns the
sample generation.

Native Linux dependencies for JUCE:

```bash
sudo apt install -y libasound2-dev libx11-dev libxext-dev libxinerama-dev libxrandr-dev libxcursor-dev libxrender-dev libfreetype6-dev libfontconfig1-dev libgl1-mesa-dev libcurl4-openssl-dev
```

```bash
$HOME/.local/bin/cmake -S . -B build
$HOME/.local/bin/cmake --build build
./build/dandrum-drum-machine-demo_artefacts/dandrum-drum-machine-demo
```

This uses JUCE as the wrapper/host side. The current binary links a Rust static library from `src/rust-engine/` and
calls it from the JUCE audio callback.

## Engine Development

The headless engine core is implemented in Rust under `src/rust-engine/`. The `core` module is the frontend-independent
engine boundary; JUCE, CLI, GUI, plugin, and realtime driver code should stay outside that module.

A machine-readable patch schema lives at `schema/patch.schema.yaml`. The Rust
loader checks it before graph construction, then validates graph semantics.

Reusable defined modules can be loaded from the versioned `$LIB` standard library or a mutable `$USER_LIB` directory.
See [module library authoring and the drum voice example](docs/module-library.md).

Patch YAML can declare an external preset contract with `instrument` and `preset_surface`. `instrument.id` identifies
the compatible instrument, and `instrument.preset_schema_version` lets future incompatible public-surface changes reject
old preset files. `preset_surface.parameters` gives public names such as `kick.decay_ms` to root control ports; the ports
hold the defaults and ranges and map to module controls. `preset_surface.assets` maps public asset names to static
resource arguments.

External preset YAML files live independently from patches. A preset declares `name`, matching `instrument`, optional
`metadata`, `values` for public parameter targets, and `assets` for public asset targets. Presets cannot declare graph,
routing, render, event, script, scheduling, or feedback fields; those remain patch structure. See
`examples/patches/synthetic-808-kick.yaml` and `examples/presets/tight-808-kick.yaml`.

## Patch authoring and offline rendering

A patch is a graph definition. Its `ports` declare the host interface;
`modules` instantiate primitives or defined modules; `connections` are cables
between module ports. This two-channel patch exposes one named output bus:

```yaml
metadata: { name: Simple Tone }
ports:
  - { name: master, direction: output, signal: audio, channels: 2, maps_from: osc.audio }
modules:
  - { id: osc, type: oscillator, static: { channels: 2, waveform: sine }, defaults: { pitch: 1.0 } }
connections: []
```

`static` supplies construction-time arguments such as channel count, waveform,
or a sample resource. `defaults` sets unconnected control input values; a
cable to that input takes precedence. Root control inputs can map to module
controls and carry public preset values. Define reusable graphs under
`module_definitions` or load a [module package](docs/module-library.md).
The [polyphonic chords example](examples/patches/polyphonic-chords.yaml) shows
an explicit `poly` region. Feedback cycles require `feedback_delay`; ordinary
effect delays do not legalize a cycle. The
[delayed feedback example](examples/patches/delayed-feedback.yaml) uses
`delay_samples` equal to one prepared block; the declared delay must be at
least the host's maximum block size. Its `source` root input expects a host
audio bus, and its `master` output carries the dry signal plus decaying repeats.

From the repository root, render a checked-in kernel patch with host settings
on the command line:

```bash
CARGO_TARGET_DIR="$PWD/build/rust-target" $HOME/.cargo/bin/cargo run \
  --manifest-path src/rust-engine/Cargo.toml --bin dandrum-cli -- \
  render examples/patches/event-routing-drum-machine.yaml \
  --output /tmp/dandrum-drum-machine.wav --duration-frames 4800 \
  --sample-rate 48000 --block-size 128
```

`--duration-frames` is required. The sample rate and block size default to
48,000 Hz and 128 frames. For one `master` output, `--output` names its WAV;
additional named audio outputs get separate WAV files. A pair of mono `left`
and `right` outputs becomes one stereo WAV. The authored patch has no `render`
settings or `audio_output` module.

Rust unit tests are the default home for core behavior:

```bash
CARGO_TARGET_DIR="$PWD/build/rust-target" $HOME/.cargo/bin/cargo test \
  --manifest-path src/rust-engine/Cargo.toml
```

For iterative sound design, render the checked-in TB-303 proof-of-concept
fixture to an audition WAV and a spectral/level trajectory:

```bash
CARGO_TARGET_DIR="$PWD/build/rust-target" $HOME/.cargo/bin/cargo run \
  --manifest-path src/rust-engine/Cargo.toml \
  --bin dandrum-sound-workbench -- \
  render examples/sound-design/tb303-acid-poc.yaml \
  --output-wav /tmp/tb303-dandrum.wav \
  --output-metrics /tmp/tb303-dandrum.csv
```

See [the sound implementation workflow](docs/sound-implementation-workflow.md)
for analyzing external clone, software-instrument, or hardware recordings with
the same settings and promoting useful observations into regression tests.

CMake exposes the same Rust tests through CTest for CI:

Node.js is required when configuring the CMake project because CTest also runs
the WebView control, page, and native bridge JavaScript checks.

```bash
$HOME/.local/bin/cmake -S . -B build
$HOME/.local/bin/cmake --build build
ctest --test-dir build
```

## Plugin demo development

The same CMake build produces the JUCE standalone plugin at
`build/dandrum-plugin_artefacts/Standalone/Dandrum` and a VST3 bundle at
`build/dandrum-plugin_artefacts/VST3/Dandrum.vst3`. Run the standalone plugin with:

```bash
./build/dandrum-plugin_artefacts/Standalone/Dandrum
```

The shipped `Dandrum` demo opens the TB-303 patch with its embedded React panel. The separate [drum sampler VST3 example](docs/advanced-sampler.md) opens a prepared sample kit with MIDI drum pads, shared host controls, and separate host controls for each pad. `InstrumentDemoConfiguration::kick()` selects a second 808 kick patch and optional Sound Lab fixture/source through the same processor. The Web adapter chooses the fallback kick and sampler pages by instrument ID; HTML is not part of the instrument configuration. Fallback pages load `/shared-instrument-ui.js` for metadata-driven controls and playable notes. A Sound Lab-enabled page places `<!--sound-lab-panel-->`, `/*sound-lab-style*/`, and `<!--sound-lab-script-->` markers where the shared feature should appear. The editor fills those markers only while the active instrument matches the configured Sound Lab source and ID.

To build and run the native editor without Node or WebView/WebKit dependencies:

```sh
$HOME/.local/bin/cmake -S . -B build-native -DDANDRUM_NATIVE_ONLY=ON
$HOME/.local/bin/cmake --build build-native --target dandrum-native-editor-smoke-test dandrum-plugin_VST3 dandrum-sampler-plugin_VST3
ctest --test-dir build-native -R native-editor-smoke --output-on-failure
```

The native editor target currently verifies processor ownership, instrument identity and editor construction. Parameter editing and prepared sample views are subsequent UI tasks.

Developer builds of the original `Dandrum` demo resolve maintained patches and fixtures from the CMake source checkout, even when the process starts elsewhere. The sampler VST3 embeds its default patch and synthetic WAV, then stages them before playback. Other developer demo assets are not packaged for installation. The executable browser, bridge, processor, and Rust checks run through `ctest --test-dir build --output-on-failure`.

## Realtime Callback Contract

The audio and MIDI callback paths follow strict realtime-safety constraints to avoid glitching, priority inversion, or
unbounded latency.

### Audio callback (`RustEngineSource::getNextAudioBlock`)

- **No locks**: Must never acquire `engineLock` (or any mutex/critical section).
- **No I/O**: Must never access the filesystem, parse YAML, load samples, or perform network operations.
- **No console**: Must never write to `std::cout`, `std::cerr`, or any logging stream.
- **No allocation**: Must never allocate on the heap during steady-state rendering (all scratch buffers are prepared
  upfront).
- **Drains events**: Reads pending MIDI events from the lock-free SPSC queue (`pendingMidiEvents`) at the start of each
  block.
- **Renders directly**: Calls `dandrum_kernel_render` with a planar named `master` bus, using
  prepared engine state.

### MIDI callback (`MidiToRustEngine::handleIncomingMidiMessage`)

- **No locks**: Must never acquire `engineLock`.
- **No console**: Must never write to `std::cout` or `std::cerr`.
- **No I/O**: Must never access the filesystem or perform blocking operations.
- **Non-blocking submission**: Enqueues events into the lock-free SPSC `pendingMidiEvents` array.
- **Overflow reporting**: If the event queue is full, the event is silently dropped and the drop counter is incremented
  atomically.

### Patch loading / preparation (off-callback)

- **Holds `engineLock`**: Patch filesystem I/O, YAML parsing, graph construction, and asset preparation happen under the
  CriticalSection.
- **Prepares realtime state**: `prepareToPlay` prepares the legacy engine or calls
  `dandrum_kernel_prepare_file` with the sample rate, maximum block size, and
  declared named buses, allocating scratch buffers off the audio thread.
- **Engine replacement**: Old engine state remains alive (via the lock) until no callback can access it. Destruction
  under the lock ensures safe teardown.

### Bounded event handoff (C FFI)

- `dandrum_realtime_event_queue_create(capacity)` — creates a fixed-capacity queue.
- `dandrum_realtime_event_queue_note_on` / `note_off` — non-blocking submit, returns `0` (accepted) or `1` (dropped).
- `dandrum_realtime_event_queue_dropped_count` — reports total dropped events since creation.

### Kernel buses and public controls (C FFI)

`dandrum_kernel_prepare_file` binds host buses by name, direction, and channel
count. A missing root output or a mismatched width fails preparation; an
unbound root input reads silence or its declared control default. Hosts can
enumerate the prepared root interface with `dandrum_kernel_root_port_count`
and `dandrum_kernel_root_port`, and report latency with
`dandrum_kernel_total_latency_samples`. Event root ports are discoverable but
do not use planar float views.

For each render call, `dandrum_kernel_render` receives planar input and output
views. Each view's `busIndex` is its zero-based position among the matching
direction's declarations supplied at preparation; views may arrive in any
order. Render uses those prepared indices, so view names are informational and
may be null. The caller keeps channel-pointer arrays and sample buffers valid
through the call; the engine does not retain them. The JUCE demo and
plugin bind a stereo `master` output. The plugin also binds public root control
inputs and uses `dandrum_kernel_set_public_numeric_parameter_by_slot` to apply
prevalidated numeric values without allocating in the audio callback. Note
events use `dandrum_kernel_note_on_at` and `dandrum_kernel_note_off_at`.

### Reset / panic (C FFI)

When a voice slot is assigned to a new note, its filter, envelope follower, and other carried DSP state start clean;
global reverb and echo tails continue through ordinary note activity. Oscillator phase remains free-running across note
retrigger.

A host can call `dandrum_kernel_reset(instrument)` for panic, all-notes-off, or patch reload. The call
returns `true` for a live handle and clears pending notes, active voices, and effect tails. The handle
remains usable for later notes and renders. Serialize reset with rendering and MIDI calls on that handle, as for other
mutable engine FFI calls.

### Oversized block handling

If the audio callback delivers a block larger than the prepared max block size, the engine splits the render internally
into prepared-size chunks. This avoids unbounded per-callback allocation while still producing correct output.

## MIDI Input

JUCE owns MIDI device IO and forwards note events into the Rust engine.

```bash
./build/dandrum-drum-machine-demo_artefacts/dandrum-drum-machine-demo --list-midi-inputs
./build/dandrum-drum-machine-demo_artefacts/dandrum-drum-machine-demo --midi-input 0
```

The default no-argument command plays a Rust-generated test note and exits. MIDI mode stays open until Ctrl+C.

For test harnesses without a physical MIDI device, inject a synthetic JUCE MIDI note through the same MIDI handler path:

```bash
./build/dandrum-drum-machine-demo_artefacts/dandrum-drum-machine-demo --test-midi-note 60
```
