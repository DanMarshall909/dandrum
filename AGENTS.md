# Agent Notes

## Project Shape
- This is currently a C++20 CMake/JUCE wrapper plus Rust engine crate; the planned headless engine core is starting under `src/rust-engine/src/core.rs`.
- Root `CMakeLists.txt` builds Rust crate `src/rust-engine/`, links it into one JUCE console target, `dandrum-beep`, from `src/juce-wrapper/Main.cpp`, and exposes Rust tests through CTest.
- JUCE currently owns audio/MIDI device IO; Rust owns DSP/event state behind a C FFI boundary.
- `third_party/JUCE/` is vendored JUCE; do not edit it unless the task is explicitly about vendored JUCE changes.

## Build And Run
- Configure/build/run with the README commands: `$HOME/.local/bin/cmake -S . -B build`, `$HOME/.local/bin/cmake --build build`, `./build/dandrum-beep_artefacts/dandrum-beep`.
- MIDI input commands: `./build/dandrum-beep_artefacts/dandrum-beep --list-midi-inputs`, `./build/dandrum-beep_artefacts/dandrum-beep --midi-input 0`.
- Synthetic MIDI test command: `./build/dandrum-beep_artefacts/dandrum-beep --test-midi-note 60`.
- CMake expects Cargo at `$HOME/.cargo/bin/cargo`.
- JUCE configure needs native Linux dev packages from `README.md`; without them CMake can fail while building `juceaide` on missing headers such as `X11/Xlib.h`.
- Rust unit tests: `$HOME/.cargo/bin/cargo test --manifest-path src/rust-engine/Cargo.toml`.
- CI-ready test path: configure/build with CMake, then run `ctest --test-dir build`.

## Development Practice
- Use TDD for implementation work: write or update a failing test that describes the behavior first, run it to confirm the failure when feasible, then add the smallest production change that makes it pass.
- Keep tests close to the behavior under development. Prefer Rust unit tests for engine core behavior and acceptance tests only for end-to-end CLI/rendering behavior.
- Do not mark OpenSpec implementation tasks complete until the related tests and relevant build/test commands pass, or until any unavoidable verification gap is documented.

## Current Technical Debt
- MIDI callbacks and audio callbacks share the Rust engine through a simple JUCE `CriticalSection`. This is acceptable for the proof, but should become a lock-free event queue before serious realtime work.

## OpenSpec Workflow
- Active repo-local change: `headless-modular-instrument-engine` under `openspec/changes/`; it is spec-driven and has implementation tasks still unchecked.
- Before implementing that planned engine, read the change via `openspec status --change "headless-modular-instrument-engine" --json` and `openspec instructions apply --change "headless-modular-instrument-engine" --json` rather than guessing artifact paths.
- Repo-local OpenCode shortcuts live in `.opencode/commands/` (`/opsx-propose`, `/opsx-apply`, `/opsx-sync`, `/opsx-archive`, `/opsx-explore`).

## Architecture Constraints From Specs
- The planned engine core should stay independent of CLI, GUI, plugin, and realtime audio driver/front-end code.
- Planned patches are YAML modular graphs with explicit named typed ports; many-to-one routing requires an explicit mixer/summing module.
- Planned feedback is valid only through explicit delay or future scheduling boundaries.
