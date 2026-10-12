# Native sampler engine roadmap

The integrated Trigger editor now plays real samples. Complete the remaining
editor capabilities in the order below. Every item is planned work, not a claim
that the current native adapter implements it.

## Capability inventory

An unavailable editor control does not necessarily mean its DSP is missing.
Check the authoritative [kernel registry](../../src/rust-engine/src/kernel/builtins.rs),
[preparation](../../src/rust-engine/src/preparation.rs) and existing rendered tests
before adding a primitive. The current
[adapter](../../web/trigger/engine/native-engine.mjs) and
[sample compiler](../../web/trigger/engine/native-document.mjs) deliberately reject
features they cannot compile and execute faithfully.

| Editor capability | Current evidence and gap | Planned work |
|---|---|---|
| Sample modes and layers | Forward loops work; preparation explicitly rejects ping-pong loops. The adapter rejects gated/sustain playback, overlapping stacks, nested selectors and velocity crossfades. | Define note-off, boundary, selection and gain semantics; identify which need sample-player/preparation changes and which compose existing graphs. |
| Voice chain | The registry contains oscillator, ADSR, filter, gain and bounded `poly`; Trigger currently compiles only its sample-map template. | Compile declared voice templates to YAML compositions, including per-voice envelopes and filters. Define Lush's actual sound and compose existing modules where possible. Review mono/legato/glide, same-note and exclusive-group behavior separately. |
| Effects | Filter, dynamics, saturator, echo, reverb and convolution already exist. The editor's chains are not compiled. | Bind these existing processors, their order, bypass and resources. Review EQ and Bitcrush against existing compositions before proposing new DSP. |
| Routing and modulation | Mixers, LFO, envelope, multiply, curve mapper, slew and note-to-control primitives exist. Trigger sends, buses, macros and routes are not compiled. | Compose typed cables with stable public control IDs; define MIDI/performance sources, modulation ranges and host output-channel admission. Use explicit feedback-delay boundaries where required. |
| Sample analysis and history | Existing analysis returns peak/RMS and spectral centroid; this does not prove pitch, transient, loop or LUFS analysis. Normalize and collapsed history are rejected. | Add cancellable off-thread jobs for missing analysis results, zero-crossing queries and immutable derived assets. Analysis remains read-only until the user applies a result. |
| Runtime feedback and audition | Native waveforms and main-output meters work; active voice counts, playheads, selector positions and isolated-source audition are unavailable. | Add bounded generation-tagged capture and subscriptions, plus explicit audition/session lifetime. Reuse existing analysis/meter infrastructure where its contract fits. |
| Resources and persistence | Mono/stereo WAV selection is staged under the authoring root. Native relinking, portable patch saving and additional decoders are not wired. | Define owned-resource manifests, save/reload and relink transactions; preserve originals and keep codec admission consistent with Rust preparation. |

## Delivery order

1. Finish sample playback semantics and layers, then map each supported editor
   operation to the accepted sample-map/voice graph. This is the first engine
   increment because it unblocks the Sample and Layers workspaces.
2. Compile voice chains, effects, modulation and routing from existing modules.
   Start with Sample → Filter → Amplifier and one LFO-to-cutoff route. Add a new
   primitive only after the composition fails a concrete requirement and meets
   the repository's primitive admission rules.
3. Add offline analysis, normalization and immutable history rendering, followed
   by reliable resource persistence and relinking.
4. Add voice/playhead/selector feedback and isolated audition. Each earlier
   increment should also expose the minimum real feedback its controls need.

For example, the first filter increment must make the React Cutoff control
address a declared public control on the compiled filter, survive undo/reload,
and demonstrably change a representative rendered signal.

## Test boundaries

- Test every React control's ID, conversion, gesture and invoked adapter method
  using a recording bridge. A registry-driven inventory must fail when a control
  is omitted or targets the wrong parameter.
- Test generated graph/API contracts, including invalid values, stable IDs,
  preparation failure, undo and retired generations. Code generation reduces
  duplication; it does not establish that the host calls the right DSP input.
- Keep one real WebView/API-to-DSP proof per distinct command family. The existing
  representative knob proof checks signed sampler output; do not repeat that
  host test for every knob sharing the same command path.
- Test new or changed DSP in focused Rust unit tests, asserting known signed
  output and lifecycle semantics. Reuse established DSP evidence for unchanged
  primitives. Keep a small native rendering/embedded-runtime lane.

## Pending implementation tasks

- [ ] Inventory each rejected operation and choose its existing owner or an explicit new capability; turn each delivery increment into a concrete OpenSpec change before implementation.
- [ ] Specify and implement sample modes, overlapping layers and selector policies with rendered Rust behavior tests.
- [ ] Implement voice/FX/modulation/routing compilation, exhaustive control bindings and representative native API proofs.
- [ ] Specify missing analysis algorithms and implement cancellable jobs plus immutable history rendering.
- [ ] Implement save/reload/relink resource ownership and decoder admission.
- [ ] Implement bounded runtime telemetry and isolated audition with lifetime tests.

Reuse the existing [renderer-independent UI plan](../../openspec/changes/add-renderer-independent-plugin-ui/tasks.md)
for shared telemetry/authoring boundaries and
[workstation sampling plan](../../openspec/changes/add-workstation-sampling/tasks.md)
for overlapping velocity layers. The current
[WebView integration](../../openspec/changes/wire-trigger-native-webview/tasks.md)
does not mark these future capabilities complete.
