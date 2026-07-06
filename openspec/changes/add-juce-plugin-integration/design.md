## Context

Dandrum already separates the Rust engine from host/frontend concerns. The existing JUCE console wrapper links the Rust static library and calls the C FFI from a JUCE audio callback. The plugin integration should reuse that direction but introduce a DAW-appropriate `AudioProcessor` boundary, stable state handling, sample-accurate MIDI, a generic JUCE editor, and an explicit companion YAML authoring/reload flow.

The key product decision is that the DAW plugin is a performance/runtime surface during normal use. Instrument definitions are authored as YAML and loaded into the plugin as immutable compiled instruments. The plugin may launch a simple YAML text editor for authoring the current instrument definition, but saving from that editor is a deliberate instrument replacement operation, not live mutation of the running graph.

## Architecture

```text
Authoring CLI / plugin-launched YAML editor / future standalone editor
    |
    | YAML instrument + samples + preset surface
    v
Validation / preparation
    |
    v
Dandrum Plugin Instance
    |
    +-- JUCE AudioProcessor
    |     - host lifecycle
    |     - processBlock
    |     - MIDI buffer decoding
    |     - parameter bridge
    |     - state save/load
    |     - mute/stop/replace orchestration for editor saves
    |
    +-- JUCE Editor
    |     - generic knobs/sliders from preset surface
    |     - preset selection
    |     - load/reload action
    |     - launch YAML editor action
    |     - status/error display
    |
    +-- YAML Editor Surface
    |     - plain YAML text editing
    |     - schema/validation feedback
    |     - DSP graph preview from YAML using Mermaid or better graph renderer
    |     - save/apply action
    |
    +-- Rust Engine via C FFI
          - immutable loaded instrument definition
          - mutable public parameter state
          - prepared instrument runtime
          - realtime graph processor
          - bounded event queues
          - stereo rendering
```

## Decisions

### Instrument definitions are immutable while running

A loaded YAML instrument definition establishes the DSP graph, module routing, asset references, public preset surface, parameter IDs, labels, ranges, defaults, and mappings. Once loaded into a plugin instance and published as the active DSP, that definition is treated as immutable.

Changing the YAML instrument definition requires one of these explicit flows:

1. remove/recreate the plugin instance;
2. use a deliberate load/reload action; or
3. launch the YAML editor, edit the definition, and save/apply it.

All definition-changing flows prepare a complete replacement runtime off the audio thread. The audio callback must never parse YAML, compile graphs, load samples, or mutate the active graph.

Rationale: DAW hosts expect automation targets and saved state to remain stable until an explicit replacement. Dynamic graph and parameter-layout mutation during rendering would create host-compatibility, project-recall, and realtime-safety problems.

### Public parameter values are mutable runtime state

The YAML instrument definition declares the public parameter surface, but it does not own the current runtime values after the instrument is loaded. Each declared public parameter has a stable identity, type, default, range, and `maps_to` binding. The loaded plugin instance owns the current value for each declared public parameter.

Parameter changes from JUCE controls, DAW automation, plugin state restore, or compatible presets may update these runtime values without changing the loaded YAML document and without changing the parameter layout.

The Rust engine should represent this split explicitly:

```rust
pub struct DandrumEngine {
    sample_rate: f32,
    prepared_max_block_size: usize,
    fallback: FallbackSynth,
    loaded_instrument: Option<LoadedInstrument>,
    runtime: Option<RealtimeGraphProcessor>,
}

pub struct LoadedInstrument {
    definition: InstrumentDefinition,
    parameters: InstrumentParameterState,
}

pub struct InstrumentDefinition {
    patch_doc: PatchDocument,
    public_parameters: Vec<PublicParameterDescriptor>,
    public_parameter_bindings: Vec<PublicParameterBinding>,
    sampler_assets: PreparedSamplerAssets,
}

pub struct InstrumentParameterState {
    values_by_id: BTreeMap<String, ParameterValue>,
}
```

Rationale: this keeps instrument structure immutable while allowing the plugin to behave like a normal synth with tweakable knobs.

### The plugin can launch a simple YAML editor

The plugin editor should include an action to open a simple YAML editor for the current instrument definition. The editor is intentionally minimal:

- plain YAML text area;
- validation/schema diagnostics;
- DSP graph visualisation generated from the YAML;
- save/apply action;
- cancel/close action.

The graph visualisation may use Mermaid initially if it is the fastest practical option. If another renderer is better suited to interactive DSP graphs, it can replace Mermaid without changing the authoring contract.

The YAML editor is allowed to change graph structure, module routing, assets, public parameter declarations, and mappings. Those changes do not affect the running DSP until the user saves/applies and the replacement compile succeeds.

Rationale: this gives fast authoring from the plugin workflow while preserving realtime safety and host parameter stability during normal playback.

### YAML editor save is a mute/stop/compile/start operation

Saving from the YAML editor is an explicit instrument replacement transaction:

1. keep the current DSP running while the draft YAML is edited;
2. on save/apply, request plugin mute/suspend for the current instance;
3. stop accepting new MIDI/render work for the old DSP instance;
4. validate and compile the new YAML off the audio thread;
5. prepare assets and realtime buffers off the audio thread;
6. reconcile existing preset/current parameter values against the new `preset_surface.parameters`;
7. publish the replacement DSP/runtime;
8. rebuild or refresh the plugin parameter/control surface if the public parameter layout changed;
9. unmute/resume audio;
10. if compile/load fails, keep or restore the previous DSP and show diagnostics.

The old DSP must not be destroyed while `processBlock` can still access it. Replacement must use a safe handoff strategy such as an atomic/shared ownership swap or a host-thread-coordinated suspension boundary.

Rationale: a failed YAML edit must not leave the plugin silent or corrupted, and the audio thread must not wait on YAML compilation.

### Preset reconciliation after instrument reload

When a saved YAML edit produces a new instrument definition, existing presets/current parameter values should be reconciled by public parameter ID:

- parameters that still exist keep their current/preset value if the type is compatible;
- values should be clamped to the new min/max where applicable;
- parameters removed from the new definition are dropped from the active runtime state;
- parameters added by the new definition are initialised from their YAML-declared default values;
- incompatible presets are retained as unavailable or reported clearly rather than silently applied incorrectly.

This allows existing presets to remain useful as the instrument evolves while ensuring new parameters are recorded with default values.

### Instrument authoring stays out of `processBlock`

The plugin may launch an authoring editor, but the audio callback is never an authoring surface. YAML editing, graph visualisation, validation, compilation, asset loading, and replacement preparation must happen on editor/background/plugin-management threads, never in `processBlock`.

### The plugin UI uses JUCE native controls

The v1 plugin editor uses JUCE widgets for the normal plugin surface. It does not embed a browser/web UI for the main runtime panel.

The editor displays:

- instrument name/identity;
- current preset, if presets are available;
- one generic control per declared public parameter;
- output/gain control if exposed by the instrument surface;
- load/reload instrument action;
- launch YAML editor action;
- status/error text;
- optional diagnostic counters such as dropped MIDI events.

The YAML editor surface may be implemented with JUCE text controls and a rendered graph preview. A web view should only be introduced if it is justified by the graph rendering choice.

### Controls are generated from `preset_surface.parameters`

The loaded instrument declares how many public controls it needs. The plugin creates a generic knob/slider/control for each declared parameter.

Each parameter should have a stable ID from the YAML surface. Display metadata may include label, type, default, min/max, unit, and display formatting.

Example:

```yaml
preset_surface:
  parameters:
    - name: tune
      type: number
      default: 0.5
      min: 0.0
      max: 1.0
      maps_to: osc.frequency
    - name: decay
      type: number
      default: 0.35
      min: 0.0
      max: 1.0
      maps_to: amp_env.decay
```

The plugin may display these as knobs labelled `Tune` and `Decay`, but the underlying parameter identity is the declared stable `name` (`tune`, `decay`).

Rationale: fixed macro counts are unnecessarily limiting. The author should decide how many public controls the instrument requires, while the loaded plugin instance keeps that surface stable until explicit reload/replacement.

### Parameter layout is stable between explicit reloads

The plugin parameter layout is created from the loaded instrument surface and remains stable while that instrument is loaded. Live presets and automation may change values, but not the existence, ID, type, order, range, default, label, or mapping of parameters.

If a YAML editor save or explicit reload changes the parameter surface, that is treated as a full instrument replacement. The plugin must refresh the APVTS/control layout as allowed by the host format, or require plugin recreation where the host cannot safely accept layout changes.

Rationale: this keeps automation and state recall coherent during normal playback while still allowing explicit instrument development workflows.

### Public parameter changes update values, not YAML

When a public parameter changes, the engine must not modify, serialize, or rewrite the loaded YAML document. It updates only `InstrumentParameterState` and the prepared runtime parameter target associated with the immutable `maps_to` binding.

The first implementation may apply changes at block boundaries. Sample-accurate public parameter automation is a later extension that can use timestamped parameter events.

A public parameter binding should resolve once during instrument preparation:

```rust
pub struct PublicParameterBinding {
    public_id: String,
    target_module_id: String,
    target_parameter_name: String,
    prepared_index: Option<usize>,
}
```

Then runtime updates should use the prepared binding/index rather than reparsing YAML or doing expensive string lookup in `processBlock`.

### Presets change values, not structure

Presets may be loaded live if they are compatible with the current instrument definition. A compatible preset can only set public parameter values and public asset choices declared by the loaded instrument surface.

Presets must not declare graph, routing, render, scheduling, script, feedback, or arbitrary module structure.

Rationale: presets are safe live performance state; instruments are structural state.

### Engine replacement is prepared off the audio thread

When loading/reloading an instrument from the plugin UI, YAML editor save, or project state:

1. create a new Rust engine/runtime candidate;
2. prepare it with the current sample rate and max block size;
3. load/validate/compile the instrument and assets off the audio thread;
4. initialise immutable definition metadata and mutable parameter values;
5. reconcile carried-over current/preset values against the new public parameter surface;
6. mute/suspend the active audio path;
7. publish the prepared replacement to the audio thread;
8. safely retire the previous engine after it can no longer be used by the callback;
9. unmute/resume audio.

The audio thread must not acquire the patch-loading lock or wait for loading to complete.

### MIDI handoff is sample-accurate

The plugin processes JUCE `MidiBuffer` entries in `processBlock` and forwards note events with their sample offset into Rust.

The existing immediate note API is insufficient for plugin timing because it collapses all notes to the start of the block. Add FFI methods such as:

```c
void dandrum_engine_note_on_at(
    DandrumEngine* engine,
    unsigned char note,
    unsigned char velocity,
    size_t frame_offset);

void dandrum_engine_note_off_at(
    DandrumEngine* engine,
    unsigned char note,
    size_t frame_offset);
```

Rust should store these as bounded pending `BlockEvent` values for the next render call.

### Plugin state stores enough to restore without absolute paths

Plugin state should preserve:

- schema version;
- loaded instrument identity;
- embedded instrument YAML or bundled instrument ID;
- optional original path as a hint only;
- compatible preset identity/content if applicable;
- current public parameter values;
- asset references or embedded/bundled asset identifiers as needed.

Absolute file paths alone are not sufficient for project recall.

On restore, the immutable instrument definition is restored/reloaded first, then saved public parameter values are applied to mutable runtime parameter state.

### Realtime callback contract remains strict

`processBlock` may:

- read already-prepared parameters;
- forward MIDI events into bounded queues;
- call Rust render;
- clear unused output channels;
- render silence while an explicit replacement transaction is muting the plugin.

`processBlock` must not:

- allocate;
- lock;
- perform file I/O;
- parse YAML;
- load samples;
- compile graphs;
- log to console;
- create/destroy engines.

## FFI Surface

The plugin integration should extend the C FFI with explicit host-facing operations:

- create/destroy engine;
- prepare realtime with sample rate and maximum block size;
- load prepared instrument from path or memory off the audio thread;
- validate/compile instrument YAML from memory for editor-save flows;
- render stereo block;
- submit sample-accurate MIDI events;
- set public parameter value by stable prepared index or ID;
- query declared parameter metadata after load;
- query current public parameter values where needed for state/debugging;
- query graph visualisation metadata or generated graph text for editor previews where practical;
- query last error/status off the audio thread;
- query diagnostics such as dropped event counts.

String-heavy operations, YAML text transfer, graph preview generation, and metadata queries are editor/background-thread operations only. The audio callback should use prepared parameter handles, indices, or lock-free event queues rather than reparsing string IDs.

## Reference Review Notes

Reviewed `https://github.com/nberr/juce-template` (task 0). It is a Projucer-based template, not CMake-based, so its build structure is not reused directly — Dandrum keeps its existing CMake + `add_subdirectory(third_party/JUCE)` approach, consistent with the `dandrum-drum-machine-demo` console app target.

Adopted pattern:
- A generic `Parameter*` control wrapper (e.g. `ParameterSlider`) that binds one JUCE control to one stable `AudioProcessorValueTreeState` parameter ID. This matches the plan to generate one control per declared `preset_surface.parameters` entry (section 4).

Explicitly not adopted (out of Dandrum v1 scope per proposal "Out of Scope"):
- `react-juce` embedded web UI for the main runtime surface.
- `Assets/Registration` / license-server / key-generator flows.
- Preset "overlay" screens tied to a hosted preset-sharing service.
- Installer packaging scripts and Max/Python prototype tooling.
- The template's own bundled Rust DSP crate layout — Dandrum already has its own `src/rust-engine` crate and C FFI (`RustEngineBindings.h`), which stays as the integration boundary.

## Open Questions

- Should the initial plugin require selecting an instrument before the `AudioProcessorValueTreeState` layout is created, or should v1 ship with a bundled default instrument to establish an initial surface?
- Should parameter IDs be YAML keys directly, or normalized to a host-safe form with a stable hash/alias?
- Should reloading an instrument with the exact same parameter surface preserve existing automation/value state automatically?
- Should block-boundary public parameter changes be sufficient for v1, or should sample-accurate parameter automation be part of the first release?
- Should Mermaid be good enough for the first DSP graph preview, or should the editor use a graph renderer better suited to ports, signal types, and grouped modules?
- Should the YAML editor be embedded in the plugin window, a separate JUCE document window, or an external companion app launched by the plugin?
- How should hosts that do not tolerate dynamic parameter-layout changes handle YAML editor saves that add/remove public parameters?
- Should asset references be embedded in plugin state, copied to a project-local cache, or resolved through bundled instrument packages?
