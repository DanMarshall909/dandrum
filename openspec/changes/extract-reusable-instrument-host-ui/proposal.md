## Why

The TB-303 editor contains instrument-independent WebView controls and host commands, while its processor starts with an 808 kick and Sound Lab uses a 303 fixture. A coherent demo configuration and reusable UI boundary will let another instrument use the same host plumbing without copying the editor.

## What Changes

- Extract the shared WebView parameter and MIDI bridge, metadata-driven controls, keyboard behavior, and resource serving from the TB-303 presentation.
- Define an explicit demo configuration that pairs a starting instrument, optional Sound Lab fixture, match-acceptance source, title, and UI assets. The TB-303 demo will start with the TB-303 patch that its fixture and appearance describe.
- Make Sound Lab an optional editor feature that uses the selected demo's fixture and acceptance source while retaining its existing background controller.
- Prove reuse with a second instrument configuration, distinct fixture and appearance, using the same host and UI plumbing.
- Preserve DSP algorithms, fixture event timing, public parameter IDs, fixed host automation slots, and replacement/state behavior.
- Replace source-text WebView assertions with behavior-focused bridge and page tests where extraction moves implementation.

The existing default plugin instrument changes from the synthetic 808 kick to the TB-303 for the TB-303 demo. Saved plugin state continues to restore its embedded instrument definition.

## Capabilities

### New Capabilities

- `instrument-demo-configuration`: Selects a coherent instrument, fixture, appearance, and optional Sound Lab for a reusable demo host.

### Modified Capabilities

- `plugin-integration`: Allows a metadata-driven WebView control surface alongside the host's stable public parameter and state contract.
- `sound-lab-ui`: Uses the configured fixture and match-acceptance instrument instead of TB-303 paths embedded in editor commands.

## Impact

- JUCE plugin editor, processor startup configuration, WebView assets, and native bridge tests.
- Demo asset lookup and a second instrument fixture/appearance used to prove the shared boundary.
- No Rust DSP, patch-schema, or fixture event-format change. Canonical-port resolution and a step-to-event compiler remain separate proposals.
