## Why

Compare Slint's authoring and AI workflow with Trigger's existing React controls using a small, visually faithful performance view before attempting the full sampler shell.

## What Changes

- Add a standalone, silent Slint performance preview with Trigger's eight macros.
- Match the React charcoal palette, bundled fonts, rotary geometry and compact spacing.
- Support macro dragging, precision editing, keyboard/wheel changes, reset and typed values.
- Register the preview in the maintained `./demo` launcher and document its development workflow.

## Capabilities

### New Capabilities
- `slint-trigger-performance`: A visually faithful, interactive macro performance preview.

### Modified Capabilities

None.

## Impact

Isolated Slint prototype, optional native CMake target, launcher catalog and owning inventory tests. Reuses pinned Slint 1.18.1 and the proven JUCE software-rendering adapter. No engine, production plugin or full-shell changes.
