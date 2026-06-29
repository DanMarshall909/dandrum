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
- Rust mutation tests (install once with `cargo install cargo-mutants`): `$HOME/.cargo/bin/cargo mutants --manifest-path src/rust-engine/Cargo.toml` (runs ~8 min; tests test quality by mutating source code and checking that tests catch the mutations).
- CI-ready test path: configure/build with CMake, then run `ctest --test-dir build`.

## Development Practice
- Use TDD for implementation work: write or update a failing test that describes the behavior first, run it to confirm the failure when feasible, then add the smallest production change that makes it pass.
- Keep tests close to the behavior under development. Prefer Rust unit tests for engine core behavior and acceptance tests only for end-to-end CLI/rendering behavior.
- Before changing system behavior, explain what behavior is changing, why it should change, and which tests specify it.
- Every new behavior must be specified by a test before implementation. Tests should describe externally observable behavior, not implementation minutia.
- After completing and verifying behavior work, proactively look for refactoring opportunities before moving on. Split or extract code only after the behavior at that boundary is covered by tests, and require 100% coverage for any newly extracted module before committing it.
- Run mutation tests periodically (`cargo-mutants`) to catch weak or missing test coverage, especially after implementing path-critical DSP or control-flow behavior.
- Pre-push hook (`.githooks/pre-push`) runs `cargo test` then `cargo mutants` before every push. Skip with `git push --no-verify` when needed.
- Teach Rust through this project as work proceeds: briefly explain Rust syntax, ownership/borrowing, traits, macros, modules, error handling, and testing patterns when they appear in the code being changed, without turning implementation updates into long tutorials.
- Teach modern C++ through this project when C++ code changes: the user is an experienced pre-2000 C++ developer, so briefly introduce post-2000 language/library features when they appear, without turning implementation updates into long tutorials.
- Do not mark OpenSpec implementation tasks complete until the related tests and relevant build/test commands pass, or until any unavoidable verification gap is documented.
- Treat each verified material change as a commit boundary. Before committing, inspect `git status`, `git diff`, and recent log, stage only the intended files, and commit the focused change separately from unrelated work.

## Current Technical Debt
- MIDI callbacks and audio callbacks share the Rust engine through a simple JUCE `CriticalSection`. This is acceptable for the proof, but should become a lock-free event queue before serious realtime work.

## OpenSpec Workflow
- Active repo-local change: `headless-modular-instrument-engine` under `openspec/changes/`; it is spec-driven and has implementation tasks still unchecked.
- Before implementing that planned engine, read the change via `openspec status --change "headless-modular-instrument-engine" --json` and `openspec instructions apply --change "headless-modular-instrument-engine" --json` rather than guessing artifact paths.
- For implementation work on an OpenSpec change, follow the apply workflow: read all `contextFiles` returned by `openspec instructions apply`, implement the next pending task(s), verify with relevant tests/build commands, then update the task checkbox in the change's `tasks.md`.
- Do not skip ahead or broaden scope without checking the OpenSpec task list. If a task is unclear, pause and ask rather than inventing requirements.
- Use the repo-local OpenCode shortcuts/skills when they match the user's intent: `/opsx-propose` for new changes, `/opsx-explore` for investigation/requirements discussion, `/opsx-apply` for implementation, `/opsx-sync` for syncing accepted delta specs, and `/opsx-archive` after a completed change is verified.
- Repo-local OpenCode shortcuts live in `.opencode/commands/`; matching skills live in `.opencode/skills/`.
- Run OpenSpec validation before finalizing or archiving a change, and document any unavoidable verification gaps in the response.

## Architecture Constraints From Specs
- The planned engine core should stay independent of CLI, GUI, plugin, and realtime audio driver/front-end code.
- Planned patches are YAML modular graphs with explicit named typed ports; many-to-one routing requires an explicit mixer/summing module.
- Planned feedback is valid only through explicit delay or future scheduling boundaries.
