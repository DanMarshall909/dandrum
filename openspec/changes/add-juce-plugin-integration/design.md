## Context

Dandrum already separates the Rust engine from host/frontend concerns. The existing JUCE console wrapper links the Rust static library and calls the C FFI from a JUCE audio callback. The plugin integration should reuse that direction but introduce a DAW-appropriate `AudioProcessor` boundary, stable state handling, sample-accurate MIDI, and a generic JUCE editor.

The key product decision is that the DAW plugin is a performance/runtime surface, not an authoring surface. Instrument definitions are authored externally and loaded into the plugin as immutable compiled instruments.

## Architecture

```text
Authoring CLI / future standalone editor
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
    |
    +-- JUCE Editor
    |     - generic knobs/sliders from preset surface
    |     - preset selection
    |     - load/reload action
    |     - status/error display
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

### Instrument definitions are immutable in the plugin

A loaded YAML instrument definition establishes the DSP graph, module routing, asset references, public preset surface, parameter IDs, labels, ranges, defaults, and mappings. Once loaded into a plugin instance, that definition is treated as immutable.

Changing the YAML instrument definition requires one of these explicit flows:

1. remove/recreate the plugin instance; or
2. use a deliberate load/reload action that prepares a complete replacement engine off the audio thread and swaps it safely.

The audio callback must never parse YAML, compile graphs, load samples, or mutate the active graph.

Rationale: DAW hosts expect automation targets and saved state to remain stable. Dynamic graph and parameter-layout mutation would create host-compatibility, project-recall, and realtime-safety problems.

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

### Instrument authoring stays outside the plugin

The plugin editor is not a graph editor. Authoring and validation of YAML instruments belongs in:

- the existing CLI;
- offline validation/render tests; and
- a future standalone Dandrum Studio/editor if needed.

Rationale: DAW plugin UX should stay focused on playing, automating, and recalling instruments, not building them.

### The plugin UI uses JUCE native controls

The v1 plugin editor uses JUCE widgets only. It does not embed a browser/web UI.

The editor displays:

- instrument name/identity;
- current preset, if presets are available;
- one generic control per declared public parameter;
- output/gain control if exposed by the instrument surface;
- load/reload instrument action;
- status/error text;
- optional diagnostic counters such as dropped MIDI events.

Rationale: JUCE-native controls are simpler, portable across AU/VST3 hosts, and sufficient for the v1 runtime UI.

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

Rationale: fixed macro counts are unnecessarily limiting. The author should decide how many public controls the instrument requires, while the loaded plugin instance keeps that surface stable.

### Parameter layout is stable per loaded instrument instance

The plugin parameter layout is created from the loaded instrument surface and remains stable while that instrument is loaded. Live presets and automation may change values, but not the existence, ID, type, order, range, default, label, or mapping of parameters.

If a different instrument definition has a different parameter surface, it requires plugin recreation or explicit replacement. Replacement must be treated as a full instrument load, not an in-place graph edit.

Rationale: this keeps automation and state recall coherent for the lifetime of a plugin instance.

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

When loading/reloading an instrument from the plugin UI or project state:

1. create a new Rust engine instance;
2. prepare it with the current sample rate and max block size;
3. load/validate/compile the instrument and assets off the audio thread;
4. initialise immutable definition metadata and mutable parameter values;
5. atomically publish the prepared replacement to the audio thread;
6. safely retire the previous engine after it can no longer be used by the callback.

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
- clear unused output channels.

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
- render stereo block;
- submit sample-accurate MIDI events;
- set public parameter value by stable prepared index or ID;
- query declared parameter metadata after load;
- query current public parameter values where needed for state/debugging;
- query last error/status off the audio thread;
- query diagnostics such as dropped event counts.

String-heavy operations and metadata queries are editor/background-thread operations only. The audio callback should use prepared parameter handles, indices, or lock-free event queues rather than reparsing string IDs.

## Reference Review Notes

Reviewed `https://github.com/nberr/juce-template` (task 0). It is a Projucer-based template, not CMake-based, so its build structure is not reused directly — Dandrum keeps its existing CMake + `add_subdirectory(third_party/JUCE)` approach, consistent with the `dandrum-drum-machine-demo` console app target.

Adopted pattern:
- A generic `Parameter*` control wrapper (e.g. `ParameterSlider`) that binds one JUCE control to one stable `AudioProcessorValueTreeState` parameter ID. This matches the plan to generate one control per declared `preset_surface.parameters` entry (section 4).

Explicitly not adopted (out of Dandrum v1 scope per proposal "Out of Scope"):
- `react-juce` embedded web UI.
- `Assets/Registration` / license-server / key-generator flows.
- Preset "overlay" screens tied to a hosted preset-sharing service.
- Installer packaging scripts and Max/Python prototype tooling.
- The template's own bundled Rust DSP crate layout — Dandrum already has its own `src/rust-engine` crate and C FFI (`RustEngineBindings.h`), which stays as the integration boundary.

## Open Questions

- Should the initial plugin require selecting an instrument before the `AudioProcessorValueTreeState` layout is created, or should v1 ship with a bundled default instrument to establish an initial surface?
- Should parameter IDs be YAML keys directly, or normalized to a host-safe form with a stable hash/alias?
- Should reloading an instrument with the exact same parameter surface preserve existing automation/value state automatically?
- Should block-boundary public parameter changes be sufficient for v1, or should sample-accurate parameter automation be part of the first release?
- Should asset references be embedded in plugin state, copied to a project-local cache, or resolved through bundled instrument packages?
