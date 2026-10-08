## Why

The existing sampler editor does not reflect the supplied Trigger design. Build the requested complete React shell from the downloaded design and its implementation instructions, with a mock engine boundary that can later be implemented by Dandrum.

## What Changes

- Create the Trigger React shell with the supplied eight pages, exact four window layouts, tree, browser, macros, inspector, history and nested menus.
- Implement the documented EngineAdapter, command-based undo/redo, Empty and Felt Kit data, silent telemetry, asynchronous loading and analysis, and actionable failure switches.
- Reach all 24 supplied design states through ordinary UI actions and mock switches.
- Deliver the isolated-component catalog, feature audit and final engine contract/platform documentation.

## Capabilities

### New Capabilities

- `trigger-react-shell`: Faithful, interactive React shell with an isolated mock engine contract, complete editor interactions and reviewable catalog/evidence.

### Modified Capabilities

None. Existing native and host-backed editors keep their contracts.

## Impact

New standalone React/Vite package under `web/trigger`, supplied source/provenance and handoff records under `docs/trigger`, and this OpenSpec change. Uses the supplied Dandrum React primitives and locally bundled fonts. Real audio, DSP, file decoding, disk presets and JUCE implementation are explicitly outside the supplied plan's scope.
