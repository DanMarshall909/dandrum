# UI implementation baseline (2026-10-02)

The implementation worktree starts from `main` at `0b9981917d710586f75832a024ec7eea5ab37a4f`. The existing editor is browser-only and uses `InstrumentHostWebBridge` with four native commands: `getParameters`, `setParameter`, `noteOn`, and `noteOff`.

## Observed contracts

- The bundled drum kit exposes 23 fixed host-backed public controls: five shared controls plus named per-pad controls. `PluginSamplerHostTest.cpp` checks the IDs and signed kick output (`-0.5` in both channels on the first frame), a shared-level change (`-0.25`), independent pad edits, state restore, and a preset.
- `getParameters` returns ID, display name, and normalized value. It has no instrument generation or scope field. A successful `setParameter` call brackets **each update** with one JUCE begin/end gesture; the new baseline assertion in `PluginEditorBridgeTest.cpp` records this before the command contract is changed.
- Editor notes enter a fixed 128-event FIFO. Current tests cover note-on rendering, note-off admission, full-queue rejection and a drop counter. A note-off can currently be rejected when the queue is full; the planned session-release contract must fix that before relying on held editor notes.
- A successful instrument reload advances the parameter-surface generation; the browser bridge refreshes and exposes the replacement IDs. Existing C++ tests also cover stable host slot count and removal of obsolete public IDs.

## Verification and coverage limits

- The published build at `/home/dan/code/dandrum/build` passed `cxx-sampler-plugin-host`, `cxx-plugin-construction`, `cxx-plugin-host-parameter-surface`, `cxx-plugin-editor-bridge`, `shared-instrument-ui`, and `tb303-page` before production edits. The isolated worktree's focused build passed `cxx-sampler-plugin-host`, `cxx-plugin-editor-bridge` (including the added gesture characterization), `shared-instrument-ui`, and `tb303-page`.
- The repository's strict LLVM coverage gate measures Rust engine files. The current C++/JavaScript UI tests have no instrumented line-coverage report, so they establish the named contracts above but no line-coverage percentage. Before changing a bridge path, add behavior assertions for its acceptance and rejection branches and run the focused suite.
