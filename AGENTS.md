# Agent Notes

## Project Shape
- This is currently a tiny C++20 CMake/JUCE app, not yet the planned headless engine.
- Root `CMakeLists.txt` builds one console target, `dandrum-beep`, from `src/juce-wrapper/Main.cpp`.
- `third_party/JUCE/` is vendored JUCE; do not edit it unless the task is explicitly about vendored JUCE changes.

## Build And Run
- Configure/build/run with the README commands: `$HOME/.local/bin/cmake -S . -B build`, `$HOME/.local/bin/cmake --build build`, `./build/dandrum-beep_artefacts/dandrum-beep`.
- JUCE configure needs native Linux dev packages from `README.md`; without them CMake can fail while building `juceaide` on missing headers such as `X11/Xlib.h`.
- There are no configured lint, typecheck, unit test, or CI commands yet.

## OpenSpec Workflow
- Active repo-local change: `headless-modular-instrument-engine` under `openspec/changes/`; it is spec-driven and has implementation tasks still unchecked.
- Before implementing that planned engine, read the change via `openspec status --change "headless-modular-instrument-engine" --json` and `openspec instructions apply --change "headless-modular-instrument-engine" --json` rather than guessing artifact paths.
- Repo-local OpenCode shortcuts live in `.opencode/commands/` (`/opsx-propose`, `/opsx-apply`, `/opsx-sync`, `/opsx-archive`, `/opsx-explore`).

## Architecture Constraints From Specs
- The planned engine core should stay independent of CLI, GUI, plugin, and realtime audio driver/front-end code.
- Planned patches are YAML modular graphs with explicit named typed ports; many-to-one routing requires an explicit mixer/summing module.
- Planned feedback is valid only through explicit delay or future scheduling boundaries.
