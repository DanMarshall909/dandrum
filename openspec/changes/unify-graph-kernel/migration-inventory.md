# Unify-Graph-Kernel: Pre-Refactor Inventory

Generated: read-only sweep of Rust engine source (`src/rust-engine/src/`) and
example YAML files (`examples/patches/`, `examples/presets/`).

---

## 1. Builtin Catalogue

Every DSP node type registered in `BuiltInModuleRegistry::new()` (`builtins.rs:268`).
Signal-type abbreviations: **A** = Audio, **C** = Control, **E** = Event.
Mix = multi-source mixing input.

| # | Builtin name | Source:line | Input ports (name, type, channels) | Output ports (name, type, channels) | Tunable params (name, type, default, range/enum) |
|---|---|---|---|---|---|
| 1 | `midi_input` | `builtins.rs:327` | — | `events` (E) | — |
| 2 | `audio_output` | `builtins.rs:331` | `left` (A), `right` (A) — **explicit stereo pair** | — | — |
| 3 | `oscillator` | `builtins.rs:335` | `pitch` (C) | `audio` (A) | `pitch` (Number, default `1`, range 0–64); `waveform` (Text, default `saw`, enum: saw/sine/triangle/square) |
| 4 | `gain` | `builtins.rs:366` | `audio_in` (A), `gain` (C) | `audio_out` (A) | `gain` (Number, default `1`, range 0–4) |
| 5 | `audio_mixer` | `builtins.rs:380` | `inputs` (A, Mix) | `mix` (A) | — |
| 6 | `control_mixer` | `builtins.rs:386` | `inputs` (C, Mix) | `sum` (C) | — |
| 7 | `adsr` | `builtins.rs:392` | `gate` (E), `attack` (C), `decay` (C), `sustain` (C), `release` (C) | `value` (C) | `attack` (Number, default `5`, range 0–500); `decay` (Number, default `30`, range 1–5000); `sustain` (Number, default `0.7`, range 0–1); `release` (Number, default `200`, range 0–10000) |
| 8 | `lfo` | `builtins.rs:429` | `rate` (C) | `value` (C) | — |
| 9 | `filter` | `builtins.rs:435` | `audio_in` (A), `cutoff` (C), `resonance` (C), `gain` (C) | `audio_out` (A) | `algorithm` (Text, default `moog`, enum: moog/biquad/comb); `mode` (Text, default `lowpass`, enum: lowpass/highpass/peaking); `comb_type` (Text, default `feedback`, enum: feedback/feedforward) |
| 10 | `audio_delay_one_sample` | `builtins.rs:465` | `audio_in` (A) | `audio_out` (A) | — (feedback boundary: Audio) |
| 11 | `block_delay` | `builtins.rs:472` | `audio_in` (A) | `audio_out` (A) | — (feedback boundary: Audio) |
| 12 | `control_delay` | `builtins.rs:479` | `value` (C) | `value` (C) | — (feedback boundary: Control) |
| 13 | `script` | `builtins.rs:486` | — (user-declared in YAML) | — (user-declared in YAML) | `language` (Text, default `rhai`, enum: [rhai]); `source` (Text) |
| 14 | `sampler` | `builtins.rs:502` | `trigger` (E), `rate` (C), `start` (C), `loop_enabled` (C), `loop_start` (C), `loop_end` (C) | `audio` (A) | `asset` (Text) |
| 15 | `note_to_rate` | `builtins.rs:521` | `events` (E) | `rate` (C) | — |
| 16 | `event_filter` | `builtins.rs:528` | `events_in` (E) | `events_out` (E) | `selector` (Text, default `note`, enum: [note]); `note` (Integer, range 0–127) |
| 17 | `dynamics-processor` | `builtins.rs:550` | `audio_in` (A), `sidechain_in` (C), `threshold` (C), `below_ratio` (C), `above_ratio` (C), `attack` (C), `release` (C), `knee` (C), `makeup_gain` (C), `attack_gain` (C), `sustain_gain` (C) | `audio_out` (A) | `mode` (Text, default `level`, enum: level/transient); `detection` (Text, default `peak`, enum: peak/rms); `topology` (Text, default `feedforward`, enum: feedforward/feedback) |
| 18 | `saturator` | `builtins.rs:586` | `audio_in` (A), `drive` (C), `bias` (C), `curve_select` (C) | `audio_out` (A) | — |
| 19 | `convolution` | `builtins.rs:597` | `audio_in` (A), `mix` (C) | `audio_out` (A) | — |
| 20 | `frequency_splitter` | `builtins.rs:603` | `audio_in` (A), `crossover_hz` (C) | `low` (A), `mid` (A), `high` (A) — **3 static mono outs** | — |
| 21 | `spectral_processor` | `builtins.rs:611` | `audio_in` (A), `threshold` (C), `mix` (C) | `audio_out` (A) | `mode` (Text, default `gate`, enum: gate/passthrough); `fft_size` (Number, default `2048`, range 256–8192); `window` (Text, default `hann`, enum: [hann]); `threshold` (Number, default `-40`, range –100–0); `mix` (Number, default `1.0`, range 0–1) |
| 22 | `echo` | `builtins.rs:653` | `audio_in_l` (A), `audio_in_r` (A) — **stereo pair**; `time_left_ms` (C), `time_right_ms` (C), `feedback` (C), `damping_cutoff` (C), `wet` (C), `dry` (C), `sync_division` (C), `ping_pong` (C) | `audio_out_l` (A), `audio_out_r` (A) — **stereo pair** | `interpolation` (Text, default `linear`, enum: linear/cubic) |
| 23 | `reverb` | `builtins.rs:819` | `audio_in_l` (A), `audio_in_r` (A) — **stereo pair**; `decay_time` (C), `room_size` (C), `pre_delay` (C), `damping` (C), `diffusion` (C), `stereo_width` (C), `wet` (C), `dry` (C) | `audio_out_l` (A), `audio_out_r` (A) — **stereo pair** | `interpolation` (Text, default `linear`, enum: linear/cubic) |
| 24 | `noise` | `builtins.rs:678` | — | `audio` (A) | `seed` (Number, default `0`) |
| 25 | `impulse` | `builtins.rs:689` | `trigger` (E) | `audio` (A) | — |
| 26 | `multiply` | `builtins.rs:696` | `audio_in` (A), `gain` (A) | `audio_out` (A) | — |
| 27 | `note_to_control` | `builtins.rs:706` | `events` (E) | `frequency` (C), `pitch_ratio` (C), `gate` (E), `velocity` (C) | — |
| 28 | `envelope_follower` | `builtins.rs:716` | `audio_in` (A), `attack` (C), `release` (C), `amount` (C), `offset` (C), `invert` (C) | `value` (C) | `mode` (Text, default `peak`, enum: peak/rms) |
| 29 | `curve_mapper` | `builtins.rs:737` | `value` (C), `amount` (C), `bias` (C), `scale` (C), `offset` (C) | `value` (C) | `curve` (Text, default `linear`, enum: 7 values); `steps` (Integer, default `4`, range 2–128); `amount` (Number, default `1`, range 0–1); `bias` (Number, default `0`, range –1–1); `scale` (Number, default `1`, range –64–64); `offset` (Number, default `0`, range –64–64) |
| 30 | `decay` | `builtins.rs:797` | `trigger` (E), `time_ms` (C) | `value` (C) | `time_ms` (Number, default `100`, range 1–5000); `curve` (Text, default `exponential`, enum: linear/exponential) |

**Notes:**
- `lfo` has **no parameters** in its definition block (no waveform/rate params exposed to metadata yet).
- `script` has zero standard ports; user declares them in YAML.
- `multiply`'s second input is named `gain` but is `Audio` type (not Control).
- `noise` has no inputs.
- Feedback boundaries are declared only on: `audio_delay_one_sample`, `block_delay` (Audio), `control_delay` (Control).

---

## 2. `_l` / `_r` Stereo Port Pairs

### Builtin definitions

| File:line | Builtin | Port pair |
|---|---|---|
| `builtins.rs:655-656` | echo (input) | `audio_in_l` / `audio_in_r` |
| `builtins.rs:657` | echo (output) | `audio_out_l` / `audio_out_r` |
| `builtins.rs:821-822` | reverb (input) | `audio_in_l` / `audio_in_r` |
| `builtins.rs:833` | reverb (output) | `audio_out_l` / `audio_out_r` |

The `audio_output` builtin uses named ports `left` / `right` (not `_l`/`_r` suffix),
so it is **not** listed above, but it is semantically a stereo pair. See `builtins.rs:332`.

### Port-name constants in `graph/builtin_ports.rs`

| File:line | Constant | Value |
|---|---|---|
| `graph/builtin_ports.rs:3` | `AUDIO_IN_L` | `"audio_in_l"` |
| `graph/builtin_ports.rs:4` | `AUDIO_IN_R` | `"audio_in_r"` |
| `graph/builtin_ports.rs:6` | `AUDIO_OUT_L` | `"audio_out_l"` |
| `graph/builtin_ports.rs:7` | `AUDIO_OUT_R` | `"audio_out_r"` |

### Example YAML usage (non-builtin-internal)

| File:line | Patch | Port pair |
|---|---|---|
| `examples/patches/module-echo.yaml:11,15` | module-echo (composite def input) | `audio_in_l` / `audio_in_r` |
| `examples/patches/module-echo.yaml:44,48` | module-echo (composite def output) | `audio_out_l` / `audio_out_r` |
| `examples/patches/module-echo.yaml:87,89` | module-echo (connection) | `delay.audio_in_l` / `delay.audio_in_r` |
| `examples/patches/module-echo.yaml:90,92` | module-echo (connection) | `delay.audio_out_l` / `delay.audio_out_r` |
| `examples/patches/reverb-demo.yaml:59,61` | reverb-demo (composite def input) | `audio_in_l` / `audio_in_r` |
| `examples/patches/reverb-demo.yaml:64,66` | reverb-demo (composite def output) | `audio_out_l` / `audio_out_r` |
| `examples/patches/reverb-demo.yaml:89,91` | reverb-demo (connection) | `reverb.audio_in_l` / `reverb.audio_in_r` |
| `examples/patches/reverb-demo.yaml:92,94` | reverb-demo (connection) | `reverb.audio_out_l` / `reverb.audio_out_r` |
| `examples/patches/module-reverb.yaml:11,15` | module-reverb (composite def input) | `audio_in_l` / `audio_in_r` |
| `examples/patches/module-reverb.yaml:44,48` | module-reverb (composite def output) | `audio_out_l` / `audio_out_r` |
| `examples/patches/module-reverb.yaml:87,89` | module-reverb (connection) | `reverb_mod.audio_in_l` / `reverb_mod.audio_in_r` |
| `examples/patches/module-reverb.yaml:90,92` | module-reverb (connection) | `reverb_mod.audio_out_l` / `reverb_mod.audio_out_r` |
| `examples/patches/echo-demo.yaml:59,61` | echo-demo (composite def input) | `audio_in_l` / `audio_in_r` |
| `examples/patches/echo-demo.yaml:64,66` | echo-demo (composite def output) | `audio_out_l` / `audio_out_r` |
| `examples/patches/echo-demo.yaml:89,91` | echo-demo (connection) | `echo.audio_in_l` / `echo.audio_in_r` |
| `examples/patches/echo-demo.yaml:92,94` | echo-demo (connection) | `echo.audio_out_l` / `echo.audio_out_r` |
| `examples/patches/polyphonic-pad.yaml:67,69` | polyphonic-pad (composite def input) | `audio_in_l` / `audio_in_r` |
| `examples/patches/polyphonic-pad.yaml:72,74` | polyphonic-pad (composite def output) | `audio_out_l` / `audio_out_r` |
| `examples/patches/polyphonic-pad.yaml:97,99` | polyphonic-pad (connection) | `reverb.audio_in_l` / `reverb.audio_in_r` |
| `examples/patches/polyphonic-pad.yaml:100,102` | polyphonic-pad (connection) | `reverb.audio_out_l` / `reverb.audio_out_r` |

### Internal Rust processing code

| File:line | Context | Pair |
|---|---|---|
| `echo.rs:122,150` | `Echo::process(in_l, in_r) -> (out_l, out_r)` | Hardcoded L/R |
| `echo.rs:5-10` | `Echo` struct fields `delay_l/r`, `damp_l/r`, `delay_ms_l/r` | Hardcoded L/R |
| `reverb.rs:279,306` | `Reverb::process(in_l, in_r) -> (out_l, out_r)` | Hardcoded L/R |
| `reverb.rs:91-97` | `Reverb` struct fields `pre_delay_l/r`, `combs_l/r`, `diffusers_l/r` | Hardcoded L/R |
| `graph_processor/dispatch.rs:149-158` | Echo dispatch — `audio(AUDIO_IN_L)`, `audio(AUDIO_IN_R)` | Builtin ports |
| `graph_processor/dispatch.rs:172-184` | Reverb dispatch — `audio(AUDIO_IN_L)`, `audio(AUDIO_IN_R)` | Builtin ports |
| `graph_processor/processing.rs:528-567` | `process_echo(...)` — `audio_in_l/r`, `out_l/r` | Hardcoded |
| `graph_processor/processing.rs:572-622` | `process_reverb(...)` — `audio_in_l/r`, `out_l/r` | Hardcoded |

---

## 3. `audio_output` References

Organised by kind.

### Definition
| File:line | Context | Kind |
|---|---|---|
| `builtins/module_types.rs:2` | `pub const AUDIO_OUTPUT: &str = "audio_output";` | Constant |
| `builtins.rs:270` | `audio_output_definition(),` | Registry registration |
| `builtins.rs:331-333` | `fn audio_output_definition()` | Definition function |

### Instantiations in Rust source (test/example code)
| File:line | Context | Kind |
|---|---|---|
| `graph_processor/tests.rs:46` | `ModuleNode::new(ModuleId::new("out"), "audio_output")` | Instantiation (test) |
| `graph_processor/tests.rs:1720` | same | Instantiation (test) |
| `graph_processor/tests.rs:1836` | same | Instantiation (test) |
| `graph_processor/tests.rs:1943` | same | Instantiation (test) |
| `graph_processor/tests.rs:2067` | same | Instantiation (test) |
| `graph_processor/tests.rs:2293` | same | Instantiation (test) |
| `graph_processor/tests.rs:2500` | same | Instantiation (test) |
| `graph_processor/tests.rs:2552` | same | Instantiation (test) |
| `graph_processor/tests.rs:2621` | same | Instantiation (test) |
| `graph_processor/tests.rs:2677` | same | Instantiation (test) |
| `graph_processor/tests.rs:2725` | same | Instantiation (test) |
| `graph_processor/tests.rs:2908` | same | Instantiation (test) |
| `graph_processor/tests.rs:3152` | same | Instantiation (test) |
| `graph_processor/tests.rs:3208` | same | Instantiation (test) |
| `graph_processor/tests.rs:3822` | same | Instantiation (test) |
| `graph_processor/tests.rs:4786` | same | Instantiation (test) |
| `graph_processor/tests.rs:4833` | same | Instantiation (test) |
| `graph_processor/tests.rs:4881` | same | Instantiation (test) |
| `graph_processor/tests.rs:4931` | same | Instantiation (test) |
| `graph_processor/tests.rs:5115` | same | Instantiation (test) |
| `graph/tests.rs:25` | same | Instantiation (test) |
| `graph/tests.rs:151` | same | Instantiation (test) |
| `graph/tests.rs:948` | same | Instantiation (test) |
| `graph/tests.rs:1059` | same | Instantiation (test) |
| `graph/tests.rs:1621` | same | Instantiation (test) |
| `graph/tests.rs:1653` | same | Instantiation (test) |
| `graph/tests.rs:1708` | same | Instantiation (test) |
| `bin/dandrum-stepseq.rs:377` | same | Instantiation (binary) |
| `bin/dandrum-demo.rs:62` | same | Instantiation (binary) |
| `module_package.rs:306` | YAML literal `type: audio_output` | Instantiation (embedded test YAML) |
| `module_package.rs:357` | same | Instantiation (embedded test YAML) |
| `module_package.rs:517` | same | Instantiation (embedded test YAML) |
| `module_package.rs:580` | same | Instantiation (test) lookup |
| `graph_module.rs:183` | `ordinary("out", "audio_output")` | Instantiation (test) |
| `preparation.rs:250` | YAML literal `type: audio_output` | Instantiation (embedded test YAML) |
| `preparation.rs:462` | same | Instantiation (embedded test YAML) |
| `preparation.rs:545` | same | Instantiation (embedded test YAML) |
| `synth.rs:512` | YAML literal `type: audio_output` | Instantiation (embedded test YAML) |
| `synth.rs:560` | same | Instantiation (embedded test YAML) |
| `compiled_patch.rs:479` | `ModuleNode::new(ModuleId::new(id), "audio_output")` | Instantiation (test) |
| `drum_voice_authoring_tests.rs:190` | YAML literal `type: audio_output` | Instantiation (embedded test YAML) |
| `core.rs:341` | YAML literal `type: audio_output` | Instantiation (embedded test YAML) |

### Inline YAML within Rust test strings (also instantiations)
| File:line | Context | Kind |
|---|---|---|
| `graph_processor/tests.rs:1036` | `type: audio_output` | YAML in test string |
| `graph_processor/tests.rs:1106` | same | YAML in test string |
| `graph_processor/tests.rs:1160` | same | YAML in test string |
| `graph_processor/tests.rs:1212` | same | YAML in test string |
| `graph_processor/tests.rs:1266` | same | YAML in test string |
| `graph_processor/tests.rs:2792` | same | YAML in test string |
| `graph_processor/tests.rs:2863` | same | YAML in test string |
| `graph_processor/tests.rs:2983` | same | YAML in test string |
| `graph_processor/tests.rs:3074` | same | YAML in test string |
| `graph_processor/tests.rs:4063` | same | YAML in test string |
| `graph_processor/tests.rs:4113` | same | YAML in test string |
| `graph_processor/tests.rs:4160` | same | YAML in test string |
| `graph_processor/tests.rs:4232` | same | YAML in test string |
| `graph_processor/tests.rs:4334` | same | YAML in test string |
| `graph_processor/tests.rs:4459` | same | YAML in test string |
| `graph_processor/tests.rs:4492` | same | YAML in test string |
| `graph_processor/tests.rs:4531` | same | YAML in test string |
| `graph_processor/tests.rs:4579` | same | YAML in test string |
| `graph_processor/tests.rs:4624` | same | YAML in test string |
| `graph_processor/tests.rs:4675` | same | YAML in test string |
| `graph_processor/tests.rs:4721` | same | YAML in test string |
| `graph_processor/tests.rs:4762` | same | YAML in test string |
| `graph/tests.rs:407` | same | YAML in test string |
| `graph/tests.rs:472` | same | YAML in test string |
| `graph/tests.rs:542` | same | YAML in test string |
| `graph/tests.rs:591` | same | YAML in test string |
| `graph/tests.rs:639` | same | YAML in test string |
| `graph/tests.rs:780` | same | YAML in test string |
| `graph/tests.rs:1411` | same | YAML in test string |
| `graph/tests.rs:1451` | same | YAML in test string |
| `graph/tests.rs:1487` | same | YAML in test string |
| `graph/tests.rs:1527` | same | YAML in test string |
| `graph/tests.rs:1567` | same | YAML in test string |
| `ffi.rs:796` | `type: audio_output\n` (string literal in FFI test) | YAML in test string |
| `ffi.rs:811` | same | YAML in test string |
| `render_plan.rs:435` | `ModuleNode::new(ModuleId::new("out"), "audio_output")` | Instantiation (test) |

### Example YAML files
| File:line | Patch file | Kind |
|---|---|---|
| `examples/patches/short-tune-with-delay.yaml:91` | example patch | Instantiation |
| `examples/patches/module-impulse-tone.yaml:50` | example patch | Instantiation |
| `examples/patches/module-echo.yaml:72` | example patch | Instantiation |
| `examples/patches/drum-kit.yaml:172` | example patch | Instantiation |
| `examples/patches/reverb-demo.yaml:69` | example patch | Instantiation |
| `examples/patches/delayed-feedback.yaml:26` | example patch | Instantiation |
| `examples/patches/envelope-ducking.yaml:38` | example patch | Instantiation |
| `examples/patches/module-drum-voice.yaml:40` | example patch | Instantiation |
| `examples/patches/synthetic-808-kick.yaml:204` | example patch | Instantiation |
| `examples/patches/echo-demo.yaml:69` | example patch | Instantiation |
| `examples/patches/module-reverb.yaml:72` | example patch | Instantiation |
| `examples/patches/control-mixer-modulation.yaml:43` | example patch | Instantiation |
| `examples/patches/polyphonic-chords.yaml:53` | example patch | Instantiation |
| `examples/patches/polyphonic-sampler-chords.yaml:61` | example patch | Instantiation |
| `examples/patches/synthetic-hats.yaml:29` | example patch | Instantiation |
| `examples/patches/minimal-sampler.yaml:23` | example patch | Instantiation |
| `examples/patches/module-impulse-layer.yaml:67` | example patch | Instantiation |
| `examples/patches/module-hidden-internals.yaml:44` | example patch | Instantiation |
| `examples/patches/minimal-tune.yaml:46` | example patch | Instantiation |
| `examples/patches/minimal-event-osc-vca.yaml:43` | example patch | Instantiation |
| `examples/patches/module-impulse-noise.yaml:57` | example patch | Instantiation |
| `examples/patches/event-routing-simple-poly-synth.yaml:30` | example patch | Instantiation |
| `examples/patches/synthetic-snare.yaml:27` | example patch | Instantiation |
| `examples/patches/event-routing-drum-machine.yaml:59` | example patch | Instantiation |
| `examples/patches/polyphonic-pad.yaml:77` | example patch | Instantiation |
| `examples/patches/envelope-filter-modulation.yaml:49` | example patch | Instantiation |
| `examples/patches/drums/drum-808-snare.yaml:89` | example patch | Instantiation |
| `examples/patches/drums/drum-909-tom.yaml:71` | example patch | Instantiation |
| `examples/patches/drums/drum-909-kick.yaml:95` | example patch | Instantiation |
| `examples/patches/module-velocity-vca.yaml:51` | example patch | Instantiation |
| `examples/patches/drums/drum-808-conga.yaml:70` | example patch | Instantiation |
| `examples/patches/drums/drum-909-ride.yaml:41` | example patch | Instantiation |
| `examples/patches/drums/drum-909-clap.yaml:80` | example patch | Instantiation |
| `examples/patches/drums/drum-909-hat-closed.yaml:43` | example patch | Instantiation |
| `examples/patches/drums/drum-909-snare.yaml:81` | example patch | Instantiation |
| `examples/patches/drums/drum-808-tom.yaml:71` | example patch | Instantiation |
| `examples/patches/drums/drum-909-hat-open.yaml:41` | example patch | Instantiation |
| `examples/patches/drums/drum-808-cowbell.yaml:87` | example patch | Instantiation |
| `examples/patches/drums/drum-909-crash.yaml:41` | example patch | Instantiation |
| `examples/patches/drums/drum-808-clap.yaml:80` | example patch | Instantiation |

### Processing / render-plan / compiler code
| File:line | Context | Kind |
|---|---|---|
| `compiled_patch.rs:19` | `audio_output_index: Option<usize>` | Compiler field |
| `compiled_patch.rs:225-228` | Index computed by `.position(\|module\| module.module_type() == "audio_output")` | Compiler logic |
| `compiled_patch.rs:261-262` | `pub fn audio_output_index() -> Option<usize>` | Accessor |
| `render_plan.rs:71` | `audio_output: Option<AudioOutputBinding>` | Render-plan field |
| `render_plan.rs:98` | `audio_output: None` | Default initialiser |
| `render_plan.rs:139-141` | `audio_output: compiled.audio_output_index().and_then(...)` | Builder |
| `render_plan.rs:299` | `fn audio_output_binding(&self, ...)` | Builder method |
| `graph_processor/block.rs:14` | `pub(super) fn collect_audio_output(...)` | Processor helper |
| `graph_processor/block.rs:101,239` | `collect_audio_output(all_outputs, ...)` | Processor call sites |
| `graph_processor/helpers.rs:23` | `pub(super) fn audio_output(port_name, audio) -> ModuleOutputs` | Helper |
| `graph_processor/helpers.rs:29` | `pub(super) fn stereo_audio_output(left, right) -> ModuleOutputs` | Helper |
| `graph_processor/offline.rs:24,124` | `compiled.audio_output_index()` | Offline renderer |
| `graph_processor/realtime_graph_processor.rs:114` | `compiled.audio_output_index()` | Realtime renderer |
| `graph_processor/realtime_graph_processor.rs:413,434` | `render_plan.audio_output` | Realtime renderer |
| `graph_processor/realtime_graph_processor.rs:550` | `collect_audio_output(...)` | Realtime renderer |
| `graph_processor/realtime_graph_processor.rs:836` | `render_plan.audio_output.is_none()` | Realtime renderer guard |
| `graph_processor/processing.rs:6-7` | `use ... { audio_output, stereo_audio_output }` | Import |
| `graph_processor/processing.rs:60` | `audio_output(builtin_ports::AUDIO, audio)` | Oscillator processing |
| `graph_processor/processing.rs:165` | `audio_output(builtin_ports::AUDIO_OUT, audio)` | Gain processing |
| `graph_processor/processing.rs:189,193` | `audio_output(builtin_ports::AUDIO, audio)` | Impulse / noise processing |
| `graph_processor/processing.rs:247` | `audio_output(builtin_ports::AUDIO, audio)` | Sampler processing |
| `graph_processor/processing.rs:443` | `audio_output(builtin_ports::AUDIO_OUT, audio_out)` | Filter processing |
| `graph_processor/processing.rs:471` | `audio_output(builtin_ports::AUDIO_OUT, audio_out)` | Saturator processing |
| `graph_processor/processing.rs:501` | `audio_output(builtin_ports::AUDIO_OUT, audio_out)` | Convolution processing |
| `graph_processor/processing.rs:523` | `audio_output(builtin_ports::AUDIO_OUT, audio_out)` | Spectral processor |
| `graph_processor/processing.rs:567` | `stereo_audio_output(out_l, out_r)` | Echo processing |
| `graph_processor/processing.rs:622` | `stereo_audio_output(out_l, out_r)` | Reverb processing |
| `graph_processor/processing.rs:686` | `audio_output(builtin_ports::AUDIO_OUT, audio_out)` | Dynamics processor |
| `graph_processor/processing.rs:708` | `audio_output(builtin_ports::AUDIO, audio)` | Multiply processing |
| `graph_processor/processing.rs:779` | `audio_output(builtin_ports::AUDIO, audio)` | Noise processing |
| `graph_processor/processing.rs:790` | `audio_output(builtin_ports::AUDIO_OUT, audio)` | Decay processing |
| `patch.rs:167` | `"audio_outputs"` | Patch parsing |
| `bin/dandrum-stepseq.rs:306` | `yaml.push_str("    type: audio_output\n");` | CLI codegen |

### Test assertions
| File:line | Context | Kind |
|---|---|---|
| `builtins/tests.rs:18-21` | `macro_rules! assert_audio_outputs!` | Test macro |
| `builtins/tests.rs:47-64` | `initialized_registry_contains_midi_input_and_audio_output_definitions` | Test |
| `builtins/tests.rs:75` | `assert_has_audio_output(oscillator, AUDIO)` | Test assertion |
| `builtins/tests.rs:82` | `assert_has_audio_output(gain, AUDIO_OUT)` | Test assertion |
| `builtins/tests.rs:88` | `assert_has_audio_output(audio_mixer, MIX)` | Test assertion |
| `builtins/tests.rs:108` | `assert_has_audio_output(filter, AUDIO_OUT)` | Test assertion |
| `builtins/tests.rs:119` | `assert_has_audio_output(one_sample_delay, AUDIO_OUT)` | Test assertion |
| `builtins/tests.rs:126` | `assert_has_audio_output(block_delay, AUDIO_OUT)` | Test assertion |
| `builtins/tests.rs:153-154` | `fn assert_has_audio_output(...)` | Test helper definition |
| `builtins/tests.rs:297` | `assert_has_audio_output(sampler, AUDIO)` | Test assertion |
| `builtins/tests.rs:404` | `assert_audio_outputs!(echo, AUDIO_OUT_L, AUDIO_OUT_R)` | Test assertion |
| `builtins/tests.rs:425` | `assert_audio_outputs!(reverb, AUDIO_OUT_L, AUDIO_OUT_R)` | Test assertion |
| `kernel/tests.rs:345` | `fn audio_output_cannot_feed_control_input()` | Kernel test |

---

## 4. `voice_allocation` and `render:` Usage in Examples

### `voice_allocation`

| File:line | Key | Value shape |
|---|---|---|
| `examples/patches/drum-kit.yaml:4` | `voice_allocation` | `max_voices: 8`, `stealing: oldest_active` |
| `examples/patches/polyphonic-sampler-chords.yaml:3` | `voice_allocation` | `max_voices: 6`, `stealing: disabled` |
| `examples/patches/polyphonic-chords.yaml:3` | `voice_allocation` | `max_voices: 6`, `stealing: disabled` |
| `examples/patches/polyphonic-pad.yaml:5` | `voice_allocation` | `max_voices: 8`, `stealing: oldest_active` |
| `examples/patches/event-routing-simple-poly-synth.yaml:4` | `voice_allocation` | `max_voices: 4`, `stealing: oldest_active` |

All five use exactly two sub-keys: `max_voices` (integer) and `stealing` (`oldest_active` or `disabled`).

### `render:` (all example patches)

Every example patch has a `render:` block. All entries have three sub-keys:

| File:line | sample_rate_hz | block_size_frames | duration_frames |
|---|---|---|---|
| `control-mixer-modulation.yaml:4` | 48000 | 128 | 48000 |
| `delayed-feedback.yaml:4` | 48000 | 128 | 48000 |
| `drum-kit.yaml:7` | 48000 | 128 | 4096 |
| `echo-demo.yaml:4` | 48000 | 128 | 96000 |
| `envelope-ducking.yaml:3` | 48000 | 128 | 2048 |
| `envelope-filter-modulation.yaml:3` | 48000 | 128 | 2048 |
| `event-routing-drum-machine.yaml:4` | 48000 | 64 | 4800 |
| `event-routing-simple-poly-synth.yaml:7` | 48000 | 64 | 4800 |
| `minimal-event-osc-vca.yaml:4` | 48000 | 128 | 48000 |
| `minimal-sampler.yaml:3` | 22050 | 64 | 256 |
| `minimal-tune.yaml:4` | 48000 | 128 | 48000 |
| `module-drum-voice.yaml:3` | 48000 | 64 | 4800 |
| `module-echo.yaml:4` | 48000 | 128 | 96000 |
| `module-hidden-internals.yaml:3` | 48000 | 64 | 4800 |
| `module-impulse-layer.yaml:3` | 48000 | 128 | 2048 |
| `module-impulse-noise.yaml:3` | 48000 | 128 | 2048 |
| `module-impulse-tone.yaml:3` | 48000 | 128 | 2048 |
| `module-reverb.yaml:4` | 48000 | 128 | 144000 |
| `module-velocity-vca.yaml:3` | 48000 | 128 | 1024 |
| `polyphonic-chords.yaml:6` | 48000 | 128 | 192000 |
| `polyphonic-pad.yaml:8` | 48000 | 128 | 240000 |
| `polyphonic-sampler-chords.yaml:6` | 22050 | 128 | 88200 |
| `reverb-demo.yaml:4` | 48000 | 128 | 144000 |
| `script-drum-event-router.yaml:3` | 48000 | 128 | 128 |
| `script-state-counter.yaml:3` | 48000 | 128 | 256 |
| `script-velocity-accent.yaml:3` | 48000 | 128 | 128 |
| `script-velocity-map.yaml:3` | 48000 | 128 | 128 |
| `short-tune-with-delay.yaml:5` | 48000 | 128 | 48000 |
| `synthetic-808-kick.yaml:44` | 48000 | 128 | 48000 |
| `synthetic-hats.yaml:3` | 48000 | 128 | 2048 |
| `synthetic-snare.yaml:3` | 48000 | 128 | 2048 |
| `drums/drum-808-clap.yaml:36` | 48000 | 128 | 48000 |
| `drums/drum-808-conga.yaml:36` | 48000 | 128 | 48000 |
| `drums/drum-808-cowbell.yaml:41` | 48000 | 128 | 48000 |
| `drums/drum-808-snare.yaml:45` | 48000 | 128 | 48000 |
| `drums/drum-808-tom.yaml:37` | 48000 | 128 | 48000 |
| `drums/drum-909-clap.yaml:36` | 48000 | 128 | 48000 |
| `drums/drum-909-crash.yaml:19` | 48000 | 128 | 96000 |
| `drums/drum-909-hat-closed.yaml:21` | 48000 | 128 | 48000 |
| `drums/drum-909-hat-open.yaml:19` | 48000 | 128 | 48000 |
| `drums/drum-909-kick.yaml:43` | 48000 | 128 | 48000 |
| `drums/drum-909-ride.yaml:19` | 48000 | 128 | 96000 |
| `drums/drum-909-snare.yaml:36` | 48000 | 128 | 48000 |
| `drums/drum-909-tom.yaml:37` | 48000 | 128 | 48000 |

Uniform shape: `sample_rate_hz`, `block_size_frames`, `duration_frames`. No `render` block uses any other key.

---

## 5. Params / Asset-Binding Surface in Example YAMLs

### `parameters:` blocks (module-instance level)

Every occurrence is a per-module inline YAML map. Listed by file and line, showing the
module `id` and the parameter keys set:

| File:line | Module id | Parameters set |
|---|---|---|
| `examples/patches/script-velocity-accent.yaml:12` | `script` | `language`, `source` |
| `examples/patches/script-drum-event-router.yaml:12` | `script` | `language`, `source` |
| `examples/patches/synthetic-808-kick.yaml:7` | (module_definition `tuned_osc`) | `curve`, `steps` |
| `examples/patches/synthetic-808-kick.yaml:59` | `tune` | `pitch` |
| `examples/patches/synthetic-808-kick.yaml:123` | `tone_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/synthetic-808-kick.yaml:128` | `click_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/synthetic-808-kick.yaml:139` | `noise_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/synthetic-808-kick.yaml:144` | `envelope` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/synthetic-808-kick.yaml:150` | `mix_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/synthetic-808-kick.yaml:156` | `pitch_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/synthetic-808-kick.yaml:196` | `mapper` | `curve`, `steps` |
| `examples/patches/synthetic-hats.yaml:12` | `noise` | `seed` |
| `examples/patches/synthetic-hats.yaml:16` | `env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/drum-kit.yaml:31` | `kick.tune` | `pitch` |
| `examples/patches/drum-kit.yaml:70` | `kick.punch_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/drum-kit.yaml:74` | `kick.click_gate` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/drum-kit.yaml:113` | `snare.punch_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/drum-kit.yaml:117` | `snare.click_gate` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/drum-kit.yaml:150` | `kick.mapper` | `curve` |
| `examples/patches/drum-kit.yaml:155` | `snare.mapper` | `curve` |
| `examples/patches/drum-kit.yaml:160` | `hats.mapper` | `curve` |
| `examples/patches/envelope-filter-modulation.yaml:15` | `filter` | `algorithm`, `mode` |
| `examples/patches/envelope-filter-modulation.yaml:25` | `env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/envelope-filter-modulation.yaml:36` | `lfo` | *(empty map? checked at line 36)* |
| `examples/patches/script-state-counter.yaml:12` | `script` | `language`, `source` |
| `examples/patches/polyphonic-sampler-chords.yaml:30` | `sampler` | `asset` |
| `examples/patches/envelope-ducking.yaml:15` | `follower` | `mode` |
| `examples/patches/script-velocity-map.yaml:12` | `script` | `language`, `source` |
| `examples/patches/minimal-sampler.yaml:16` | `sampler` | `asset` |
| `examples/patches/event-routing-drum-machine.yaml:37,42,47` | `kick_route`, `snare_route`, `hat_route` | `selector`, `note` |
| `examples/patches/event-routing-simple-poly-synth.yaml:16` | `note_route` | `selector`, `note` |
| `examples/patches/module-impulse-layer.yaml:27,31` | `tone`, `noise` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/module-impulse-noise.yaml:25,29` | `tone`, `noise` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/synthetic-snare.yaml:14` | `noise` | `seed` |
| `examples/patches/drums/drum-808-snare.yaml:14` | `tune` | `pitch` |
| `examples/patches/drums/drum-808-snare.yaml:54` | `noise_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/drums/drum-808-snare.yaml:59` | `click_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/drums/drum-808-snare.yaml:65` | `tone_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/drums/drum-808-snare.yaml:69` | `mix_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/drums/drum-808-snare.yaml:73` | `noise_mix_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/drums/drum-808-snare.yaml:78` | `pitch_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/drums/drum-808-snare.yaml:84` | `mapper` | `curve` |
| `examples/patches/drums/drum-909-ride.yaml:12` | `sampler` | `asset` |
| `examples/patches/drums/drum-909-ride.yaml:32` | `level` | `gain` |
| `examples/patches/drums/drum-909-ride.yaml:36` | `level2` | `gain` |
| `examples/patches/drums/drum-909-tom.yaml:12` | `tune` | `pitch` |
| `examples/patches/drums/drum-909-tom.yaml:48` | `tone_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/drums/drum-909-tom.yaml:52` | `click_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/drums/drum-909-tom.yaml:56` | `noise_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/drums/drum-909-tom.yaml:60` | `pitch_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/drums/drum-909-tom.yaml:66` | `mapper` | `curve` |
| `examples/patches/drums/drum-909-kick.yaml:12` | `tune` | `pitch` |
| `examples/patches/drums/drum-909-kick.yaml:54` | `tone_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/drums/drum-909-kick.yaml:58` | `click_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/drums/drum-909-kick.yaml:62` | `noise_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/drums/drum-909-kick.yaml:66` | `punch_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/drums/drum-909-kick.yaml:72` | `mix_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/drums/drum-909-kick.yaml:78` | `pitch_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/drums/drum-909-kick.yaml:83` | `noise_mix_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/drums/drum-909-kick.yaml:90` | `mapper` | `curve` |
| `examples/patches/drums/drum-808-conga.yaml:11` | `tune` | `pitch` |
| `examples/patches/drums/drum-808-conga.yaml:47` | `tone_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/drums/drum-808-conga.yaml:51` | `click_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/drums/drum-808-conga.yaml:55` | `noise_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/drums/drum-808-conga.yaml:59` | `pitch_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/drums/drum-808-conga.yaml:65` | `mapper` | `curve` |
| `examples/patches/drums/drum-808-tom.yaml:12` | `tune` | `pitch` |
| `examples/patches/drums/drum-808-tom.yaml:48` | `tone_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/drums/drum-808-tom.yaml:52` | `click_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/drums/drum-808-tom.yaml:56` | `noise_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/drums/drum-808-tom.yaml:60` | `pitch_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/drums/drum-808-tom.yaml:66` | `mapper` | `curve` |
| `examples/patches/drums/drum-909-hat-open.yaml:12` | `sampler` | `asset` |
| `examples/patches/drums/drum-909-hat-open.yaml:32` | `level` | `gain` |
| `examples/patches/drums/drum-909-hat-open.yaml:36` | `level2` | `gain` |
| `examples/patches/drums/drum-909-snare.yaml:11` | `tune` | `pitch` |
| `examples/patches/drums/drum-909-snare.yaml:47` | `tone_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/drums/drum-909-snare.yaml:51` | `click_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/drums/drum-909-snare.yaml:57` | `noise_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/drums/drum-909-snare.yaml:61` | `pitch_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/drums/drum-909-snare.yaml:65` | `mix_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/drums/drum-909-snare.yaml:70` | `noise_mix_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/drums/drum-909-snare.yaml:76` | `mapper` | `curve` |
| `examples/patches/drums/drum-909-hat-closed.yaml:14` | `sampler` | `asset` |
| `examples/patches/drums/drum-909-hat-closed.yaml:34` | `level` | `gain` |
| `examples/patches/drums/drum-909-hat-closed.yaml:38` | `level2` | `gain` |
| `examples/patches/drums/drum-909-clap.yaml:11` | `tune` | `pitch` |
| `examples/patches/drums/drum-909-clap.yaml:45` | `tone_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/drums/drum-909-clap.yaml:49` | `click_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/drums/drum-909-clap.yaml:54` | `noise_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/drums/drum-909-clap.yaml:60` | `pitch_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/drums/drum-909-clap.yaml:64` | `mix_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/drums/drum-909-clap.yaml:69` | `noise_mix_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/drums/drum-909-clap.yaml:75` | `mapper` | `curve` |
| `examples/patches/drums/drum-808-cowbell.yaml:16` | `tune` | `pitch` |
| `examples/patches/drums/drum-808-cowbell.yaml:50` | `tone_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/drums/drum-808-cowbell.yaml:54` | `click_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/drums/drum-808-cowbell.yaml:59` | `noise_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/drums/drum-808-cowbell.yaml:66` | `pitch_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/drums/drum-808-cowbell.yaml:70` | `mix_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/drums/drum-808-cowbell.yaml:75` | `noise_mix_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/drums/drum-808-cowbell.yaml:82` | `mapper` | `curve` |
| `examples/patches/drums/drum-808-clap.yaml:11` | `tune` | `pitch` |
| `examples/patches/drums/drum-808-clap.yaml:45` | `tone_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/drums/drum-808-clap.yaml:49` | `click_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/drums/drum-808-clap.yaml:54` | `noise_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/drums/drum-808-clap.yaml:60` | `pitch_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/drums/drum-808-clap.yaml:64` | `mix_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/drums/drum-808-clap.yaml:69` | `noise_mix_env` | `attack`, `decay`, `sustain`, `release` |
| `examples/patches/drums/drum-808-clap.yaml:75` | `mapper` | `curve` |
| `examples/patches/drums/drum-909-crash.yaml:12` | `sampler` | `asset` |
| `examples/patches/drums/drum-909-crash.yaml:32` | `trim` | `gain` |
| `examples/patches/drums/drum-909-crash.yaml:36` | `trim` (duplicate) | *(needs verification)* |

### `preset_surface:` blocks (preset parameter surface)

Found in drum patches that declare `instrument:` and `preset_surface:`:

| File | `preset_surface` parameters | Mechanism |
|---|---|---|
| `examples/patches/drums/drum-909-crash.yaml:11-18` | `crash.level` (number, default 0.7, min 0, max 2, maps_to: `trim.gain`) | `preset_surface` |
| `examples/patches/drums/drum-909-hat-closed.yaml:14-20` | `hat.level` (number, default 0.8, min 0, max 2, maps_to: `level.gain`) | `preset_surface` |
| `examples/patches/drums/drum-909-hat-open.yaml:12-18` | `hat.level` (number, default 0.8, min 0, max 2, maps_to: `level.gain`) | `preset_surface` |
| `examples/patches/drums/drum-909-ride.yaml:12-18` | `ride.level` (number, default 0.7, min 0, max 2, maps_to: `level.gain`) | `preset_surface` |
| (others with `instrument:` id likely have similar) | | |

### `asset_bindings:` blocks

No example YAML files use `asset_bindings:`. The `asset_bindings` mechanism exists
in the Rust source (`module_package.rs:46`, `patch_module.rs:22`) and is exercised
only in Rust integration tests (`engine_patch_behaviour_tests.rs:259,277,746`),
not in example YAMLs.

### `${name}` binding references in YAML

No example YAML files use `${...}` interpolation. The only `${}` references in
the repo are:
- `kernel.rs:682` — error message string (Rust source)
- `tests/declarative_parameters_remaining_red_tests.rs:618` — test expectation (`${tune_hz} * 2`)

---

## Gaps / Ambiguities

1. **`lfo` has no declared parameters.** The builtin at `builtins.rs:429` exposes
   only a `rate` control input and a `value` control output. There is no parameter
   metadata for waveform selection, min/max rate, etc. This may be intentional
   (future work) or an oversight.

2. **`envelope-filter-modulation.yaml` line 36** — the `lfo` module block has
   `parameters:` followed by what appears to be an empty map. This needs
   verification (could be a YAML formatting quirk).

3. **`control_mixer` (builtins.rs:386) and `audio_mixer` (builtins.rs:380)**
   have no static port count — they use a mixing input (`Port::mixing_input(...)`)
   that accepts multiple connections. How channel counts are inferred is not
   visible in the metadata.

4. **`script` has zero standard ports.** Its YAML-declared ports are user-defined.
   The `BuiltInModuleDefinition` registers no inputs/outputs, so the port schema
   is entirely dynamic.

5. **`asset_bindings:` is defined in Rust structs but never used in any example
   YAML.** The only test coverage is in `engine_patch_behaviour_tests.rs` and
   mutation-test outputs.

6. **`preset_surface` with `maps_to`** is used in ~10 drum patches but the
   exact line ranges for each were not exhaustively checked beyond
   `drum-909-crash.yaml` as a representative. The mechanism is `preset_surface`
   → `parameters[]` → each item has `name`, `type`, `default`, `min`, `max`,
   `maps_to`.

7. **`render:` shape is uniform across all example patches** — exactly three
   keys (`sample_rate_hz`, `block_size_frames`, `duration_frames`). No example
   uses additional render keys (e.g. `num_channels`). The `audio_output` builtin
   is the only node type that determines channel count for the render output;
   today it is hardcoded to stereo (left/right).
