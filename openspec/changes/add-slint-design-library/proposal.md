## Why

DanDrum has a complete design guide but only a macro-strip Slint experiment. Instruments need the entire shared visual vocabulary as small, composable native components, with supplied data and explicit callbacks that can attach to the existing engine contracts.

## What Changes

- Add the guide's 29 public UI components and themed scrollbar under `ui/slint`, decomposed into reusable drawing, input, layout and display subcomponents.
- Generate Slint tokens from the maintained source already used by CSS and C++; reuse the original font binaries and vector icons.
- Supply a public import surface, typed display models, capability-aware controls and a runnable interactive component catalog with composed instrument examples.
- Verify compilation, rendered default/compact states, keyboard/pointer interaction and command boundaries; document each guide-to-Slint mapping and integration limits.

## Capabilities

### New Capabilities

- `slint-design-library`: Reusable Slint implementation of the maintained DanDrum design guide, including catalog, state contracts and verification.

### Modified Capabilities

None. Existing engine, plugin and macro-preview contracts remain separate.

## Impact

Adds renderer-only Slint sources, catalog/verification support and a generated Slint token output. Extends the existing token generator and its tests without changing existing CSS/C++ values. Uses the project's pinned Slint 1.18.1. No audio callback, DSP, persistence format or existing host parameter identity changes.
