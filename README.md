# Dandrum

Headless-first OSS virtual instrument experiment.

## Launch demos

From this repository, run `./demo` or `./demo --list` to see demo names, descriptions and source checkouts. Launch one with:

```bash
./demo tb303                          # embedded standalone, real audio
./demo react303                       # embedded React standalone, real audio
./demo react-trigger                  # full sampler shell in JUCE, real sample playback
./demo sampler                        # embedded sampler standalone, real audio
./demo filter-slint                   # experimental native Slint filter effect
./demo filter-jive                    # experimental native JIVE filter effect
./demo trigger-slint                  # silent native Slint macro performance preview
./demo slint-library                  # complete guide component catalog, silent native UI
./demo sampler-slint                  # advanced native sampler editor, silent mock engine
./demo drums --list-midi-inputs
./demo drums --midi-input 0
./demo trigger -- --port 8324          # silent React shell in a browser
```

The launcher uses this checkout first, then registered Dandrum Git worktrees in pathname order. It prints the selected checkout, builds only the requested native target, and installs locked npm dependencies when missing or older than the package lock. Demos absent from all those checkouts are listed as unavailable. Native demos need the CMake/Cargo/JUCE prerequisites below; React previews also need a Node version supported by their package and npm.

Embedded WebView demos also need GTK3 and WebKitGTK development packages on Linux.

Native launches preserve the configured CMake build type. With a multi-config generator they choose Release if available, otherwise the first configured type, and run the executable from that configuration.

`react303` and `react-sampler` embed React in JUCE and use the Rust engine. They use an isolated `build/demo-webview/` configuration so an existing native-only build keeps its settings. `react-trigger` embeds the full React sampler shell with real sample playback in `build/trigger-webview/`; supported sample edits prepare through the native engine before publication. `tb303-web` and `sampler-web` are browser panel previews that require the JUCE host bridge for audio. `trigger` uses a silent mock engine; `instruments` is a visual design prototype. Browser demos stay in the foreground and print their URL; open that URL in your browser. Ctrl+C stops the demo. Existing dev servers keep running; pass another `--port` if necessary.

Move the full Trigger standalone by dragging its system title bar. Its title follows the current patch, for example `trigger - Advanced Drum Kit`. The Options menu provides audio/MIDI settings and state save, load and reset actions. The sampler's size choices resize the editor inside the window.

`filter-slint` and `filter-jive` are [experimental stereo filter effects](spikes/filter-ui/README.md) with native framework UIs and real Rust audio. They process stereo input and can use an optional audition-generated test signal. The launcher enables their opt-in dependencies in an isolated Release/native-only `build/filter-ui-spikes/` configuration and skips npm preparation. These experiments do not migrate the production instrument UIs.

See the [React editor performance measurements](docs/resource-measurements/2026-10-07-react-ui-performance/README.md) for CPU, memory and control latency before and after optimization, including platform and verification limits.

`trigger-slint` is a silent native Slint preview of Trigger's eight macro controls. It uses the opt-in `DANDRUM_BUILD_TRIGGER_SLINT=ON` target in the same isolated Release/native-only `build/filter-ui-spikes/` configuration and skips npm preparation. See [the preview development guide](spikes/trigger-slint/README.md) for UI authoring and native verification commands.

`slint-library` is the [reusable DanDrum Slint design library](ui/slint/README.md): all 29 guide components, a themed scrollbar, composable subcomponents, and a six-page catalog. It uses the maintained design guide and shared tokens. Install the Slint 1.18.1 C++ SDK and set `CMAKE_PREFIX_PATH` to its directory. Its isolated `DANDRUM_SLINT_LIBRARY_ONLY=ON` build needs a C++20 compiler and CMake; it does not configure JUCE, Rust, Node, or audio. Windows users can run `python scripts/demo_launcher.py slint-library`.

`sampler-slint` launches the [advanced sampler Slint editor](ui/advanced-sampler/README.md) from its independent `DANDRUM_ADVANCED_SAMPLER_ONLY=ON` build. It uses Slint 1.18.1 and a silent mock engine adapter. Native binding tests run without a display; pointer and screenshot tests support `--headless` and use a private virtual display.

[![Made with Slint](ui/advanced-sampler/assets/MadeWithSlint-logo-dark.svg)](https://slint.dev)

Maintain demo registrations in [scripts/demos.json](scripts/demos.json) whenever a demo is added, renamed, moved or removed. Native entries can set `nativeOnly`, `buildDirectory`, and `cmakeOptions` (CMake definitions without the `-D` prefix); omitted fields preserve the normal build and React settings. Update launch tests and relevant documentation in the same change. CTest's `demo-launcher-inventory` check rejects unregistered JUCE standalone, console, GUI and direct CMake application targets, incorrect native artifact paths and unregistered local React dev packages. Run `python3 -m unittest discover -s tests/python -p 'test_demo_*.py'` for focused launcher checks.

## First Sound

The first milestone is deliberately tiny: prove the JUCE wrapper can open the default audio device while Rust owns the
sample generation.

Native Linux dependencies for JUCE:

```bash
sudo apt install -y libasound2-dev libx11-dev libxext-dev libxinerama-dev libxrandr-dev libxcursor-dev libxrender-dev libfreetype6-dev libfontconfig1-dev libgl1-mesa-dev libcurl4-openssl-dev
```

The default WebView build also requires Node.js and npm. Install the React test
dependencies from the checked-in lockfile before configuring:

```bash
npm ci --prefix web/sampler
$HOME/.local/bin/cmake -S . -B build
$HOME/.local/bin/cmake --build build
./build/dandrum-drum-machine-demo_artefacts/dandrum-drum-machine-demo
```

This uses JUCE as the wrapper/host side. The current binary links a Rust static library from `src/rust-engine/` and
calls it from the JUCE audio callback.

## UI design

The current [Dandrum design system](docs/design-system/README.md) includes the reviewed Das Sampler mockups, component references, tokens, SVG assets and JUCE handoff. See the [renderer-independent UI plan](openspec/changes/add-renderer-independent-plugin-ui/design.md) for the native/WebView architecture and implementation tasks.

The [Trigger standalone React shell](web/trigger/README.md) implements the supplied Trigger design with a silent mock adapter. Run `npm ci` in `web/trigger` before CTest; the `trigger-react-shell` CTest runs its type, component, unit, production-build and browser checks. Browser verification uses Chrome (`TRIGGER_CHROME` can select another executable).

## JUCE-free engine build

On Linux/WSL, the engine and its independent native ABI/service tests can be configured without
JUCE, Node.js, React assets or UI toolkits. Use CMake, a C++20 compiler and Cargo at
`$HOME/.cargo/bin/cargo`:

```bash
cmake -S . -B build/engine-only -DDANDRUM_ENGINE_ONLY=ON -DCMAKE_BUILD_TYPE=Release
cmake --build build/engine-only --parallel 2
ctest --test-dir build/engine-only --output-on-failure
```

This builds the existing Rust static library and independent C++ tests for named buses, signed
rendering, sample-offset MIDI, waveform services, meters, commands and Sound Lab. Its build
regression uses a first-party source fixture that omits JUCE, browser assets and UI toolkits,
and keeps the reusable build cache in `build/engine-only-regression/`. The existing JUCE build
separately checks that its comparison host/plugin targets and engine tests remain available.

Leave `DANDRUM_ENGINE_ONLY` disabled for the existing JUCE host/plugins. It rejects combinations
with `DANDRUM_NATIVE_ONLY`, React embedding and Slint/JIVE product options; use separate build
directories for those configurations. Mutation CTest remains separately opt-in.

This is the independent build prerequisite for [issue #24](https://github.com/DanMarshall909/dandrum/issues/24).
Replacement plugin formats, devices, parented React editors, the #23 session contract and installed
resource packaging still need their migration proofs. It does not replace the JUCE comparison product.

## Experimental JUCE-free CLAP proof

On Linux/WSL, opt in to the headless proof using the same engine build directory and cache:

```bash
cmake -S . -B build/engine-only -DDANDRUM_ENGINE_ONLY=ON -DDANDRUM_CLAP_PROOF=ON -DCMAKE_BUILD_TYPE=Release
cmake --build build/engine-only --target dandrum-clap-proof --parallel 2
ctest --test-dir build/engine-only --output-on-failure -R '^clap-(dynamic-host|build-boundary)$'
```

The exported `build/engine-only/clap/DandrumHeadlessProof.clap` contains separate experimental
TB-303 and signed stereo gain-fixture descriptors. The dynamic host tests load that binary and
prove root controls, sample offsets/order, MIDI 1 key semantics, signed audio, reset/reactivation,
independent instances, fixture-state save/load and stopped parameter flushing. The build guard repeats packaging with JUCE/browser/UI source trees
absent and checks incremental rebuilds after embedded schema/library changes. For warm
functional feedback, select only `clap-dynamic-host`; run the slower `clap-build-boundary`
at the final checkpoint. First-time Cargo dependency fetching needs network access; the exact Clack revision and
lockfile are retained. The engine and adapter use matching release profiles, while only the new
adapter's dev/test output strips debug information.

This is a rendering and fixture-state proof for [issue #24](https://github.com/DanMarshall909/dandrum/issues/24),
not a production plugin or replacement-stack selection. It opens no audio device and has no
editor. Its state extension saves schema-version-1 JSON containing the experimental instrument ID
and complete named public parameter values. Input is limited to 16 KiB; invalid state or stream
failure preserves the working controls. This format contains no Trigger authoring document,
sample assets or undo journal. MIDI 1 notes use the engine's existing key semantics; independent
CLAP note IDs/expression, MIDI 2 and per-note parameter modulation are not advertised. There
are at most 1,024 input events per block; same-offset notes must also fit the prepared engine
queue. Invalid event/layout/value input is silently rejected before admission and leaves host
processing eligible to continue; full callback allocation/I/O and platform hardening are separate
required proofs. Fractional rates reach the existing floating-point DSP runtime; integer preparation
metadata is rounded for these resource-free fixtures. Asset-rate handling remains unproved.

The experimental descriptor IDs are `org.dandrum.experimental.tb303` and
`org.dandrum.experimental.signed-gain`; they are distinct from the retained JUCE products.
These proof identities are not a stable release ABI. Parameter IDs are one-based positions in
each fixture's authored preset surface; host values target the named public root control.
All parameters are global and automatable, with no per-note or modulation flags.

| Descriptor | Parameter ID | Public alias | Root control | Range | Default |
| --- | --- | --- | --- | --- | --- |
| TB-303 | 1 | `filter.cutoff` | `filter_cutoff` | 0.02–0.9 | 0.4 |
| TB-303 | 2 | `filter.resonance` | `filter_resonance` | 0–1 | 0.65 |
| TB-303 | 3 | `filter.envelope_modulation` | `filter_envelope_modulation` | 0–1 | 0.4 |
| TB-303 | 4 | `filter.decay_ms` | `filter_decay_ms` | 50–3000 | 1000 |
| TB-303 | 5 | `accent.brightness` | `accent_brightness` | 0–0.5 | 0.12 |
| TB-303 | 6 | `amp.release_ms` | `amp_release_ms` | 5–500 | 60 |
| TB-303 | 7 | `slide.time_ms` | `slide_time_ms` | 0–500 | 60 |
| Signed gain | 1 | `fixture.level` | `level` | 0–1 | 0.5 |

Each descriptor exposes one float32 stereo main output at audio port ID 0, without an
in-place pair. Signed gain additionally exposes stereo input ID 0. TB-303 instead exposes
MIDI 1 input note-port ID 0; neither descriptor exposes an output note port.

The [progress receipt](docs/migrations/juce-replacement-progress.md) retains the pending session,
full Trigger state/assets, callback hardening, parented production React, native second OS, VST3/AU, devices and packaging
gates. The [dependency receipt](docs/migrations/juce-free-clap-dependencies.json) records exact
provenance and scope; project/distribution licence decisions remain pending. Keep the existing
JUCE products for comparison. No launchable demo or production cutover is added here.

The opt-in session-backed Trigger product builds beside the proof artifact:

```bash
npm --prefix web/trigger run build:native
cmake -S . -B build/engine-only -DDANDRUM_ENGINE_ONLY=ON -DDANDRUM_CLAP_PROOF=ON -DDANDRUM_TRIGGER_CLAP_EDITOR=ON -DDANDRUM_TRIGGER_EDITOR_BUNDLE_DIR="$PWD/web/trigger/dist-native" -DCMAKE_BUILD_TYPE=Release
cmake --build build/engine-only --target dandrum-trigger-clap-editor --parallel 2
ctest --test-dir build/engine-only --output-on-failure -R '^(clap-dynamic-host|trigger-clap-dynamic-host|trigger-host-tests|trigger-editor-tests)$'
```

`DandrumTrigger.clap` adds `org.dandrum.trigger` with the Advanced Drum Kit,
23 physical host parameters and version-2 Trigger state. Keep its sibling
`DandrumTrigger.resources` directory beside the plugin when copying it to another
location. Initialization resolves that explicit directory only. Trigger accepts
integral sample rates, float32 stereo output and at most 64 input events per block;
invalid batches preserve accepted values and playback position while clearing output.
The current product is headless; the packaged React assets support the pending
CLAP GUI integration. The default proof artifact retains its two fixture descriptors.

The default-off Cargo `trigger-editor` feature is a developer lifecycle test
checkpoint. Parameter reads, gestures and feedback, heartbeat/expiry servicing,
mapping/status/undo and persistent worker servicing are still unjoined. Its GUI
is not ready for instrument editing or normal playing. These remain required
work in the automation and recovery boundaries (G2/G3); the maintained CMake
product above continues to use the headless `trigger` feature.

For native lifecycle verification, configure the same build with
`-DDANDRUM_TRIGGER_EDITOR_NATIVE_X11=ON`, build its resource target, and run
`ctest --test-dir build/engine-only --output-on-failure -R '^trigger-clap-native-'`
in the Linux GNU/X11 session. This opt-in lane builds a private test fixture,
selects the library and test executable from Cargo's JSON artifact records,
copies their exact bytes and resources, then runs each ignored case in a fresh
process. `trigger-clap-native-fixture/binding.json` records the selected paths
and hashes. Do not substitute the top-level Cargo library path: a test build can
report a different library under `release/deps`. Ordinary `cargo test` skips
these native cases. This fixture adds no editor demo or installed GUI product.

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

The default WebView configuration requires Node.js and the sampler package's
locked dependencies because CTest runs the JavaScript checks and renders the
shared icons with React. CMake checks these dependencies before configuring
JUCE. The native-only configuration does not require Node.js or npm packages.

```bash
npm ci --prefix web/sampler
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

The `Dandrum` TB-303 plugin embeds its default patch and Sound Lab fixture, then stages them in the user's application data directory before loading, so its instrument and React panel work outside the source checkout. The sampler VST3 likewise embeds its default patch and synthetic WAV. The optional 808 kick developer demo still resolves its patch and fixture from the CMake source checkout. The executable browser, bridge, processor, and Rust checks run through `ctest --test-dir build --output-on-failure`.

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
