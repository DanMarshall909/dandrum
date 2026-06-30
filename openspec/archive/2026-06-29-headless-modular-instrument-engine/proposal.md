## Why

The project needs a clear product and engineering contract for an OSS virtual instrument that starts headless, with routing and scripting as first-class capabilities rather than GUI-driven afterthoughts. Defining the modular engine first keeps the future GUI, plugin, and library ecosystem grounded in a testable core.

## What Changes

- Introduce a headless instrument engine that loads YAML patch files and renders audio offline.
- Define an explicit modular graph model where modules expose named typed ports and cables connect compatible ports.
- Make VCA/control routing first-class so any compatible VCA/control output can connect to any compatible VCA/control input.
- Make script modules first-class graph participants with declared input/output ports and bounded execution.
- Define feedback handling rules that allow musical feedback only through explicit delay or scheduling boundaries.
- Define the minimum built-in modules needed to prove routing, scripting, modulation, feedback, and offline rendering.

## Capabilities

### New Capabilities

- `headless-engine`: Loading patches, running without a GUI, and rendering offline from input events to WAV output.
- `yaml-patch-format`: Human-readable YAML instrument patch format for modules, ports, connections, scripts, and assets.
- `modular-routing-graph`: Explicit module/port/cable graph with typed signal compatibility and validation.
- `vca-control-routing`: First-class VCA/control signals, compatible port routing, and explicit mixer/summing behavior.
- `script-modules`: Script modules as graph nodes with custom ports, state, event/control processing, and bounded scheduling.
- `feedback-routing`: Cycle detection and valid feedback rules using explicit delay or future-tick boundaries.
- `built-in-modules`: Minimum core module set required to exercise the MVP architecture.

### Modified Capabilities

- None.

## Impact

- Adds OpenSpec planning artifacts for the initial engine architecture and acceptance criteria.
- Establishes YAML as the patch file format before implementation begins.
- Establishes validation requirements that future CLI, GUI, plugin, and library code must obey.
- No application code exists yet, so there are no breaking runtime changes.
