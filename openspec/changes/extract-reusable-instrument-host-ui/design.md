## Context

`DandrumAudioProcessor` already owns the reusable engine, fixed host parameter slots, editor MIDI queue, state, and replacement transaction. `PluginEditor.cpp` combines generic parameter/MIDI commands with Sound Lab commands and TB-303 asset selection. `Tb303WebUi.h` combines metadata-driven control behavior, Sound Lab presentation, and a TB-303 enclosure in one HTML literal. The processor currently loads `synthetic-808-kick.yaml`, while Sound Lab renders `tb303-acid-poc.yaml` and match acceptance names `tb303-acid.yaml` directly.

The checked-in plugin spec still names native JUCE controls, while the current implementation uses a WebView. This change updates that contract to describe the actual metadata-driven editor boundary. The current tests cover processor state/reload and Sound Lab controller behavior, but the WebView contract test searches source strings and does not prove the bridge after code moves.

## Goals / Non-Goals

**Goals:**

- Let two instrument demos select different patches, fixtures, and appearances through one host and WebView bridge.
- Keep parameter gestures, note queueing, browser events, resource responses, and Sound Lab jobs behaviorally stable.
- Ensure a configured Sound Lab fixture and accepted match correspond to the demo's instrument.
- Keep Sound Lab absent when no fixture is configured, without adding conditionals to the shared parameter/MIDI controls.

**Non-Goals:**

- Generalize TB-303 DSP or add a sequencer engine.
- Infer YAML ports, compile steps to events, or move C++ preset/identity parsing to Rust in this change.
- Change public parameter IDs, host automation slot count/order, fixture timing, or the Rust render path.

## Decisions

### Keep one processor; inject demo selection at construction

Use a small immutable demo configuration with an instrument path, optional fixture path, match-acceptance source, title, and UI asset entrypoint. The shipped TB-303 demo selects the TB-303 patch at startup. A second configuration selects the existing synthetic 808 kick patch, a dedicated kick fixture, and a distinct page. Tests can construct either configuration; the plugin factory selects the shipped TB-303 configuration. `DandrumAudioProcessor` remains the common host and continues to restore saved instrument content before applying saved public values. This avoids a synth-specific processor hierarchy.

Alternative considered: keep the kick as the default while presenting a 303 UI. That leaves the current instrument, fixture, and appearance in conflict on a fresh plugin instance.

### Separate bridge capabilities by ownership

Move WebView bootstrap, parameter snapshot/get/set, note-on/off, and shared asset serving into a host-facing bridge. Keep Sound Lab commands and WAV resources in an optional integration object around `SoundLabController`; the editor composes it only for configurations with a fixture. The editor owns the browser and configures its appearance/asset entrypoint. Use RAII lifetime order so native callbacks cannot outlive the bridge or processor they reference.

Alternative considered: one large generic editor with `if (is303)` branches. That would preserve the coupling and make the second instrument proof weak.

### Share behavior while keeping pages distinct

Serve shared JavaScript for native-call/error handling, metadata-driven parameter controls, and the playable keyboard. Keep each demo's HTML/CSS and layout separate. The TB-303 step buttons remain presentation-only until a separately specified sequencer exists. Sound Lab UI behavior can be shared as an optional script and panel contract without forcing every instrument to display it.

Alternative considered: reuse the whole TB-303 page for the second instrument. A skin swap alone would not prove that the host does not own the 303 layout.

### Test through observable boundaries

First characterize the existing bridge commands and page behavior, including invalid arguments, unknown parameter IDs, queue overflow, surface refresh, and Sound Lab resource generation. Add tests that use the extracted bridge/resource handlers and execute page behavior with a minimal browser harness where practical. Replace source-text assertions only after equivalent behaviors are proved. Use the existing processor tests for state restoration, automation-slot identity, and failed-reload recovery; add a second-configuration test that loads and renders its own fixture. Compare the maintained TB-303 fixture output before and after extraction with a deterministic PCM/metrics fingerprint.

Alternative considered: move functions and only adjust string searches. Such tests would pass even if callbacks were never registered or resources were inaccessible.

## Risks / Trade-offs

- Startup instrument changes from kick to 303 → Keep explicit kick-config tests and verify saved kick state still restores; update tests that assumed the implicit default.
- WebView callback lifetime or registration regressions → Exercise commands through the composed bridge and retain processor/state integration tests.
- Fixture and active instrument drift after user reload → Disable or clearly reject Sound Lab actions when the active instrument no longer matches the configured fixture until a compatible instrument is restored.
- Optional Sound Lab adds composition paths → Test both enabled and disabled configurations.

## Migration Plan

1. Record baseline TB-303 fixture audio/metric fingerprint and current plugin/bridge behavior.
2. Introduce configuration and a second demo fixture/page, then switch the shipped TB-303 startup path.
3. Extract the shared native bridge and browser controls; compose Sound Lab only where configured.
4. Run focused C++/web tests, complete CTest, Rust tests, realtime guard, OpenSpec validation, and spec coverage checks. Preserve existing serialized state handling for previously saved projects.

## Open Questions

- Whether the second demo should be exposed through a separate installed plugin target after the test-only configuration proves the boundary. This change requires only the testable second configuration.
