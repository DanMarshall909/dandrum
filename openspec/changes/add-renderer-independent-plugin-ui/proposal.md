## Why

Dandrum needs the downloaded design system to support both a production WebView editor and a native C++ JUCE editor. A shared UI contract will let either editor present the prepared instrument, control its public parameters, and display asynchronous analysis while keeping visualization work subordinate to audio rendering.

## What Changes

- Introduce typed C++ UI state, commands, capabilities, and background-analysis services shared by native JUCE and WebView adapters.
- Preserve Rust's independence from JUCE, browser transport, and visual presentation; extend the C FFI only where prepared metadata or bounded telemetry needs an explicit lifetime-safe handoff.
- Prove both renderer paths with the same parameter knob, meter, and prepared-waveform fixtures. Provide a native-only build with browser support disabled and no frontend build prerequisite.
- Adopt the revised Dandrum design system, including `KeyMap`, `LayerStack`, and `OutputBusses`, with one token source generating CSS and C++ constants.
- Present the loaded instrument's actual zones, control groups, source types, module chains, and host buses. Capability-gate data or operations absent from the prepared instrument or host.
- Preserve external authoring and explicit reload for structural changes. The design package's drag-to-edit zones and add/remove/reroute controls are reference interactions, not authorization to add graph authoring to the plugin.
- Add bounded asynchronous meter/activity/cursor telemetry, prepared waveform and spectrogram analysis, and a subscribed live analysis stream. Drop visual history on overload while preserving audio, note-release safety, and latched clip state.
- Package production web assets and redistributable fonts locally; retain native parameter identity, automation, state, and MIDI behaviour across editor choices.
- Replace the existing TB-303 example page with the supplied React panel, binding its playable keyboard and supported controls to the same authoritative command surface. Present pattern/transport controls as unavailable until the patch supplies those capabilities.

## Capabilities

### New Capabilities

- `renderer-independent-plugin-ui`: Typed shared state/commands, renderer parity, independent builds, design tokens, and capability-aware presentation of the revised design system.
- `plugin-visual-telemetry`: Bounded publication, measured meter semantics, pad/cursor feedback, backpressure, lifetimes, and audio-priority verification.
- `plugin-analysis-display`: Shared prepared waveform and spectral analysis, optional subscribed live capture, neutral result formats, caching, cancellation, and stale-result rejection.

### Modified Capabilities

None. These additive capabilities preserve `plugin-integration`'s existing public-parameter and external-authoring requirements and consume the existing `advanced-sampling-options` and `host-buses` contracts.

## Impact

- C++: split shared commands/state from `InstrumentHostWebBridge`, make editor selection independent of `InstrumentDemoConfiguration::indexHtml`, add a native JUCE component path, and scope workers/caches/subscriptions outside the callback.
- Web: introduce a production frontend build and thin transport adapter; adapt the supplied component sources to typed commands and authoritative snapshots.
- Rust/FFI: reuse prepared source, region, slice, zone and control-group metadata; add only covered read-only metadata access or bounded capture seams required by the UI.
- Build/tests: separate common, native, and browser targets; run renderer contract tests, deterministic DSP/analysis tests, callback-safety checks, packaging checks, and runtime stress/visual inspection.
- Existing instrument and host parameter IDs and persisted DSP state remain compatible. Editor choice is presentation configuration, not a new instrument identity.

## Baseline And Design Evidence

- Implementation baseline rechecked on 2026-10-02: `main` and `origin/main` resolve to `d02ee1b6126fa9fb579ac8380716c691a280abdb`, which imports the design reference above the integrated sampling and UI proposal histories. PR #4 is merged. The implementation worktree is `/tmp/dandrum-react-plugin-editors` on `work/react-plugin-editors-2026-10-02`; the original sampling worktree remains at `3b313a7`. Integrating main preserves the completed implementation slices. Recheck upstream and worktrees before publication.
- Prepared against published sampling revision `3b313a78680f305d10b84cf56dd6c35a6f4752e8` on `work/advanced-sampling-options-2026-10-01`. That revision archives `add-advanced-sampling-options`; this proposal follows it without reopening its completed tasks. Its reported validation is historical evidence, not verification of this new UI.
- Design input: `Dandrum Design System (1).zip`, SHA-256 `f42e8cb34db9120807577a00af830dd41bcce8ea06c3f7aa9355f2593276b887`. This revision adds the three display components; its native handoff and original sampler mockups are unchanged. A byte-preserved copy is now in [docs/design-system/reference](../../../docs/design-system/reference/), with [provenance](../../../docs/design-system/provenance.json) and maintained [adaptation rules](../../../docs/design-system/README.md).
- TB-303 design input: `/home/dan/Downloads/tb303-react.zip`, SHA-256 `30e47b38d24a23f62b9b54a4d24ca651cb8a562294e280c8a46abbb079b187a0`, also checked out at `/home/dan/code/tb303-react` revision `dd05982`. Its panel is a standalone visual demo; default step lights, transport state, knob values and key toggles are not engine state and must be replaced or disabled during integration.
- The current kit uses soft snare velocities 1-63 and hard velocities 64-127, with shared and per-pad controls. The export's 1-95/96-127 mock split and patch-wide-only assumption must not override loaded metadata.
- Source references: [sampler behaviour](https://github.com/DanMarshall909/dandrum/blob/3b313a78680f305d10b84cf56dd6c35a6f4752e8/docs/advanced-sampler.md), [plugin contract](https://github.com/DanMarshall909/dandrum/blob/3b313a78680f305d10b84cf56dd6c35a6f4752e8/openspec/specs/plugin-integration/spec.md), and [named buses](https://github.com/DanMarshall909/dandrum/blob/3b313a78680f305d10b84cf56dd6c35a6f4752e8/openspec/specs/host-buses/spec.md).
