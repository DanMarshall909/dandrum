# INTEGRATION — design controls vs. existing Dandrum capabilities

Repo inspected: `DanMarshall909/dandrum@main` (`d02ee1b`), 2026-10-08. Files read: `docs/advanced-sampler.md`, `docs/nomenclature.md`, `docs/module-library.md`, `examples/patches/advanced-drum-kit.yaml`, `examples/patches/drum-kit.yaml`, `src/juce-plugin/{SamplerWebUi.h, SharedInstrumentUi.h, InstrumentHostWebBridge.h, PluginEditor.h, PluginProcessor.cpp (search)}`, `src/rust-engine/src/builtins/module_types.rs`, `openspec/specs/control-primitives/spec.md`, `openspec/changes/{add-renderer-independent-plugin-ui, add-yaml-editor, add-workstation-sampling}/proposal.md`, `AGENTS.md`.

"Exists" below means found in those files. Anything not found is marked **not found** — Codex must verify before relying on it.

## 1. Existing surfaces the editor can use today

| Capability | Where | Notes |
|---|---|---|
| WebView editor with native functions `setParameter(id, value)`, `getParameters()`, `noteOn(note, velocity)`, `noteOff(note)` and event `parameterValuesChanged` | `InstrumentHostWebBridge.h`, `SharedInstrumentUi.h` | Current sampler UI uses these. `noteOn` velocity is sent as 0–1 (0.9 in the demo). Knob values are clamped 0–1 before sending. |
| Public parameter surface with stable host slots | `PluginProcessor.cpp` (`publicSlotParameterId`, `getActivePublicParameterIds`, `getPublicParameterDisplayName`) | Dropped IDs on reload produce "Instrument no longer defines: …". |
| Sampler public parameters | `advanced-drum-kit.yaml` `preset_surface` | `drums.pitch_ratio` 0.125–8 ×, `drums.start_offset` 0–1, `drums.level` 0–4, `drums.pan` −1–1, `drums.variation` 0–1; per pad `drums.{kick,snare,closed_hat,open_hat}.{pitch_ratio,start_offset,level,pan}` + `variation` on snare and open hat. |
| Sample map | `advanced-drum-kit.yaml` `assets.sample_maps` | Zones: `key_range`, `velocity_range`, `round_robin_group`, `choke_group`, `control_group` (1–8), `gain_db`; map `selection_mode: round_robin`, `selection_seed`. |
| Player policy | `sample_map_player` static | `max_voices: 8`, `voice_steal: oldest`, `choke_mode: cut`, `channels: 2`. |
| Primitives | `module_types.rs` | `adsr`, `lfo`, `filter`, `gain`, `audio_mixer`, `control_mixer`, `sampler`, `sample_player`, `sample_zone_selector`, `sample_slicer`, `sample_map_player` (8 control groups × 5 controls), `dynamics-processor`, `saturator`, `convolution`, `echo`, `reverb`, `envelope_follower`, `curve_mapper`, `slew`, `decay`, `poly`, `noise`, `oscillator`, `note_to_control`, `event_filter`, `spectral_processor`, `frequency_splitter`. |
| Root ports → named buses | `drum-kit.yaml` (`master`, `kick`, `snare`, `hat`), `nomenclature.md` | Multi-out exists at the patch level. |
| File watch + reload | `InstrumentFileWatcher.{h,cpp}`, `add-yaml-editor` | External editing of the patch file with safe replacement; failed reload keeps the previous DSP. |
| Planned shared UI contract | `add-renderer-independent-plugin-ui` (tasks unchecked) | Typed snapshots, commands with generation and gestures, 30 Hz bounded telemetry, native + WebView renderers, capability-gated KeyMap / LayerStack / OutputBusses. **Explicitly says drag-to-edit zones and add/remove/reroute are reference interactions, not authorisation for graph authoring in the plugin.** |

## 2. Control mapping

| Design control | Maps to | Status |
|---|---|---|
| Pads (PadCell, PlayPads) trigger | `noteOn(note, vel)` / `noteOff(note)` | **Exists.** Convert design velocity 1–127 to the bridge's 0–1. |
| Keyboard (PlayKeys) | `noteOn` / `noteOff` | **Exists.** |
| PC keys | Same note functions, driven by editor key events | Bridge exists; keyboard focus inside a plugin window is host-dependent (Q-5). |
| Mapped / unmapped keys, pad names, Fit range | Zone `key_range` from the prepared sample map | Needs read-only prepared metadata in the snapshot (planned in renderer-independent change; **not found** as a current bridge function). |
| Pad activity, voices "6 / 32", meter, sounding keys | Telemetry | **Planned** (`plugin-visual-telemetry`), not available. Mockup uses 32; the kit uses `max_voices: 8`. |
| Region play modes, start/end, fades, loops | `sample_player` / region definitions | Region `start_frame`/`end_frame` exist. Play modes, fades, loops: **not found** in files read — verify in `advanced-sampling-options` spec. Prepared, not live. |
| "Start offset" knob | `drums.start_offset` / `drums.<pad>.start_offset` | **Exists** (live, next hit). |
| Pitch (Tune/Fine) | `drums.pitch_ratio` (×) | **Exists.** Display as ratio with a semitone readout; host range is the truth. |
| Gain / Level | `drums.level` 0–4 | **Exists.** Design shows dB; convert for display only. |
| Pan | `drums.pan` −1–1 | **Exists.** Display L/C/R. |
| Variation | `drums.variation`, pad `variation` | **Exists.** |
| Velocity layers | Zone `velocity_range` | **Exists** as prepared data. |
| Round robin / alternates | `round_robin_group`, `selection_mode`, `selection_seed` | **Exists** (round robin). Stack, Random, Weighted modes: **not found**. |
| Choke groups | `choke_group`, `choke_mode: cut` | **Exists.** |
| Voice limit / steal | `max_voices`, `voice_steal` | **Exists** (prepared). Same note, glide, mono/legato: **not found**. |
| Slices | `sample_slicer` primitive | Primitive exists; transient analysis job, Export MIDI: **not found**. |
| Filter section, Filter response drag, Cutoff/Reso | `filter` primitive | Primitive exists; **not in the sampler kit's graph** (`sample_map_player` only). Needs a patch that wires a filter and exposes control ports. |
| Amp / Filter envelopes | `adsr` | Primitive exists; not in the kit graph. Default amp envelope semantics: see ticket #9 in reconciliation notes. |
| Modulators (LFO, envelopes, random), route table, Add route, drag-to-assign | `lfo`, `adsr`, `curve_mapper`, `slew`, cables to control ports | Primitives exist; **in-plugin route editing does not exist**. Today modulation is host/DAW modulation of public parameters (Bitwig example in `advanced-sampler.md`). |
| Macros (8 knobs, destinations, Learn MIDI) | — | **Not found.** Closest existing: the five shared `drums.*` controls. |
| Routing: output bus per layer, sends, FX busses | Root ports / named buses; `reverb`, `echo`, `convolution` | Buses exist per patch; per-layer sends and FX busses inside `sample_map_player`: **not found**. |
| Effects LayerStack (inserts) | `dynamics-processor`, `saturator`, `filter`, … | Primitives exist; editable chains in the plugin: **not found**. |
| Patch browser, load | Patch files + reload transaction | Reload exists. A patch catalogue with categories/origin/favourites: **not found**; `instrument-presets` spec exists (patch vs. preset distinction must be kept). |
| Import / export definition | The patch YAML file itself | Export = copy the loaded file; Import = load another file through reload. No in-plugin YAML editing (add-yaml-editor scopes it out). |
| Reload Patch, pending banner, load errors | Reload transaction, dropped-parameter warning | **Exists** (behaviour), UI surface new. |
| Diagnostics "Why did this play?" | — | **Not found** (see `docs/routing-diagnostics.md`, not read in full). |
| Freeze | — | **Not found.** |
| Overview / inheritance, scope bar, "Mixed" | Shared × per-pad parameter combination (multiply / add) | Combination rules exist; inheritance UI is new. |
| Themes (surface/accent/secondary) | DS tokens | Token pipeline planned (one token source → CSS + C++). Extra themes are new. |
| Multi-file drop, Browse samples… | — | Asset import at runtime: **not found**; structure is prepared. |

## 3. Architectural mismatches (not fixed here)

1. **Live structural editing vs. prepared structure.** The prototype edits zones, layers, slices, routes, modules and the tree in place. The repo treats them as preparation-time data changed by editing the patch and reloading; the renderer-independent change says the design's drag-to-edit is reference only. The prototype's "Reload pending" state (26) is the bridge, but whether the plugin may author structure at all is unresolved (Q-1).
2. **In-plugin modulation.** The Modulation page, modulators list, rings on knobs and drag-to-assign assume an editable modulation matrix. The repo has primitives and host modulation, not a plugin-side route editor.
3. **Macros.** Eight macros with destinations do not exist; the patch exposes five shared controls plus per-pad controls.
4. **Per-layer chains, sends, FX busses.** `sample_map_player` is a single module with control groups; the prototype's per-layer module chains and send matrix have no counterpart.
5. **Velocity split.** The prototype now uses soft 1–95 / hard 96–127 everywhere. The maintained kit uses **1–63 / 64–127**. Production must read the split from the loaded map; do not hard-code either.
6. **Voice counts.** Prototype shows 32 voices; kit is 8.
7. **Parameter units.** Prototype shows semitones / dB / %; repo parameters are ratio 0.125–8, level 0–4, offset 0–1. Display formatting only.
8. **Renderer.** The prototype is HTML (Design Components + a React-based bundle). Production targets typed C++ state with native JUCE and WebView renderers. The prototype's logic class (state, layout maths, mock data) is not reusable code.
9. **Copy.** The DS content rules forbid "YAML", "node", "graph" in UI text; the prototype's patch menu says "YAML definition" because it was requested. Decide wording (Q-4).
10. **Mock data.** Patches listed in the browser (Amen Break 172, Brush Kit, Felt Upright, Glass Mallets, Sub Kick Stack, Lush) are invented; only three acceptance patches have content, and those are mock content too.
11. **Font minimum.** Rail labels, pad badges and key letters use 10 px; the DS minimum is 11 px.
12. **Design-system additions** (8 icons, `--dd-cap`/`--dd-pad` variables) exist only in the prototype bundle, not in `docs/design-system/`.
