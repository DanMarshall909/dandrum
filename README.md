# Dandrum

Headless-first OSS virtual instrument experiment.

## First Sound

The first milestone is deliberately tiny: prove the JUCE wrapper can open the default audio device while Rust owns the sample generation.

Native Linux dependencies for JUCE:

```bash
sudo apt install -y libasound2-dev libx11-dev libxext-dev libxinerama-dev libxrandr-dev libxcursor-dev libxrender-dev libfreetype6-dev libfontconfig1-dev libgl1-mesa-dev libcurl4-openssl-dev
```

```bash
$HOME/.local/bin/cmake -S . -B build
$HOME/.local/bin/cmake --build build
./build/dandrum-beep_artefacts/dandrum-beep
```

This uses JUCE as the wrapper/host side. The current binary links a Rust static library from `src/rust-engine/` and calls it from the JUCE audio callback.

## Engine Development

The headless engine core is implemented in Rust under `src/rust-engine/`. The `core` module is the frontend-independent engine boundary; JUCE, CLI, GUI, plugin, and realtime driver code should stay outside that module.

A machine-readable patch schema lives at `schema/patch.schema.yaml`. It is for editor and external validation; Rust still performs semantic validation when loading patches.

Rust unit tests are the default home for core behavior:

```bash
$HOME/.cargo/bin/cargo test --manifest-path src/rust-engine/Cargo.toml
```

CMake exposes the same Rust tests through CTest for CI:

```bash
$HOME/.local/bin/cmake -S . -B build
$HOME/.local/bin/cmake --build build
ctest --test-dir build
```

## Realtime Callback Contract

The audio and MIDI callback paths follow strict realtime-safety constraints to avoid glitching, priority inversion, or unbounded latency.

### Audio callback (`RustEngineSource::getNextAudioBlock`)

- **No locks**: Must never acquire `engineLock` (or any mutex/critical section).
- **No I/O**: Must never access the filesystem, parse YAML, load samples, or perform network operations.
- **No console**: Must never write to `std::cout`, `std::cerr`, or any logging stream.
- **No allocation**: Must never allocate on the heap during steady-state rendering (all scratch buffers are prepared upfront).
- **Drains events**: Reads pending MIDI events from the lock-free SPSC queue (`pendingMidiEvents`) at the start of each block.
- **Renders directly**: Calls `dandrum_engine_render` with the prepared engine state.

### MIDI callback (`MidiToRustEngine::handleIncomingMidiMessage`)

- **No locks**: Must never acquire `engineLock`.
- **No console**: Must never write to `std::cout` or `std::cerr`.
- **No I/O**: Must never access the filesystem or perform blocking operations.
- **Non-blocking submission**: Enqueues events into the lock-free SPSC `pendingMidiEvents` array.
- **Overflow reporting**: If the event queue is full, the event is silently dropped and the drop counter is incremented atomically.

### Patch loading / preparation (off-callback)

- **Holds `engineLock`**: Patch filesystem I/O, YAML parsing, graph construction, and asset preparation happen under the CriticalSection.
- **Prepares realtime state**: `prepareToPlay` calls `dandrum_engine_prepare_realtime` to set sample rate and max block size, allocating scratch buffers off the audio thread.
- **Engine replacement**: Old engine state remains alive (via the lock) until no callback can access it. Destruction under the lock ensures safe teardown.

### Bounded event handoff (C FFI)

- `dandrum_realtime_event_queue_create(capacity)` — creates a fixed-capacity queue.
- `dandrum_realtime_event_queue_note_on` / `note_off` — non-blocking submit, returns `0` (accepted) or `1` (dropped).
- `dandrum_realtime_event_queue_dropped_count` — reports total dropped events since creation.

### Oversized block handling

If the audio callback delivers a block larger than the prepared max block size, the engine splits the render internally into prepared-size chunks. This avoids unbounded per-callback allocation while still producing correct output.

## MIDI Input

JUCE owns MIDI device IO and forwards note events into the Rust engine.

```bash
./build/dandrum-beep_artefacts/dandrum-beep --list-midi-inputs
./build/dandrum-beep_artefacts/dandrum-beep --midi-input 0
```

The default no-argument command plays a Rust-generated test note and exits. MIDI mode stays open until Ctrl+C.

For test harnesses without a physical MIDI device, inject a synthetic JUCE MIDI note through the same MIDI handler path:

```bash
./build/dandrum-beep_artefacts/dandrum-beep --test-midi-note 60
```
